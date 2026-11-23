//! Хранилище настроек и пресетов (JSON на диске, атомарная запись).
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);
const FORMAT: &str = "bambuk-pro-preset";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub auto_apply_on_game_start: bool,
    pub auto_revert_on_game_exit: bool,
}
impl Default for AppSettings {
    fn default() -> Self { Self { auto_apply_on_game_start: false, auto_revert_on_game_exit: true } }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct StoreData {
    pub version: u32,
    pub profile: Profile,
    pub user_presets: Vec<Preset>,
    pub settings: AppSettings,
    pub active_graphics: Option<String>,
    pub active_color: Option<String>,
}

pub struct Store { path: PathBuf, pub data: StoreData }

#[derive(Serialize, Deserialize)]
struct Envelope { format: String, version: u32, preset: Preset }

fn new_id() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("u-{:x}-{:x}", t, COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn sanitize(p: &mut Profile) {
    p.color.clamp();
    if let Some(s) = p.potato.custom_scale { p.potato.custom_scale = if s.is_finite() { Some(s.clamp(0.4, 1.0)) } else { None }; }
    for t in [&mut p.filters.grayscale, &mut p.filters.sepia, &mut p.filters.invert, &mut p.filters.blue_cut, &mut p.filters.high_contrast, &mut p.filters.red_boost] {
        t.amount = if t.amount.is_finite() { t.amount.clamp(0.0, 100.0) } else { 100.0 };
    }
}

impl Store {
    pub fn open(dir: &Path) -> Store {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("bambuk-pro.json");
        let data = match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<StoreData>(&s) {
                Ok(mut d) => { sanitize(&mut d.profile); d }
                Err(_) => { let _ = std::fs::rename(&path, dir.join("bambuk-pro.corrupt.json")); StoreData::default() }
            },
            Err(_) => StoreData::default(),
        };
        let mut s = Store { path, data };
        s.data.version = 1;
        s
    }
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("запись {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("замена файла: {e}"))
    }
    pub fn all_presets(&self) -> Vec<Preset> {
        let mut v = builtin_graphics_presets();
        v.extend(builtin_color_presets());
        v.extend(self.data.user_presets.iter().cloned());
        v
    }
    pub fn find(&self, id: &str) -> Option<Preset> { self.all_presets().into_iter().find(|p| p.id == id) }

    fn unique_name(&self, base: &str) -> String {
        let base = { let t = base.trim(); if t.is_empty() { "Preset" } else { t } }.chars().take(48).collect::<String>();
        let taken = |n: &str| self.all_presets().iter().any(|p| p.name.eq_ignore_ascii_case(n));
        if !taken(&base) { return base; }
        (2..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap()
    }
    pub fn create_preset(&mut self, name: &str, kind: PresetKind, from: &Profile) -> Result<Preset, String> {
        let mut profile = Profile::default();
        match kind {
            PresetKind::Graphics => profile.potato = from.potato.clone(),
            PresetKind::Color => { profile.color = from.color.clone(); profile.filters = from.filters.clone(); }
            PresetKind::Full => profile = from.clone(),
        }
        sanitize(&mut profile);
        let p = Preset { id: new_id(), name: self.unique_name(name), kind, builtin: false, profile };
        self.data.user_presets.push(p.clone());
        self.save()?;
        Ok(p)
    }
    pub fn update_preset(&mut self, id: &str, from: &Profile) -> Result<Preset, String> {
        let p = self.data.user_presets.iter_mut().find(|p| p.id == id).ok_or("Пресет не найден или встроенный (его нельзя менять)")?;
        match p.kind {
            PresetKind::Graphics => p.profile.potato = from.potato.clone(),
            PresetKind::Color => { p.profile.color = from.color.clone(); p.profile.filters = from.filters.clone(); }
            PresetKind::Full => p.profile = from.clone(),
        }
        sanitize(&mut p.profile);
        let out = p.clone();
        self.save()?;
        Ok(out)
    }
    pub fn rename_preset(&mut self, id: &str, name: &str) -> Result<(), String> {
        if name.trim().is_empty() { return Err("Имя не может быть пустым".into()); }
        let new = { let others: Vec<String> = self.all_presets().into_iter().filter(|p| p.id != id).map(|p| p.name).collect();
            let base: String = name.trim().chars().take(48).collect();
            if !others.iter().any(|n| n.eq_ignore_ascii_case(&base)) { base } else { (2..).map(|i| format!("{base} {i}")).find(|n| !others.iter().any(|o| o.eq_ignore_ascii_case(n))).unwrap() } };
        let p = self.data.user_presets.iter_mut().find(|p| p.id == id).ok_or("Пресет не найден или встроенный")?;
        p.name = new;
        self.save()
    }
    pub fn delete_preset(&mut self, id: &str) -> Result<(), String> {
        let before = self.data.user_presets.len();
        self.data.user_presets.retain(|p| p.id != id);
        if self.data.user_presets.len() == before { return Err("Пресет не найден или встроенный".into()); }
        if self.data.active_graphics.as_deref() == Some(id) { self.data.active_graphics = None; }
        if self.data.active_color.as_deref() == Some(id) { self.data.active_color = None; }
        self.save()
    }
    pub fn export_preset(&self, id: &str) -> Result<String, String> {
        let preset = self.find(id).ok_or("Пресет не найден")?;
        serde_json::to_string_pretty(&Envelope { format: FORMAT.into(), version: 1, preset }).map_err(|e| e.to_string())
    }
    pub fn import_preset(&mut self, json: &str) -> Result<Preset, String> {
        if json.len() > 256 * 1024 { return Err("Файл слишком большой для пресета".into()); }
        let env: Envelope = serde_json::from_str(json).map_err(|e| format!("Это не файл пресета Bambuk Pro: {e}"))?;
        if env.format != FORMAT { return Err("Это не файл пресета Bambuk Pro".into()); }
        if env.version != 1 { return Err(format!("Неподдерживаемая версия пресета: {}", env.version)); }
        let mut p = env.preset;
        sanitize(&mut p.profile);
        p.id = new_id(); p.builtin = false; p.name = self.unique_name(&p.name);
        self.data.user_presets.push(p.clone());
        self.save()?;
        Ok(p)
    }
}

/// Накладывает пресет на профиль согласно его типу.
pub fn merge_preset(profile: &mut Profile, preset: &Preset) {
    match preset.kind {
        PresetKind::Graphics => profile.potato = preset.profile.potato.clone(),
        PresetKind::Color => { profile.color = preset.profile.color.clone(); profile.filters = preset.profile.filters.clone(); }
        PresetKind::Full => *profile = preset.profile.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bambuk-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d); d
    }
    #[test]
    fn persists_between_runs() {
        let d = tmp("persist");
        let mut s = Store::open(&d);
        s.data.profile.color.saturation = 33.0;
        s.data.settings.auto_apply_on_game_start = true;
        s.save().unwrap();
        let s2 = Store::open(&d);
        assert_eq!(s2.data.profile.color.saturation, 33.0);
        assert!(s2.data.settings.auto_apply_on_game_start);
    }
    #[test]
    fn corrupt_file_is_backed_up() {
        let d = tmp("corrupt");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("bambuk-pro.json"), "{ not json").unwrap();
        let s = Store::open(&d);
        assert_eq!(s.data.profile, Profile::default());
        assert!(d.join("bambuk-pro.corrupt.json").exists());
    }
    #[test]
    fn preset_crud() {
        let d = tmp("crud");
        let mut s = Store::open(&d);
        let mut prof = Profile::default();
        prof.color.contrast = 40.0; prof.potato.level = PotatoLevel::Extreme;
        let c = s.create_preset("Мой цвет", PresetKind::Color, &prof).unwrap();
        assert_eq!(c.profile.color.contrast, 40.0);
        assert_eq!(c.profile.potato.level, PotatoLevel::Default, "цветовой пресет не несёт графику");
        let g = s.create_preset("Мой цвет", PresetKind::Graphics, &prof).unwrap();
        assert_eq!(g.name, "Мой цвет 2");
        s.rename_preset(&c.id, "Renamed").unwrap();
        assert_eq!(s.find(&c.id).unwrap().name, "Renamed");
        prof.color.contrast = -10.0;
        s.update_preset(&c.id, &prof).unwrap();
        assert_eq!(s.find(&c.id).unwrap().profile.color.contrast, -10.0);
        s.delete_preset(&c.id).unwrap();
        assert!(s.find(&c.id).is_none());
        assert!(Store::open(&d).find(&g.id).is_some());
    }
    #[test]
    fn builtin_presets_are_protected() {
        let mut s = Store::open(&tmp("builtin"));
        assert!(s.delete_preset("builtin-color-warm").is_err());
        assert!(s.rename_preset("builtin-color-warm", "x").is_err());
        assert!(s.update_preset("builtin-gfx-potato", &Profile::default()).is_err());
        assert!(s.find("builtin-gfx-ultra-potato").is_some());
        assert_eq!(s.all_presets().len(), 5 + 6);
    }
    #[test]
    fn export_import_roundtrip_and_validation() {
        let mut s = Store::open(&tmp("io"));
        let json = s.export_preset("builtin-color-vibrant").unwrap();
        let p = s.import_preset(&json).unwrap();
        assert!(!p.builtin && p.name != "Vibrant" && p.profile.color.saturation == 25.0);
        assert!(s.import_preset("{\"hello\":1}").is_err());
        assert!(s.import_preset("мусор").is_err());
        let evil = json.replace("\"saturation\": 25.0", "\"saturation\": 9999.0");
        assert_eq!(s.import_preset(&evil).unwrap().profile.color.saturation, 100.0, "значения клампятся");
    }
    #[test]
    fn merge_respects_kind() {
        let mut p = Profile::default();
        p.potato.level = PotatoLevel::Potato;
        let warm = builtin_color_presets().into_iter().find(|x| x.name == "Warm").unwrap();
        merge_preset(&mut p, &warm);
        assert_eq!(p.potato.level, PotatoLevel::Potato);
        assert_eq!(p.color.temperature, 45.0);
        let ultra = builtin_graphics_presets().into_iter().find(|x| x.name == "Ultra Potato").unwrap();
        merge_preset(&mut p, &ultra);
        assert_eq!(p.color.temperature, 45.0);
        assert_eq!(p.potato.scale(), 0.5);
    }
    #[test]
    fn potato_scales_are_ordered() {
        use PotatoLevel::*;
        let s: Vec<f32> = [Default, Potato, MaxPotato, Extreme, UltraPotato].iter().map(|l| l.render_scale()).collect();
        assert!(s.windows(2).all(|w| w[0] > w[1]));
        let c = PotatoParams { level: Default, custom_scale: Some(0.1) };
        assert_eq!(c.scale(), 0.4);
    }
}
