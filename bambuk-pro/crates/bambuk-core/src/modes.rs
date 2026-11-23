//! Подбор видеорежима для Potato-масштаба (чистая логика, тестируется без Windows).
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Mode { pub w: u32, pub h: u32, pub hz: u32 }

pub fn pick_mode(native: Mode, modes: &[Mode], scale: f32) -> Option<Mode> {
    let scale = scale.clamp(0.3, 1.0);
    if scale >= 0.999 { return Some(native); }
    let fits = |m: &&Mode| m.w <= native.w && m.h <= native.h && m.w >= 640 && m.h >= 360;
    let target_w = native.w as f32 * scale;
    let aspect = native.w as f32 / native.h as f32;
    let same_aspect = |m: &&Mode| ((m.w as f32 / m.h as f32) - aspect).abs() / aspect < 0.02;
    let best = |it: Vec<&Mode>| it.into_iter().min_by(|a, b| {
        let da = (a.w as f32 - target_w).abs(); let db = (b.w as f32 - target_w).abs();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            .then((a.hz as i32 - native.hz as i32).abs().cmp(&(b.hz as i32 - native.hz as i32).abs()))
    }).copied();
    let same_hz: Vec<&Mode> = modes.iter().filter(fits).filter(same_aspect).filter(|m| m.hz == native.hz).collect();
    if let Some(m) = best(same_hz) { return Some(m); }
    let any_hz: Vec<&Mode> = modes.iter().filter(fits).filter(same_aspect).collect();
    if let Some(m) = best(any_hz) { return Some(m); }
    let target_area = native.w as f32 * native.h as f32 * scale * scale;
    modes.iter().filter(fits).min_by(|a, b| {
        let da = ((a.w * a.h) as f32 - target_area).abs(); let db = ((b.w * b.h) as f32 - target_area).abs();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    }).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn m(w: u32, h: u32, hz: u32) -> Mode { Mode { w, h, hz } }
    fn list() -> Vec<Mode> {
        vec![m(1920, 1080, 144), m(1920, 1080, 60), m(1600, 900, 144), m(1366, 768, 60), m(1280, 720, 144), m(1280, 720, 60), m(1024, 768, 60), m(800, 600, 60), m(640, 480, 60)]
    }
    #[test]
    fn full_scale_keeps_native() { assert_eq!(pick_mode(m(1920, 1080, 144), &list(), 1.0), Some(m(1920, 1080, 144))); }
    #[test]
    fn picks_same_aspect_and_refresh() {
        let n = m(1920, 1080, 144);
        assert_eq!(pick_mode(n, &list(), 0.85), Some(m(1600, 900, 144)));
        assert_eq!(pick_mode(n, &list(), 0.6), Some(m(1280, 720, 144)));
        assert_eq!(pick_mode(n, &list(), 0.5), Some(m(1280, 720, 144)), "ниже 1280x720 в списке нет 16:9");
    }
    #[test]
    fn never_exceeds_native_and_handles_missing_aspect() {
        let n = m(2560, 1440, 165);
        let only = vec![m(1920, 1080, 60), m(1024, 768, 60), m(3840, 2160, 60)];
        let r = pick_mode(n, &only, 0.75).unwrap();
        assert!(r.w <= n.w && r.h <= n.h);
        assert_eq!(pick_mode(n, &[], 0.5), None);
    }
}
