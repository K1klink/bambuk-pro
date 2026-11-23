//! Digital Vibrance через NVAPI (nvapi64.dll ставится драйвером NVIDIA). Загрузка динамическая:
//! если DLL/функций нет — честно возвращаем ошибку, ничего не имитируем.
use std::ffi::c_void;
use std::sync::OnceLock;
use windows::core::{s, PCSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

type QueryInterface = unsafe extern "C" fn(u32) -> *const c_void;
type FnInit = unsafe extern "C" fn() -> i32;
type FnEnum = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type FnGetEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcEx) -> i32;
type FnSetEx = unsafe extern "C" fn(*mut c_void, u32, i32) -> i32;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct DvcEx { version: u32, current: i32, min: i32, max: i32, default: i32 }
const DVC_EX_VERSION: u32 = (std::mem::size_of::<DvcEx>() as u32) | (1 << 16);

struct Api { enum_display: FnEnum, get_ex: FnGetEx, set_ex: FnSetEx }
// Указатели на функции из DLL, живущей весь процесс.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

static API: OnceLock<Result<Api, String>> = OnceLock::new();

fn api() -> Result<&'static Api, String> {
    API.get_or_init(|| unsafe {
        let lib = LoadLibraryA(s!("nvapi64.dll")).map_err(|_| "nvapi64.dll не найдена (нужна видеокарта NVIDIA и её драйвер)".to_string())?;
        let q = GetProcAddress(lib, PCSTR(b"nvapi_QueryInterface\0".as_ptr())).ok_or("nvapi_QueryInterface не найден")?;
        let q: QueryInterface = std::mem::transmute(q);
        let get = |id: u32, name: &str| -> Result<*const c_void, String> {
            let p = q(id); if p.is_null() { Err(format!("NVAPI: функция {name} недоступна в этом драйвере")) } else { Ok(p) }
        };
        let init: FnInit = std::mem::transmute(get(0x0150E828, "Initialize")?);
        if init() != 0 { return Err("NvAPI_Initialize завершилась ошибкой".into()); }
        Ok(Api {
            enum_display: std::mem::transmute(get(0x9ABDD40D, "EnumNvidiaDisplayHandle")?),
            get_ex: std::mem::transmute(get(0x0E45002D, "GetDVCInfoEx")?),
            set_ex: std::mem::transmute(get(0x4A82C2B1, "SetDVCLevelEx")?),
        })
    }).as_ref().map_err(|e| e.clone())
}

fn display() -> Result<*mut c_void, String> {
    let a = api()?;
    let mut h: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (a.enum_display)(0, &mut h) };
    if rc != 0 || h.is_null() { return Err("NVAPI не нашёл дисплей NVIDIA".into()); }
    Ok(h)
}
fn info(h: *mut c_void) -> Result<DvcEx, String> {
    let a = api()?;
    let mut i = DvcEx { version: DVC_EX_VERSION, ..Default::default() };
    let rc = unsafe { (a.get_ex)(h, 0, &mut i) };
    if rc != 0 { return Err(format!("NvAPI_GetDVCInfoEx вернул {rc}")); }
    Ok(i)
}
/// Текущий уровень драйвера (сырое значение).
pub fn current_level() -> Result<i32, String> { Ok(info(display()?)?.current) }

/// vibrance −100..100 → уровень драйвера вокруг значения по умолчанию.
pub fn level_for(vibrance: f32, i: (i32, i32, i32)) -> i32 {
    let (min, max, def) = i;
    let v = (vibrance / 100.0).clamp(-1.0, 1.0);
    let span = if v >= 0.0 { (max - def) as f32 } else { (def - min) as f32 };
    (def as f32 + v * span).round() as i32
}
pub fn set_vibrance(vibrance: f32) -> Result<i32, String> {
    let a = api()?; let h = display()?; let i = info(h)?;
    let lvl = level_for(vibrance, (i.min, i.max, i.default));
    let rc = unsafe { (a.set_ex)(h, 0, lvl) };
    if rc != 0 { return Err(format!("NvAPI_SetDVCLevelEx вернул {rc}")); }
    let back = info(h)?.current;
    if back == lvl { Ok(lvl) } else { Err(format!("Драйвер не принял уровень {lvl} (сейчас {back})")) }
}
pub fn restore_level(level: i32) -> Result<(), String> {
    let a = api()?; let h = display()?;
    let rc = unsafe { (a.set_ex)(h, 0, level) };
    if rc == 0 { Ok(()) } else { Err(format!("NvAPI_SetDVCLevelEx вернул {rc}")) }
}
