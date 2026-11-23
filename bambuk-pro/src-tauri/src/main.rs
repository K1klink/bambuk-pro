#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bambuk_core::engine::{EngineHandle, EngineStatus};
use bambuk_core::game::{self, GAME_EXE};
use bambuk_core::model::*;
use bambuk_core::perf::{PerfMonitor, PerfStatus};
use bambuk_core::report::Report;
use bambuk_core::stats::FrameStats;
use bambuk_core::store::{merge_preset, AppSettings, Store};
use bambuk_core::telemetry::{Sampler, Telemetry};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::path::BaseDirectory;
use tauri::{Manager, RunEvent, State};

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct GameState { running: bool, pid: Option<u32>, has_window: bool, hwnd: Option<isize> }

struct AppState {
    store: Mutex<Store>,
    engine: EngineHandle,
    perf: Mutex<PerfMonitor>,
    perf_exe: PathBuf,
    perf_start_error: Mutex<Option<String>>,
    sampler: Mutex<Sampler>,
    hw: Mutex<Telemetry>,
    game: Mutex<GameState>,
    engine_status: Mutex<EngineStatus>,
    applied: Mutex<bool>,
    reports: Mutex<Vec<Report>>,
    baseline: Mutex<Option<FrameStats>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    profile: Profile,
    presets: Vec<Preset>,
    settings: AppSettings,
    active_graphics: Option<String>,
    active_color: Option<String>,
    applied: bool,
    engine: EngineStatus,
    game: GameState,
    reports: Vec<Report>,
    perf_exe_found: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Live {
    game: GameState,
    frame: FrameStats,
    perf: PerfStatus,
    hw: Telemetry,
    engine: EngineStatus,
    baseline: Option<FrameStats>,
    applied: bool,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

fn snapshot(s: &AppState) -> Snapshot {
    let st = lock(&s.store);
    Snapshot {
        profile: st.data.profile.clone(), presets: st.all_presets(), settings: st.data.settings.clone(),
        active_graphics: st.data.active_graphics.clone(), active_color: st.data.active_color.clone(),
        applied: *lock(&s.applied), engine: lock(&s.engine_status).clone(), game: lock(&s.game).clone(),
        reports: lock(&s.reports).clone(), perf_exe_found: s.perf_exe.exists(),
    }
}

fn do_apply(s: &AppState) -> Vec<Report> {
    let profile = lock(&s.store).data.profile.clone();
    let hwnd = lock(&s.game).hwnd;
    let reports = s.engine.apply(&profile, hwnd);
    *lock(&s.engine_status) = s.engine.status();
    *lock(&s.reports) = reports.clone();
    reports
}
fn do_revert(s: &AppState) {
    let reports = s.engine.revert_all();
    *lock(&s.engine_status) = s.engine.status();
    *lock(&s.reports) = reports;
    *lock(&s.applied) = false;
}

fn refresh_active(st: &mut Store) {
    let p = st.data.profile.clone();
    if let Some(id) = st.data.active_graphics.clone() {
        if st.find(&id).map(|x| x.profile.potato != p.potato).unwrap_or(true) { st.data.active_graphics = None; }
    }
    if let Some(id) = st.data.active_color.clone() {
        if st.find(&id).map(|x| x.profile.color != p.color || x.profile.filters != p.filters).unwrap_or(true) { st.data.active_color = None; }
    }
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Snapshot { snapshot(&state) }

#[tauri::command]
async fn update_profile(mut profile: Profile, state: State<'_, AppState>) -> Result<Snapshot, String> {
    profile.color.clamp();
    {
        let mut st = lock(&state.store);
        st.data.profile = profile;
        refresh_active(&mut st);
        st.save()?;
    }
    if *lock(&state.applied) { do_apply(&state); }
    Ok(snapshot(&state))
}

#[tauri::command]
async fn set_applied(on: bool, state: State<'_, AppState>) -> Result<Snapshot, String> {
    if on { *lock(&state.applied) = true; do_apply(&state); } else { do_revert(&state); }
    Ok(snapshot(&state))
}

#[tauri::command]
async fn apply_preset(id: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    {
        let mut st = lock(&state.store);
        let preset = st.find(&id).ok_or("Пресет не найден")?;
        let mut p = st.data.profile.clone();
        merge_preset(&mut p, &preset);
        st.data.profile = p;
        match preset.kind {
            PresetKind::Graphics => st.data.active_graphics = Some(id),
            PresetKind::Color => st.data.active_color = Some(id),
            PresetKind::Full => { st.data.active_graphics = Some(id.clone()); st.data.active_color = Some(id); }
        }
        st.save()?;
    }
    if *lock(&state.applied) { do_apply(&state); }
    Ok(snapshot(&state))
}

#[tauri::command]
fn create_preset(name: String, kind: PresetKind, state: State<'_, AppState>) -> Result<Snapshot, String> {
    {
        let mut st = lock(&state.store);
        let prof = st.data.profile.clone();
        let p = st.create_preset(&name, kind, &prof)?;
        match kind {
            PresetKind::Graphics => st.data.active_graphics = Some(p.id),
            PresetKind::Color => st.data.active_color = Some(p.id),
            PresetKind::Full => { st.data.active_graphics = Some(p.id.clone()); st.data.active_color = Some(p.id); }
        }
        st.save()?;
    }
    Ok(snapshot(&state))
}
#[tauri::command]
fn save_preset(id: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    { let mut st = lock(&state.store); let prof = st.data.profile.clone(); st.update_preset(&id, &prof)?; }
    Ok(snapshot(&state))
}
#[tauri::command]
fn rename_preset(id: String, name: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    lock(&state.store).rename_preset(&id, &name)?; Ok(snapshot(&state))
}
#[tauri::command]
fn delete_preset(id: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    lock(&state.store).delete_preset(&id)?; Ok(snapshot(&state))
}
#[tauri::command]
fn export_preset(id: String, state: State<'_, AppState>) -> Result<String, String> { lock(&state.store).export_preset(&id) }
#[tauri::command]
fn import_preset(json: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    lock(&state.store).import_preset(&json)?; Ok(snapshot(&state))
}
#[tauri::command]
fn set_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<Snapshot, String> {
    { let mut st = lock(&state.store); st.data.settings = settings; st.save()?; }
    Ok(snapshot(&state))
}

#[tauri::command]
fn live(state: State<'_, AppState>) -> Live {
    let perf = lock(&state.perf);
    let mut status = perf.status();
    if status.note.is_none() { status.note = lock(&state.perf_start_error).clone(); }
    Live {
        game: lock(&state.game).clone(), frame: perf.stats(), perf: status, hw: lock(&state.hw).clone(),
        engine: lock(&state.engine_status).clone(), baseline: lock(&state.baseline).clone(), applied: *lock(&state.applied),
    }
}
#[tauri::command]
fn set_baseline(state: State<'_, AppState>) -> Result<(), String> {
    let f = lock(&state.perf).stats();
    if f.frames == 0 { return Err("Нет данных о кадрах: запустите игру и подождите несколько секунд".into()); }
    *lock(&state.baseline) = Some(f);
    Ok(())
}
#[tauri::command]
fn clear_baseline(state: State<'_, AppState>) { *lock(&state.baseline) = None; }

fn overlay_wanted(p: &Profile) -> bool { p.overlay.enabled || p.color.sharpness > 0.0 }

fn tick_loop(app: tauri::AppHandle) {
    let mut sys = sysinfo::System::new();
    let mut was_running = false;
    let mut last_overlay_try = Instant::now() - Duration::from_secs(10);
    loop {
        std::thread::sleep(Duration::from_millis(1000));
        let s = app.state::<AppState>();
        let found = game::find(&mut sys);
        let hwnd = found.and_then(|g| game::main_window(g.pid));
        *lock(&s.game) = GameState { running: found.is_some(), pid: found.map(|g| g.pid), has_window: hwnd.is_some(), hwnd };
        *lock(&s.hw) = lock(&s.sampler).sample();

        {
            let mut perf = lock(&s.perf);
            if found.is_some() && !perf.is_running() {
                match perf.start(GAME_EXE) { Ok(()) => *lock(&s.perf_start_error) = None, Err(e) => *lock(&s.perf_start_error) = Some(e) }
            }
            if found.is_none() && perf.is_running() { perf.stop(); }
        }

        let running = found.is_some();
        let (auto_apply, auto_revert, wanted) = {
            let st = lock(&s.store);
            (st.data.settings.auto_apply_on_game_start, st.data.settings.auto_revert_on_game_exit, overlay_wanted(&st.data.profile))
        };
        let applied = *lock(&s.applied);
        if running && !was_running {
            if auto_apply && !applied { *lock(&s.applied) = true; do_apply(&s); }
            else if applied { do_apply(&s); }
        } else if !running && was_running && auto_revert && applied {
            do_revert(&s);
        } else if running && applied && wanted && hwnd.is_some() && last_overlay_try.elapsed() > Duration::from_secs(3) {
            let ov = lock(&s.engine_status).overlay.clone();
            if !ov.running { last_overlay_try = Instant::now(); do_apply(&s); }
        }
        was_running = running;
        *lock(&s.engine_status) = s.engine.status();
    }
}

fn resolve_presentmon(app: &tauri::App) -> PathBuf {
    if let Ok(p) = app.path().resolve("resources/PresentMon.exe", BaseDirectory::Resource) { if p.exists() { return p; } }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for c in [dir.join("resources").join("PresentMon.exe"), dir.join("PresentMon.exe"), dir.join("../../resources/PresentMon.exe")] { if c.exists() { return c; } }
        }
    }
    PathBuf::from("resources/PresentMon.exe")
}

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("app data dir");
            let engine = EngineHandle::spawn(dir.clone()); // сначала восстановит значения после аварийного выхода
            let store = Store::open(&dir);
            let perf_exe = resolve_presentmon(app);
            app.manage(AppState {
                store: Mutex::new(store), engine, perf: Mutex::new(PerfMonitor::new(perf_exe.clone())), perf_exe,
                perf_start_error: Mutex::new(None), sampler: Mutex::new(Sampler::new()), hw: Mutex::new(Telemetry::default()),
                game: Mutex::new(GameState::default()), engine_status: Mutex::new(EngineStatus::default()),
                applied: Mutex::new(false), reports: Mutex::new(Vec::new()), baseline: Mutex::new(None),
            });
            let h = app.handle().clone();
            std::thread::spawn(move || tick_loop(h));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let s = window.app_handle().state::<AppState>();
                lock(&s.perf).stop();
                do_revert(&s); // ничего временного не должно остаться
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot, update_profile, set_applied, apply_preset, create_preset, save_preset, rename_preset,
            delete_preset, export_preset, import_preset, set_settings, live, set_baseline, clear_baseline
        ])
        .build(tauri::generate_context!())
        .expect("ошибка запуска Bambuk Pro");
    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            let s = handle.state::<AppState>();
            lock(&s.perf).stop();
            do_revert(&s);
        }
    });
}
