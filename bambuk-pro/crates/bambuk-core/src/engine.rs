//! Engine: все изменения системы живут в одном выделенном потоке (Magnification API привязан к
//! потоку инициализации). Каждое изменение проверяется read-back'ом, откатывается при выходе.
use crate::model::Profile;
use crate::report::Report;
use serde::Serialize;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};

/// Резервная копия исходных системных значений на диске: если процесс убили, при следующем запуске
/// Bambuk Pro вернёт гамму и vibrance в исходное состояние.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Backup { pub gamma: Option<Vec<u16>>, pub vibrance: Option<i32> }
impl Backup {
    fn file(dir: &Path) -> PathBuf { dir.join("pending-revert.json") }
    pub fn load(dir: &Path) -> Option<Backup> {
        let b: Backup = serde_json::from_str(&std::fs::read_to_string(Self::file(dir)).ok()?).ok()?;
        if let Some(g) = &b.gamma { if g.len() != 768 { return None; } }
        Some(b)
    }
    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        if self.gamma.is_none() && self.vibrance.is_none() { Self::clear(dir); return Ok(()); }
        std::fs::create_dir_all(dir)?;
        std::fs::write(Self::file(dir), serde_json::to_string(self).unwrap_or_default())
    }
    pub fn clear(dir: &Path) { let _ = std::fs::remove_file(Self::file(dir)); }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState { pub running: bool, pub frames: u64, pub error: Option<String> }

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub supported: bool,
    pub color_active: bool,
    pub gamma_active: bool,
    pub vibrance_active: bool,
    pub display_changed: bool,
    pub native_mode: Option<crate::modes::Mode>,
    pub current_mode: Option<crate::modes::Mode>,
    pub overlay: OverlayState,
    pub recovered: Option<String>,
}

enum Msg {
    Apply(Box<Profile>, Option<isize>, Sender<Vec<Report>>),
    Revert(Sender<Vec<Report>>),
    Status(Sender<EngineStatus>),
}

#[derive(Clone)]
pub struct EngineHandle { tx: Sender<Msg> }

