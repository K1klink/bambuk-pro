//! Модели данных: параметры цвета, фильтры, Potato-уровни, пресеты.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ColorParams {
    pub saturation: f32,  // -100..100
    pub vibrance: f32,    // -100..100
    pub contrast: f32,    // -100..100
    pub brightness: f32,  // -100..100
    pub gamma: f32,       // 0.6..1.8, 1.0 = нейтрально
    pub temperature: f32, // -100 (холоднее) .. 100 (теплее)
    pub tint: f32,        // -100 (зелёный) .. 100 (пурпурный)
    pub sharpness: f32,   // 0..100, требует overlay-движок
}

impl Default for ColorParams {
    fn default() -> Self {
        Self { saturation: 0.0, vibrance: 0.0, contrast: 0.0, brightness: 0.0, gamma: 1.0, temperature: 0.0, tint: 0.0, sharpness: 0.0 }
    }
}

impl ColorParams {
    pub fn clamp(&mut self) {
        let c = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 0.0_f32.clamp(lo, hi) };
        self.saturation = c(self.saturation, -100.0, 100.0);
        self.vibrance = c(self.vibrance, -100.0, 100.0);
        self.contrast = c(self.contrast, -100.0, 100.0);
        self.brightness = c(self.brightness, -100.0, 100.0);
        self.gamma = if self.gamma.is_finite() { self.gamma.clamp(0.6, 1.8) } else { 1.0 };
        self.temperature = c(self.temperature, -100.0, 100.0);
        self.tint = c(self.tint, -100.0, 100.0);
        self.sharpness = c(self.sharpness, 0.0, 100.0);
    }
    pub fn is_neutral(&self) -> bool { *self == ColorParams::default() }
}

/// Цветовые фильтры. Все реализуются матрицей цвета (реальное воздействие на экран).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FilterParams {
    pub grayscale: Toggle,
    pub sepia: Toggle,
    pub invert: Toggle,
    pub blue_cut: Toggle,
    pub high_contrast: Toggle,
    pub red_boost: Toggle,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Toggle {
    pub enabled: bool,
    pub amount: f32, // 0..100
}
impl Default for Toggle { fn default() -> Self { Self { enabled: false, amount: 100.0 } } }

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PotatoLevel { Default, Potato, MaxPotato, Extreme, UltraPotato }

impl PotatoLevel {
    /// Доля от нативного разрешения по ширине/высоте.
    pub fn render_scale(self) -> f32 {
        match self { Self::Default => 1.0, Self::Potato => 0.85, Self::MaxPotato => 0.75, Self::Extreme => 0.6, Self::UltraPotato => 0.5 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PotatoParams {
    pub level: PotatoLevel,
    /// Если задано — перекрывает масштаб уровня (0.4..1.0).
    pub custom_scale: Option<f32>,
}
impl Default for PotatoParams { fn default() -> Self { Self { level: PotatoLevel::Default, custom_scale: None } } }
impl PotatoParams {
    pub fn scale(&self) -> f32 {
        self.custom_scale.filter(|s| s.is_finite()).map(|s| s.clamp(0.4, 1.0)).unwrap_or_else(|| self.level.render_scale())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlayParams {
    pub enabled: bool,
}

/// Полный профиль = графика + цвет + фильтры.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub potato: PotatoParams,
    pub color: ColorParams,
    pub filters: FilterParams,
    pub overlay: OverlayParams,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PresetKind { Graphics, Color, Full }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub kind: PresetKind,
    pub builtin: bool,
    pub profile: Profile,
}

pub fn builtin_color_presets() -> Vec<Preset> {
    let mk = |id: &str, name: &str, c: ColorParams| Preset {
        id: format!("builtin-color-{id}"), name: name.into(), kind: PresetKind::Color, builtin: true,
        profile: Profile { color: c, ..Profile::default() },
    };
    vec![
        mk("default", "Default", ColorParams::default()),
        mk("vibrant", "Vibrant", ColorParams { saturation: 25.0, vibrance: 40.0, contrast: 10.0, ..Default::default() }),
        mk("competitive", "Competitive", ColorParams { saturation: 10.0, contrast: 20.0, brightness: 8.0, gamma: 1.15, sharpness: 35.0, ..Default::default() }),
        mk("high-contrast", "High Contrast", ColorParams { contrast: 45.0, saturation: 15.0, gamma: 0.95, ..Default::default() }),
        mk("cold", "Cold", ColorParams { temperature: -45.0, tint: -5.0, contrast: 5.0, ..Default::default() }),
        mk("warm", "Warm", ColorParams { temperature: 45.0, saturation: 8.0, ..Default::default() }),
    ]
}

pub fn builtin_graphics_presets() -> Vec<Preset> {
    use PotatoLevel::*;
    [(Default, "Default"), (Potato, "Potato"), (MaxPotato, "Max Potato"), (Extreme, "Extreme"), (UltraPotato, "Ultra Potato")]
        .into_iter()
        .map(|(lvl, name)| Preset {
            id: format!("builtin-gfx-{}", name.to_lowercase().replace(' ', "-")),
            name: name.into(), kind: PresetKind::Graphics, builtin: true,
            profile: Profile { potato: PotatoParams { level: lvl, custom_scale: None }, ..Profile::default() },
        })
        .collect()
}
