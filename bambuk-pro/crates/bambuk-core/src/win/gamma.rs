use std::ffi::c_void;
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

pub fn get() -> Result<[u16; 768], String> {
    let mut r = [0u16; 768];
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() { return Err("GetDC не вернул контекст экрана".into()); }
        let ok = GetDeviceGammaRamp(dc, r.as_mut_ptr() as *mut c_void);
        ReleaseDC(None, dc);
        if !ok.as_bool() { return Err("GetDeviceGammaRamp не удалось".into()); }
    }
    Ok(r)
}
pub fn set(r: &[u16; 768]) -> Result<(), String> {
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() { return Err("GetDC не вернул контекст экрана".into()); }
        let ok = SetDeviceGammaRamp(dc, r.as_ptr() as *const c_void);
        ReleaseDC(None, dc);
        if !ok.as_bool() { return Err("Драйвер отклонил рампу гаммы (значение слишком далеко от нейтрального)".into()); }
    }
    Ok(())
}
pub fn set_verified(r: &[u16; 768]) -> Result<(), String> {
    set(r)?;
    let back = get()?;
    let same = r.iter().zip(back.iter()).all(|(a, b)| (*a as i32 - *b as i32).abs() <= 256);
    if same { Ok(()) } else { Err("Драйвер принял рампу, но не сохранил её".into()) }
}
