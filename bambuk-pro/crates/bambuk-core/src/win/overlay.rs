//! Overlay-движок: Windows Graphics Capture окна игры → шейдер (CAS-резкость + vibrance) →
//! прозрачное click-through окно поверх игры. Файлы и процесс игры не затрагиваются.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use windows::core::{s, w, Interface, IInspectable, PCSTR};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShaderParams { pub sharpness: f32, pub vibrance: f32 } // 0..1, -1..1

pub struct Overlay {
    stop: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    params: Arc<Mutex<ShaderParams>>,
    err: Arc<Mutex<Option<String>>>,
    handle: Option<JoinHandle<()>>,
    pub game_hwnd: isize,
}

const HLSL: &str = r#"
cbuffer P : register(b0) { float2 texel; float sharp; float vib; };
Texture2D tex : register(t0);
SamplerState smp : register(s0);
struct VO { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };
VO vs(uint id : SV_VertexID) {
    VO o; float2 uv = float2((id << 1) & 2, id & 2);
    o.pos = float4(uv * float2(2, -2) + float2(-1, 1), 0, 1); o.uv = uv; return o;
}
float3 S(float2 uv) { return tex.SampleLevel(smp, uv, 0).rgb; }
float4 ps(VO i) : SV_TARGET {
    float3 c = S(i.uv);
    float3 a = S(i.uv + float2(0, -texel.y)), b = S(i.uv + float2(-texel.x, 0));
    float3 d = S(i.uv + float2(texel.x, 0)),  e = S(i.uv + float2(0, texel.y));
    float3 mn = min(min(min(a, b), min(d, e)), c);
    float3 mx = max(max(max(a, b), max(d, e)), c);
    float3 amp = sqrt(saturate(min(mn, 1.0 - mx) / max(mx, 1e-4)));
    float3 w = amp * (-1.0 / lerp(8.0, 5.0, saturate(sharp)));
    float3 col = saturate((a * w + b * w + d * w + e * w + c) / (1.0 + 4.0 * w));
    col = lerp(c, col, step(0.001, sharp));
    float lum = dot(col, float3(0.2126, 0.7152, 0.0722));
    float sat = max(col.r, max(col.g, col.b)) - min(col.r, min(col.g, col.b));
    col = saturate(lerp(lum.xxx, col, 1.0 + vib * (1.0 - sat)));
    return float4(col, 1);
}
"#;

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT { DefWindowProcW(h, m, wp, lp) }

impl Overlay {
    pub fn start(game_hwnd: isize, params: ShaderParams) -> Overlay {
        let stop = Arc::new(AtomicBool::new(false));
        let frames = Arc::new(AtomicU64::new(0));
        let p = Arc::new(Mutex::new(params));
        let err = Arc::new(Mutex::new(None));
        let (s2, f2, p2, e2) = (stop.clone(), frames.clone(), p.clone(), err.clone());
        let handle = std::thread::Builder::new().name("bambuk-overlay".into()).spawn(move || {
            if let Err(e) = unsafe { run(HWND(game_hwnd as *mut std::ffi::c_void), &s2, &f2, &p2) } {
                *e2.lock().unwrap() = Some(format!("{e}"));
            }
            s2.store(true, Ordering::SeqCst);
        }).ok();
        Overlay { stop, frames, params: p, err, handle, game_hwnd }
    }
    pub fn set_params(&self, p: ShaderParams) { *self.params.lock().unwrap() = p; }
    pub fn params(&self) -> ShaderParams { *self.params.lock().unwrap() }
    pub fn frames(&self) -> u64 { self.frames.load(Ordering::Relaxed) }
    pub fn error(&self) -> Option<String> { self.err.lock().unwrap().clone() }
    pub fn alive(&self) -> bool { !self.stop.load(Ordering::SeqCst) }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() { let _ = h.join(); }
    }
}
impl Drop for Overlay { fn drop(&mut self) { self.stop(); } }

unsafe fn compile(src: &str, entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code: Option<ID3DBlob> = None;
    let mut errs: Option<ID3DBlob> = None;
    let r = D3DCompile(src.as_ptr() as *const _, src.len(), None, None, None, entry, target, 0, 0, &mut code, Some(&mut errs));
    if let Err(e) = r {
        let msg = errs.map(|b| String::from_utf8_lossy(std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize())).to_string()).unwrap_or_default();
        return Err(windows::core::Error::new(e.code(), msg));
    }
    code.ok_or_else(|| windows::core::Error::from(E_FAIL))
}

