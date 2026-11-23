//! Windows-специфичные подсистемы. Никаких инъекций в процесс игры:
//! только публичные API ОС (Magnification, GDI, NVAPI, Windows Graphics Capture).
pub mod display;
pub mod gamma;
pub mod magnify;
pub mod nvapi;
pub mod overlay;
pub mod window;
