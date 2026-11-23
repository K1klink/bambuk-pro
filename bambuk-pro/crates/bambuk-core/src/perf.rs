//! Реальные FPS/frame time: PresentMon (ETW), без инъекции в игру. Требует права администратора.
use crate::stats::{CsvParser, FrameStats, FrameWindow};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerfStatus { pub running: bool, pub receiving: bool, pub note: Option<String> }

struct Shared { win: FrameWindow, last: Option<Instant>, note: Option<String>, alive: bool }

pub struct PerfMonitor { exe: PathBuf, child: Option<Child>, shared: Arc<Mutex<Shared>> }

impl PerfMonitor {
    pub fn new(exe: PathBuf) -> Self {
        Self { exe, child: None, shared: Arc::new(Mutex::new(Shared { win: FrameWindow::new(), last: None, note: None, alive: false })) }
    }
    pub fn is_running(&self) -> bool { self.child.is_some() && self.shared.lock().unwrap().alive }

    pub fn start(&mut self, process_name: &str) -> Result<(), String> {
        self.stop();
        if !self.exe.exists() { return Err(format!("PresentMon.exe не найден: {}", self.exe.display())); }
        let mut cmd = Command::new(&self.exe);
        cmd.args(["--process_name", process_name, "--output_stdout", "--no_console_stats", "--stop_existing_session", "--session_name", "BambukPro"])
            .stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        #[cfg(windows)]
        { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0800_0000); } // CREATE_NO_WINDOW
        let mut child = cmd.spawn().map_err(|e| format!("Не удалось запустить PresentMon: {e}"))?;
        {
            let mut s = self.shared.lock().unwrap();
            s.win.clear(); s.last = None; s.note = None; s.alive = true;
        }
        let out = child.stdout.take().ok_or("нет stdout")?;
        let err = child.stderr.take().ok_or("нет stderr")?;
        let sh = self.shared.clone();
        std::thread::spawn(move || {
            let mut parser = CsvParser::default();
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(ms) = parser.feed(&line) {
                    let mut s = sh.lock().unwrap(); s.win.push(ms); s.last = Some(Instant::now());
                }
            }
            sh.lock().unwrap().alive = false;
        });
        let sh = self.shared.clone();
        std::thread::spawn(move || {
            let text: Vec<String> = BufReader::new(err).lines().map_while(Result::ok).take(5).collect();
            let joined = text.join(" ").to_lowercase();
            if joined.contains("administrator") || joined.contains("access") || joined.contains("elevat") {
                sh.lock().unwrap().note = Some("PresentMon требует запуска Bambuk Pro от имени администратора".into());
            } else if !text.is_empty() {
                sh.lock().unwrap().note = Some(text.join(" "));
            }
        });
        self.child = Some(child);
        Ok(())
    }
    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() { let _ = c.kill(); let _ = c.wait(); }
        let mut s = self.shared.lock().unwrap(); s.alive = false; s.win.clear(); s.last = None;
    }
    /// Нет свежих кадров > 2 секунд → считаем, что FPS нет (игра на паузе/свёрнута).
    pub fn stats(&self) -> FrameStats {
        let s = self.shared.lock().unwrap();
        match s.last { Some(t) if t.elapsed().as_secs_f32() < 2.0 => s.win.snapshot(), _ => FrameStats::default() }
    }
    pub fn status(&self) -> PerfStatus {
        let s = self.shared.lock().unwrap();
        PerfStatus { running: s.alive, receiving: s.last.map(|t| t.elapsed().as_secs_f32() < 2.0).unwrap_or(false), note: s.note.clone() }
    }
}
impl Drop for PerfMonitor { fn drop(&mut self) { self.stop(); } }
