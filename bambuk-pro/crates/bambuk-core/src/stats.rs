//! Статистика кадров: FPS, 1% Low, frame time. Только реальные значения от PresentMon.
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameStats {
    pub fps: f32,
    pub one_percent_low: f32,
    pub avg_frame_ms: f32,
    pub last_frame_ms: f32,
    pub p99_frame_ms: f32,
    pub frames: usize,
}

#[derive(Default)]
pub struct FrameWindow {
    frames: VecDeque<f32>, // миллисекунды между Present
    total_ms: f64,
}

const KEEP_MS: f64 = 30_000.0;
const FPS_WINDOW_MS: f64 = 1_000.0;
const LOW_WINDOW_MS: f64 = 10_000.0;

impl FrameWindow {
    pub fn new() -> Self { Self::default() }
    pub fn clear(&mut self) { self.frames.clear(); self.total_ms = 0.0; }
    pub fn push(&mut self, ms: f32) {
        if !ms.is_finite() || ms <= 0.0 || ms > 5000.0 { return; }
        self.frames.push_back(ms);
        self.total_ms += ms as f64;
        while self.total_ms > KEEP_MS && self.frames.len() > 1 {
            if let Some(f) = self.frames.pop_front() { self.total_ms -= f as f64; }
        }
    }
    fn tail(&self, window_ms: f64) -> Vec<f32> {
        let mut acc = 0.0; let mut out = Vec::new();
        for &f in self.frames.iter().rev() {
            out.push(f); acc += f as f64;
            if acc >= window_ms { break; }
        }
        out
    }
    pub fn snapshot(&self) -> FrameStats {
        if self.frames.is_empty() { return FrameStats::default(); }
        let one = self.tail(FPS_WINDOW_MS);
        let sum: f64 = one.iter().map(|&x| x as f64).sum();
        let fps = if sum > 0.0 { (one.len() as f64 * 1000.0 / sum) as f32 } else { 0.0 };
        let mut w = self.tail(LOW_WINDOW_MS);
        w.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // по убыванию
        let n = w.len();
        let worst = (n / 100).max(1);
        let worst_avg: f64 = w[..worst].iter().map(|&x| x as f64).sum::<f64>() / worst as f64;
        let p99_idx = (n as f64 * 0.01).floor() as usize;
        let avg: f64 = w.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
        FrameStats {
            fps,
            one_percent_low: if worst_avg > 0.0 { (1000.0 / worst_avg) as f32 } else { 0.0 },
            avg_frame_ms: avg as f32,
            last_frame_ms: *self.frames.back().unwrap_or(&0.0),
            p99_frame_ms: w[p99_idx.min(n - 1)],
            frames: n,
        }
    }
}

/// Разбор строки CSV PresentMon. Заголовок определяет колонку времени кадра:
/// `FrameTime` (PresentMon 2.x) или `MsBetweenPresents` (1.x).
#[derive(Default)]
pub struct CsvParser { col: Option<usize>, app_col: Option<usize> }
impl CsvParser {
    pub fn feed(&mut self, line: &str) -> Option<f32> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() { return None; }
        let cells: Vec<&str> = line.split(',').collect();
        if self.col.is_none() {
            let pos = cells.iter().position(|c| *c == "FrameTime").or_else(|| cells.iter().position(|c| *c == "MsBetweenPresents"));
            if let Some(p) = pos {
                self.col = Some(p);
                self.app_col = cells.iter().position(|c| *c == "Application" || *c == "ProcessName");
                return None;
            }
            return None;
        }
        if cells.iter().any(|c| *c == "FrameTime" || *c == "MsBetweenPresents") { return None; }
        cells.get(self.col?)?.trim().parse::<f32>().ok()
    }
    pub fn ready(&self) -> bool { self.col.is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steady_60fps() {
        let mut w = FrameWindow::new();
        for _ in 0..600 { w.push(16.6667); }
        let s = w.snapshot();
        assert!((s.fps - 60.0).abs() < 0.2, "{:?}", s);
        assert!((s.one_percent_low - 60.0).abs() < 0.5);
        assert!((s.avg_frame_ms - 16.6667).abs() < 0.01);
    }
    #[test]
    fn one_percent_low_reflects_stutters() {
        let mut w = FrameWindow::new();
        for i in 0..1000 { w.push(if i % 100 == 0 { 50.0 } else { 10.0 }); }
        let s = w.snapshot();
        assert!(s.one_percent_low < 25.0 && s.one_percent_low > 15.0, "{:?}", s);
        assert!(s.fps > 80.0);
    }
    #[test]
    fn rejects_garbage_and_old_frames_expire() {
        let mut w = FrameWindow::new();
        w.push(f32::NAN); w.push(-1.0); w.push(0.0); w.push(99999.0);
        assert_eq!(w.snapshot(), FrameStats::default());
        for _ in 0..10_000 { w.push(10.0); }
        assert!(w.snapshot().frames <= 1000 && w.total_ms <= KEEP_MS + 10.0);
    }
    #[test]
    fn csv_v2_and_v1_headers() {
        let mut p = CsvParser::default();
        assert_eq!(p.feed("Application,ProcessID,SwapChainAddress,PresentRuntime,FrameTime,CPUBusy"), None);
        assert_eq!(p.feed("ROB RUN.exe,1234,0x1,DXGI,16.5,2.1\r\n"), Some(16.5));
        let mut p = CsvParser::default();
        p.feed("Application,ProcessID,MsBetweenPresents,MsInPresentAPI");
        assert_eq!(p.feed("ROB RUN.exe,1,8.25,0.1"), Some(8.25));
        assert_eq!(p.feed("ROB RUN.exe,1,abc,0.1"), None);
    }
    #[test]
    fn csv_ignores_lines_before_header() {
        let mut p = CsvParser::default();
        assert_eq!(p.feed("PresentMon 2.4.0"), None);
        assert!(!p.ready());
    }
}
