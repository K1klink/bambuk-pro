//! Цветовая математика. Матрицы 5x5 в соглашении Magnification API:
//! строка-вектор [r g b a 1] умножается на матрицу, перенос лежит в строке 4.
use crate::model::{ColorParams, FilterParams, Toggle};

pub type M5 = [[f32; 5]; 5];
pub const IDENTITY: M5 = [
    [1.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 1.0],
];
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Применить сначала `a`, затем `b` (для row-vector: v·a·b).
pub fn mul(a: &M5, b: &M5) -> M5 {
    let mut r = [[0.0; 5]; 5];
    for i in 0..5 { for j in 0..5 { for k in 0..5 { r[i][j] += a[i][k] * b[k][j]; } } }
    r
}
fn lerp(a: &M5, b: &M5, t: f32) -> M5 {
    let mut r = *a;
    for i in 0..5 { for j in 0..5 { r[i][j] = a[i][j] + (b[i][j] - a[i][j]) * t; } }
    r
}
pub fn is_identity(m: &M5) -> bool {
    (0..5).all(|i| (0..5).all(|j| (m[i][j] - IDENTITY[i][j]).abs() < 1e-4))
}
pub fn apply(m: &M5, rgb: [f32; 3]) -> [f32; 3] {
    let v = [rgb[0], rgb[1], rgb[2], 1.0, 1.0];
    let mut o = [0.0; 3];
    for j in 0..3 { for i in 0..5 { o[j] += v[i] * m[i][j]; } }
    o
}

pub fn saturation(factor: f32) -> M5 {
    let mut m = IDENTITY;
    for i in 0..3 { for j in 0..3 {
        m[i][j] = (1.0 - factor) * LUMA[i] + if i == j { factor } else { 0.0 };
    } }
    m
}
pub fn contrast(c: f32) -> M5 {
    let mut m = IDENTITY;
    for i in 0..3 { m[i][i] = c; m[4][i] = 0.5 * (1.0 - c); }
    m
}
pub fn brightness(add: f32) -> M5 {
    let mut m = IDENTITY;
    for i in 0..3 { m[4][i] = add; }
    m
}
pub fn gains(r: f32, g: f32, b: f32) -> M5 {
    let mut m = IDENTITY;
    m[0][0] = r.max(0.0); m[1][1] = g.max(0.0); m[2][2] = b.max(0.0);
    m
}

fn amt(t: &Toggle) -> f32 { (t.amount / 100.0).clamp(0.0, 1.0) }

/// Матрица для ColorTuner (без gamma/vibrance/sharpness — они не линейны и идут другими путями).
pub fn color_matrix(p: &ColorParams) -> M5 {
    let mut m = IDENTITY;
    m = mul(&m, &saturation((1.0 + p.saturation / 100.0).max(0.0)));
    m = mul(&m, &contrast((1.0 + p.contrast / 100.0).max(0.0)));
    m = mul(&m, &brightness(p.brightness / 100.0 * 0.3));
    let k = p.temperature / 100.0;
    m = mul(&m, &gains(1.0 + 0.25 * k, 1.0, 1.0 - 0.35 * k));
    let t = p.tint / 100.0;
    m = mul(&m, &gains(1.0 + 0.05 * t, 1.0 - 0.2 * t, 1.0 + 0.05 * t));
    m
}

pub fn filter_matrix(f: &FilterParams) -> M5 {
    let mut m = IDENTITY;
    if f.grayscale.enabled { m = mul(&m, &lerp(&IDENTITY, &saturation(0.0), amt(&f.grayscale))); }
    if f.sepia.enabled {
        let sep: M5 = [
            [0.393, 0.349, 0.272, 0.0, 0.0],
            [0.769, 0.686, 0.534, 0.0, 0.0],
            [0.189, 0.168, 0.131, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0],
        ];
        m = mul(&m, &lerp(&IDENTITY, &sep, amt(&f.sepia)));
    }
    if f.invert.enabled {
        let mut inv = IDENTITY;
        for i in 0..3 { inv[i][i] = -1.0; inv[4][i] = 1.0; }
        m = mul(&m, &lerp(&IDENTITY, &inv, amt(&f.invert)));
    }
    if f.blue_cut.enabled { let a = amt(&f.blue_cut); m = mul(&m, &gains(1.0, 1.0 - 0.12 * a, 1.0 - 0.5 * a)); }
    if f.high_contrast.enabled {
        let a = amt(&f.high_contrast);
        m = mul(&m, &contrast(1.0 + 0.8 * a));
        m = mul(&m, &saturation(1.0 + 0.2 * a));
    }
    if f.red_boost.enabled { let a = amt(&f.red_boost); m = mul(&m, &gains(1.0 + 0.35 * a, 1.0, 1.0)); }
    m
}

pub fn full_matrix(c: &ColorParams, f: &FilterParams) -> M5 { mul(&color_matrix(c), &filter_matrix(f)) }

