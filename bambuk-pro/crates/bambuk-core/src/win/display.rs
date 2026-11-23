use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;

pub use crate::modes::Mode;

fn blank() -> DEVMODEW { let mut dm = DEVMODEW::default(); dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16; dm }

pub fn current() -> Result<Mode, String> {
    let mut dm = blank();
    unsafe {
        if !EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            return Err("Не удалось прочитать текущий видеорежим".into());
        }
    }
    Ok(Mode { w: dm.dmPelsWidth, h: dm.dmPelsHeight, hz: dm.dmDisplayFrequency })
}
pub fn list() -> Vec<Mode> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let mut dm = blank();
        let ok = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_DISPLAY_SETTINGS_MODE(i), &mut dm).as_bool() };
        if !ok { break; }
        if dm.dmBitsPerPel >= 32 {
            let m = Mode { w: dm.dmPelsWidth, h: dm.dmPelsHeight, hz: dm.dmDisplayFrequency };
            if !out.contains(&m) { out.push(m); }
        }
        i += 1;
        if i > 4096 { break; }
    }
    out
}
/// Временный режим: ОС вернёт исходный при завершении процесса (CDS_FULLSCREEN), плюс явный откат в Engine.
pub fn set(m: Mode) -> Result<(), String> {
    let mut dm = blank();
    dm.dmPelsWidth = m.w; dm.dmPelsHeight = m.h; dm.dmDisplayFrequency = m.hz; dm.dmBitsPerPel = 32;
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL;
    let r = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_FULLSCREEN, None) };
    if r == DISP_CHANGE_SUCCESSFUL { Ok(()) } else { Err(format!("ChangeDisplaySettingsEx вернул код {}", r.0)) }
}
pub fn reset() -> Result<(), String> {
    let r = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None) };
    if r == DISP_CHANGE_SUCCESSFUL { Ok(()) } else { Err(format!("Откат видеорежима: код {}", r.0)) }
}