impl EngineHandle {
    pub fn spawn(dir: PathBuf) -> EngineHandle {
        let (tx, rx) = channel::<Msg>();
        std::thread::Builder::new().name("bambuk-engine".into()).spawn(move || {
            let mut e = imp::Engine::new(dir);
            while let Ok(m) = rx.recv() {
                match m {
                    Msg::Apply(p, hwnd, r) => { let _ = r.send(e.apply(&p, hwnd)); }
                    Msg::Revert(r) => { let _ = r.send(e.revert_all()); }
                    Msg::Status(r) => { let _ = r.send(e.status()); }
                }
            }
            e.revert_all(); // все хэндлы закрыты → откат
        }).expect("engine thread");
        EngineHandle { tx }
    }
    pub fn apply(&self, p: &Profile, hwnd: Option<isize>) -> Vec<Report> {
        let (r, rx) = channel();
        if self.tx.send(Msg::Apply(Box::new(p.clone()), hwnd, r)).is_err() { return vec![Report::failed("engine", "Поток применения настроек остановлен")]; }
        rx.recv().unwrap_or_else(|_| vec![Report::failed("engine", "Нет ответа от потока применения")])
    }
    pub fn revert_all(&self) -> Vec<Report> {
        let (r, rx) = channel();
        if self.tx.send(Msg::Revert(r)).is_err() { return vec![]; }
        rx.recv().unwrap_or_default()
    }
    pub fn status(&self) -> EngineStatus {
        let (r, rx) = channel();
        if self.tx.send(Msg::Status(r)).is_err() { return EngineStatus::default(); }
        rx.recv().unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub struct Engine;
    impl Engine {
        pub fn new(_dir: std::path::PathBuf) -> Self { Engine }
        pub fn apply(&mut self, _p: &Profile, _h: Option<isize>) -> Vec<Report> { vec![Report::unsupported("engine", "Bambuk Pro работает только в Windows")] }
        pub fn revert_all(&mut self) -> Vec<Report> { vec![] }
        pub fn status(&self) -> EngineStatus { EngineStatus::default() }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::colormath::*;
    use crate::modes::{pick_mode, Mode};
    use crate::win::overlay::{Overlay, ShaderParams};
    use crate::win::{display, gamma, magnify, nvapi};
    use std::time::{Duration, Instant};

    pub struct Engine {
        mag_on: bool,
        gamma_orig: Option<[u16; 768]>,
        gamma_set: bool,
        vib_orig: Option<i32>,
        vib_set: bool,
        native: Option<Mode>,
        overlay: Option<Overlay>,
        dir: PathBuf,
        recovered: Option<String>,
    }

    impl Engine {
        pub fn new(dir: PathBuf) -> Self {
            let mut e = Engine { mag_on: false, gamma_orig: None, gamma_set: false, vib_orig: None, vib_set: false, native: None, overlay: None, dir, recovered: None };
            if let Some(b) = Backup::load(&e.dir) {
                let mut done = Vec::new();
                if let Some(g) = b.gamma { if let Ok(arr) = <[u16; 768]>::try_from(g) { if gamma::set(&arr).is_ok() { done.push("гамма"); } } }
                if let Some(v) = b.vibrance { if nvapi::restore_level(v).is_ok() { done.push("vibrance"); } }
                Backup::clear(&e.dir);
                if !done.is_empty() { e.recovered = Some(format!("После прошлого аварийного завершения восстановлено: {}", done.join(", "))); }
            }
            e
        }
        fn persist_backup(&self) {
            let b = Backup {
                gamma: if self.gamma_set { self.gamma_orig.map(|g| g.to_vec()) } else { None },
                vibrance: if self.vib_set { self.vib_orig } else { None },
            };
            let _ = b.save(&self.dir);
        }

        pub fn apply(&mut self, p: &Profile, hwnd: Option<isize>) -> Vec<Report> {
            let mut out = Vec::new();
            out.push(self.potato(p));
            out.push(self.color(p));
            out.push(self.gamma(p));
            let (vib_report, vib_shader) = self.vibrance(p);
            out.push(vib_report);
            out.push(self.overlay(p, hwnd, vib_shader));
            out
        }

        fn potato(&mut self, p: &Profile) -> Report {
            let scale = p.potato.scale();
            if scale >= 0.999 {
                if let Some(native) = self.native {
                    if let Err(e) = display::reset() { return Report::failed("potato", e); }
                    return match display::current() {
                        Ok(c) if c.w == native.w && c.h == native.h => { self.native = None; Report::applied("potato", format!("Исходное разрешение {}x{} восстановлено", native.w, native.h)) }
                        Ok(c) => Report::failed("potato", format!("Ожидалось {}x{}, сейчас {}x{}", native.w, native.h, c.w, c.h)),
                        Err(e) => Report::failed("potato", e),
                    };
                }
                return Report::skipped("potato", "Default: разрешение не менялось");
            }
            let cur = match display::current() { Ok(c) => c, Err(e) => return Report::failed("potato", e) };
            let native = self.native.unwrap_or(cur);
            let Some(target) = pick_mode(native, &display::list(), scale) else { return Report::failed("potato", "В списке видеорежимов нет подходящего меньшего разрешения") };
            if target.w == cur.w && target.h == cur.h && target.hz == cur.hz { self.native = Some(native); return Report::applied("potato", format!("Уже установлено {}x{}", cur.w, cur.h)); }
            self.native = Some(native);
            if let Err(e) = display::set(target) { return Report::failed("potato", e); }
            match display::current() {
                Ok(c) if c.w == target.w && c.h == target.h => Report::applied("potato", format!("Разрешение {}x{} → {}x{} @{} Гц. Если игра не подхватила изменение — перезапустите её", native.w, native.h, c.w, c.h, c.hz)),
                Ok(c) => Report::failed("potato", format!("Система осталась на {}x{}", c.w, c.h)),
                Err(e) => Report::failed("potato", e),
            }
        }

        fn color(&mut self, p: &Profile) -> Report {
            let m = full_matrix(&p.color, &p.filters);
            if is_identity(&m) {
                if !self.mag_on { return Report::skipped("color", "Цвет нейтральный — ничего не менялось"); }
                let r = magnify::set_verified(&IDENTITY);
                magnify::shutdown(); self.mag_on = false;
                return match r { Ok(()) => Report::applied("color", "Цвет сброшен к исходному"), Err(e) => Report::failed("color", e) };
            }
            if !self.mag_on { if let Err(e) = magnify::init() { return Report::failed("color", e); } self.mag_on = true; }
            match magnify::set_verified(&m) {
                Ok(()) => Report::applied("color", "Цветовая матрица применена и подтверждена системой"),
                Err(e) => Report::failed("color", format!("{e}. Возможно, включена Лупа или цветовой фильтр Windows")),
            }
        }

        fn gamma(&mut self, p: &Profile) -> Report {
            if (p.color.gamma - 1.0).abs() < 0.005 {
                if self.gamma_set {
                    if let Some(o) = self.gamma_orig {
                        return match gamma::set_verified(&o) { Ok(()) => { self.gamma_set = false; self.persist_backup(); Report::applied("gamma", "Исходная гамма восстановлена") } Err(e) => Report::failed("gamma", e) };
                    }
                }
                return Report::skipped("gamma", "Гамма нейтральная");
            }
            if self.gamma_orig.is_none() { match gamma::get() { Ok(r) => self.gamma_orig = Some(r), Err(e) => return Report::failed("gamma", e) } }
            match gamma::set_verified(&gamma_ramp(p.color.gamma)) {
                Ok(()) => { self.gamma_set = true; self.persist_backup(); Report::applied("gamma", format!("Гамма {:.2} применена и подтверждена", p.color.gamma)) }
                Err(e) => Report::failed("gamma", e),
            }
        }

        /// Возвращает (отчёт, нужно ли считать vibrance шейдером overlay).
        fn vibrance(&mut self, p: &Profile) -> (Report, bool) {
            if p.color.vibrance.abs() < 0.5 {
                if self.vib_set {
                    if let Some(o) = self.vib_orig {
                        return match nvapi::restore_level(o) { Ok(()) => { self.vib_set = false; self.persist_backup(); (Report::applied("vibrance", "Исходная насыщенность драйвера восстановлена"), false) } Err(e) => (Report::failed("vibrance", e), false) };
                    }
                }
                return (Report::skipped("vibrance", "Vibrance нейтральный"), false);
            }
            if self.vib_orig.is_none() { if let Ok(l) = nvapi::current_level() { self.vib_orig = Some(l); } }
            match nvapi::set_vibrance(p.color.vibrance) {
                Ok(l) => { self.vib_set = true; self.persist_backup(); (Report::applied("vibrance", format!("Digital Vibrance драйвера NVIDIA: уровень {l}")), false) }
                Err(e) => {
                    let overlay_wanted = p.overlay.enabled || p.color.sharpness > 0.0;
                    if overlay_wanted { (Report::skipped("vibrance", format!("NVAPI недоступен ({e}); vibrance применит overlay-шейдер")), true) }
                    else { (Report::unsupported("vibrance", format!("{e}. Включите Overlay во Filters — тогда vibrance применит шейдер")), false) }
                }
            }
        }

        fn overlay(&mut self, p: &Profile, hwnd: Option<isize>, vib_shader: bool) -> Report {
            let want = p.overlay.enabled || p.color.sharpness > 0.0;
            if !want {
                if let Some(mut o) = self.overlay.take() { o.stop(); return Report::applied("overlay", "Overlay остановлен"); }
                return Report::skipped("overlay", "Overlay выключен");
            }
            let Some(h) = hwnd else {
                if let Some(mut o) = self.overlay.take() { o.stop(); }
                return Report::skipped("overlay", "Игра не запущена — overlay включится, когда появится окно ROB RUN");
            };
            let sp = ShaderParams { sharpness: p.color.sharpness / 100.0, vibrance: if vib_shader { p.color.vibrance / 100.0 } else { 0.0 } };
            if let Some(o) = &self.overlay {
                if o.alive() && o.game_hwnd == h { o.set_params(sp); return Report::applied("overlay", "Параметры overlay обновлены на лету"); }
            }
            if let Some(mut o) = self.overlay.take() { o.stop(); }
            let o = Overlay::start(h, sp);
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(2500) && o.alive() && o.error().is_none() && o.frames() == 0 { std::thread::sleep(Duration::from_millis(50)); }
            let r = if let Some(e) = o.error() { Report::failed("overlay", format!("Захват окна игры не удался: {e}")) }
                else if !o.alive() { Report::failed("overlay", "Overlay завершился сразу после запуска") }
                else if o.frames() > 0 { Report::applied("overlay", format!("Overlay выводит кадры ({} шт. за запуск)", o.frames())) }
                else { Report::skipped("overlay", "Overlay запущен и ждёт, пока окно игры станет активным (кадры выводятся только в фокусе)") };
            if o.alive() { self.overlay = Some(o); }
            r
        }

        pub fn revert_all(&mut self) -> Vec<Report> {
            let mut out = Vec::new();
            if let Some(mut o) = self.overlay.take() { o.stop(); out.push(Report::applied("overlay", "Overlay остановлен")); }
            if self.vib_set { if let Some(l) = self.vib_orig { out.push(match nvapi::restore_level(l) { Ok(()) => { self.vib_set = false; self.persist_backup(); Report::applied("vibrance", "Vibrance восстановлен") } Err(e) => Report::failed("vibrance", e) }); } }
            if self.gamma_set { if let Some(o) = self.gamma_orig { out.push(match gamma::set_verified(&o) { Ok(()) => { self.gamma_set = false; self.persist_backup(); Report::applied("gamma", "Гамма восстановлена") } Err(e) => Report::failed("gamma", e) }); } }
            if self.mag_on { let r = magnify::set_verified(&IDENTITY); magnify::shutdown(); self.mag_on = false; out.push(match r { Ok(()) => Report::applied("color", "Цвет восстановлен"), Err(e) => Report::failed("color", e) }); }
            if self.native.is_some() { out.push(match display::reset() { Ok(()) => { self.native = None; Report::applied("potato", "Разрешение восстановлено") } Err(e) => Report::failed("potato", e) }); }
            out
        }

        pub fn status(&self) -> EngineStatus {
            EngineStatus {
                supported: true, color_active: self.mag_on, gamma_active: self.gamma_set, vibrance_active: self.vib_set,
                display_changed: self.native.is_some(), native_mode: self.native, current_mode: display::current().ok(),
                overlay: self.overlay.as_ref().map(|o| OverlayState { running: o.alive(), frames: o.frames(), error: o.error() }).unwrap_or_default(),
                recovered: self.recovered.clone(),
            }
        }
    }
    impl Drop for Engine { fn drop(&mut self) { self.revert_all(); } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_roundtrip_and_validation() {
        let d = std::env::temp_dir().join(format!("bambuk-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert!(Backup::load(&d).is_none());
        let b = Backup { gamma: Some(vec![7; 768]), vibrance: Some(12) };
        b.save(&d).unwrap();
        assert_eq!(Backup::load(&d), Some(b));
        Backup { gamma: None, vibrance: None }.save(&d).unwrap();
        assert!(Backup::load(&d).is_none(), "пустая копия удаляет файл");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("pending-revert.json"), "{\"gamma\":[1,2,3]}").unwrap();
        assert!(Backup::load(&d).is_none(), "рампа неверной длины отвергается");
    }
    #[test]
    fn engine_handle_reports_unsupported_off_windows() {
        let h = EngineHandle::spawn(std::env::temp_dir());
        let r = h.apply(&Profile::default(), None);
        assert!(!r.is_empty());
        #[cfg(not(windows))]
        assert_eq!(r[0].status, crate::report::Status::Unsupported);
        let _ = h.status();
    }
}