pub fn flatten(m: &M5) -> [f32; 25] {
    let mut o = [0.0; 25];
    for i in 0..5 { for j in 0..5 { o[i * 5 + j] = m[i][j]; } }
    o
}
pub fn unflatten(a: &[f32; 25]) -> M5 {
    let mut m = [[0.0; 5]; 5];
    for i in 0..5 { for j in 0..5 { m[i][j] = a[i * 5 + j]; } }
    m
}
pub fn matrices_close(a: &M5, b: &M5, eps: f32) -> bool {
    (0..5).all(|i| (0..5).all(|j| (a[i][j] - b[i][j]).abs() <= eps))
}

/// Рампа гаммы для SetDeviceGammaRamp: 3 канала по 256 значений.
pub fn gamma_ramp(gamma: f32) -> [u16; 768] {
    let g = gamma.clamp(0.6, 1.8);
    let mut r = [0u16; 768];
    for c in 0..3 { for i in 0..256 {
        let v = (i as f32 / 255.0).powf(1.0 / g);
        r[c * 256 + i] = (v * 65535.0).round().clamp(0.0, 65535.0) as u16;
    } }
    r
}
pub fn ramp_is_identity(r: &[u16; 768]) -> bool {
    (0..3).all(|c| (0..256).all(|i| (r[c * 256 + i] as i32 - (i as i32 * 257)).abs() <= 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neutral_params_give_identity() {
        assert!(is_identity(&full_matrix(&ColorParams::default(), &FilterParams::default())));
    }
    #[test]
    fn grayscale_makes_channels_equal() {
        let mut f = FilterParams::default(); f.grayscale.enabled = true;
        let o = apply(&filter_matrix(&f), [0.9, 0.2, 0.4]);
        assert!((o[0] - o[1]).abs() < 1e-5 && (o[1] - o[2]).abs() < 1e-5);
    }
    #[test]
    fn saturation_zero_equals_luma() {
        let o = apply(&saturation(0.0), [1.0, 0.0, 0.0]);
        assert!((o[0] - 0.2126).abs() < 1e-5);
    }
    #[test]
    fn contrast_keeps_midgray() {
        let o = apply(&contrast(1.7), [0.5, 0.5, 0.5]);
        assert!(o.iter().all(|v| (v - 0.5).abs() < 1e-5));
        let o = apply(&contrast(1.7), [0.75, 0.25, 0.5]);
        assert!(o[0] > 0.75 && o[1] < 0.25);
    }
    #[test]
    fn brightness_adds() {
        let o = apply(&brightness(0.1), [0.2, 0.2, 0.2]);
        assert!((o[0] - 0.3).abs() < 1e-5);
    }
    #[test]
    fn warm_boosts_red_cuts_blue() {
        let p = ColorParams { temperature: 100.0, ..Default::default() };
        let o = apply(&color_matrix(&p), [0.5, 0.5, 0.5]);
        assert!(o[0] > 0.5 && o[2] < 0.5);
        let p = ColorParams { temperature: -100.0, ..Default::default() };
        let o = apply(&color_matrix(&p), [0.5, 0.5, 0.5]);
        assert!(o[0] < 0.5 && o[2] > 0.5);
    }
    #[test]
    fn tint_moves_green() {
        let p = ColorParams { tint: 100.0, ..Default::default() };
        assert!(apply(&color_matrix(&p), [0.5; 3])[1] < 0.5);
    }
    #[test]
    fn invert_full() {
        let mut f = FilterParams::default(); f.invert.enabled = true;
        let o = apply(&filter_matrix(&f), [0.2, 0.8, 0.5]);
        assert!((o[0] - 0.8).abs() < 1e-5 && (o[1] - 0.2).abs() < 1e-5);
    }
    #[test]
    fn filter_amount_blends() {
        let mut f = FilterParams::default(); f.grayscale.enabled = true; f.grayscale.amount = 50.0;
        let full = apply(&saturation(0.0), [1.0, 0.0, 0.0]);
        let half = apply(&filter_matrix(&f), [1.0, 0.0, 0.0]);
        assert!((half[0] - (1.0 + full[0]) / 2.0).abs() < 1e-5);
    }
    #[test]
    fn gamma_ramp_monotonic_and_identity() {
        assert!(ramp_is_identity(&gamma_ramp(1.0)));
        let r = gamma_ramp(1.4);
        assert!(!ramp_is_identity(&r));
        assert!(r[128] > 128 * 257);
        assert!((1..256).all(|i| r[i] >= r[i - 1]));
        assert_eq!(r[255], 65535);
    }
    #[test]
    fn flatten_roundtrip() {
        let m = full_matrix(&ColorParams { saturation: 30.0, contrast: 20.0, ..Default::default() }, &FilterParams::default());
        assert!(matrices_close(&m, &unflatten(&flatten(&m)), 0.0));
    }
}
