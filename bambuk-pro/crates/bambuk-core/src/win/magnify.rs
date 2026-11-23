use crate::colormath::{flatten, matrices_close, unflatten, M5};
use windows::Win32::UI::Magnification::*;

pub fn init() -> Result<(), String> {
    unsafe { MagInitialize().ok().map_err(|e| format!("MagInitialize: {e}")) }
}
pub fn shutdown() { unsafe { let _ = MagUninitialize(); } }

pub fn set(m: &M5) -> Result<(), String> {
    let eff = MAGCOLOREFFECT { transform: flatten(m) };
    unsafe { MagSetFullscreenColorEffect(&eff).ok().map_err(|e| format!("MagSetFullscreenColorEffect: {e}")) }
}
pub fn get() -> Result<M5, String> {
    let mut eff = MAGCOLOREFFECT { transform: [0.0; 25] };
    unsafe { MagGetFullscreenColorEffect(&mut eff).ok().map_err(|e| format!("MagGetFullscreenColorEffect: {e}"))?; }
    Ok(unflatten(&eff.transform))
}
/// Применить и проверить, что ОС действительно хранит эту матрицу.
pub fn set_verified(m: &M5) -> Result<(), String> {
    set(m)?;
    let back = get()?;
    if matrices_close(m, &back, 1e-3) { Ok(()) } else { Err("ОС не подтвердила применение цветовой матрицы".into()) }
}