unsafe fn run(game: HWND, stop: &AtomicBool, frames: &AtomicU64, params: &Mutex<ShaderParams>) -> windows::core::Result<()> {
    let _ = RoInitialize(RO_INIT_MULTITHREADED);

    let mut device: Option<ID3D11Device> = None;
    let mut ctx: Option<ID3D11DeviceContext> = None;
    D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut ctx))?;
    let (device, ctx) = (device.ok_or(E_FAIL)?, ctx.ok_or(E_FAIL)?);
    let dxgi: IDXGIDevice = device.cast()?;
    let winrt_dev: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;

    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let item: GraphicsCaptureItem = interop.CreateForWindow(game)?;
    let mut size = item.Size()?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&winrt_dev, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
    let session = pool.CreateCaptureSession(&item)?;
    let _ = session.SetIsCursorCaptureEnabled(false);
    let _ = session.SetIsBorderRequired(false);
    let evt = CreateEventW(None, false, false, None)?;
    let raw = evt.0 as isize;
    pool.FrameArrived(&TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
        let _ = SetEvent(HANDLE(raw as _));
        Ok(())
    }))?;

    // окно поверх игры
    let mut rc = RECT::default();
    GetWindowRect(game, &mut rc)?;
    let (ww, wh) = (((rc.right - rc.left).max(16)) as u32, ((rc.bottom - rc.top).max(16)) as u32);
    let hinst = GetModuleHandleW(None)?;
    let class = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: w!("BambukProOverlay"), ..Default::default() };
    RegisterClassW(&class);
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED,
        w!("BambukProOverlay"), w!("Bambuk Pro Overlay"), WS_POPUP,
        rc.left, rc.top, ww as i32, wh as i32, None, None, hinst, None)?;
    SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;

    let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: ww, Height: wh, Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, BufferCount: 2,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD, ..Default::default()
    };
    let swap = factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None)?;
    let back: ID3D11Texture2D = swap.GetBuffer(0)?;
    let mut rtv: Option<ID3D11RenderTargetView> = None;
    device.CreateRenderTargetView(&back, None, Some(&mut rtv))?;
    let rtv = rtv.ok_or(E_FAIL)?;

    let vs_blob = compile(HLSL, s!("vs"), s!("vs_5_0"))?;
    let ps_blob = compile(HLSL, s!("ps"), s!("ps_5_0"))?;
    let blob_bytes = |b: &ID3DBlob| std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize());
    let mut vs: Option<ID3D11VertexShader> = None;
    let mut ps: Option<ID3D11PixelShader> = None;
    device.CreateVertexShader(blob_bytes(&vs_blob), None, Some(&mut vs))?;
    device.CreatePixelShader(blob_bytes(&ps_blob), None, Some(&mut ps))?;
    let (vs, ps) = (vs.ok_or(E_FAIL)?, ps.ok_or(E_FAIL)?);

    let sd = D3D11_SAMPLER_DESC {
        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR, AddressU: D3D11_TEXTURE_ADDRESS_CLAMP, AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP, MaxLOD: f32::MAX, ComparisonFunc: D3D11_COMPARISON_NEVER, ..Default::default()
    };
    let mut sampler: Option<ID3D11SamplerState> = None;
    device.CreateSamplerState(&sd, Some(&mut sampler))?;
    let bd = D3D11_BUFFER_DESC { ByteWidth: 16, Usage: D3D11_USAGE_DEFAULT, BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32, ..Default::default() };
    let mut cb: Option<ID3D11Buffer> = None;
    device.CreateBuffer(&bd, None, Some(&mut cb))?;
    let cb = cb.ok_or(E_FAIL)?;

    session.StartCapture()?;
    let mut visible = false;

    while !stop.load(Ordering::SeqCst) {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() { let _ = TranslateMessage(&msg); DispatchMessageW(&msg); }
        if !IsWindow(game).as_bool() { break; }
        let fg = GetForegroundWindow() == game;
        if fg != visible { let _ = ShowWindow(hwnd, if fg { SW_SHOWNOACTIVATE } else { SW_HIDE }); visible = fg; }
        WaitForSingleObject(evt, 50);
        while let Ok(frame) = pool.TryGetNextFrame() {
            let cs = frame.ContentSize()?;
            if cs.Width != size.Width || cs.Height != size.Height {
                size = cs;
                pool.Recreate(&winrt_dev, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
                continue;
            }
            if !visible { continue; }
            let tex: ID3D11Texture2D = frame.Surface()?.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface()?;
            let mut srv: Option<ID3D11ShaderResourceView> = None;
            device.CreateShaderResourceView(&tex, None, Some(&mut srv))?;
            let p = *params.lock().unwrap();
            let data = [1.0 / size.Width.max(1) as f32, 1.0 / size.Height.max(1) as f32, p.sharpness, p.vibrance];
            ctx.UpdateSubresource(&cb, 0, None, data.as_ptr() as *const _, 0, 0);
            let vp = D3D11_VIEWPORT { Width: ww as f32, Height: wh as f32, MaxDepth: 1.0, ..Default::default() };
            ctx.RSSetViewports(Some(&[vp]));
            ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.VSSetShader(&vs, None);
            ctx.PSSetShader(&ps, None);
            ctx.PSSetConstantBuffers(0, Some(&[Some(cb.clone())]));
            ctx.PSSetShaderResources(0, Some(&[srv]));
            ctx.PSSetSamplers(0, Some(&[sampler.clone()]));
            ctx.Draw(3, 0);
            let _ = swap.Present(0, DXGI_PRESENT(0));
            frames.fetch_add(1, Ordering::Relaxed);
        }
    }
    let _ = session.Close();
    let _ = pool.Close();
    let _ = DestroyWindow(hwnd);
    let _ = CloseHandle(HANDLE(raw as _));
    Ok(())
}
