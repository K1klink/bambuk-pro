//! Поиск процесса ROB RUN.
use sysinfo::{ProcessesToUpdate, System};

pub const GAME_EXE: &str = "ROB RUN.exe";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameProc { pub pid: u32 }

pub fn find(sys: &mut System) -> Option<GameProc> {
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let want = GAME_EXE.to_ascii_lowercase();
    sys.processes().iter().find(|(_, p)| p.name().to_string_lossy().to_ascii_lowercase() == want)
        .map(|(pid, _)| GameProc { pid: pid.as_u32() })
}

#[cfg(windows)]
pub fn main_window(pid: u32) -> Option<isize> { crate::win::window::main_window(pid) }
#[cfg(not(windows))]
pub fn main_window(_pid: u32) -> Option<isize> { None }
