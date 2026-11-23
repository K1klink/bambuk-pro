use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;

struct Search { pid: u32, best: isize, area: i64 }

unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
    let s = &mut *(lp.0 as *mut Search);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == s.pid && IsWindowVisible(hwnd).as_bool() {
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_ok() {
            let a = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
            if a > s.area { s.area = a; s.best = hwnd.0 as isize; }
        }
    }
    BOOL(1)
}
/// Самое большое видимое окно процесса.
pub fn main_window(pid: u32) -> Option<isize> {
    let mut s = Search { pid, best: 0, area: 0 };
    unsafe { let _ = EnumWindows(Some(cb), LPARAM(&mut s as *mut Search as isize)); }
    if s.best != 0 && s.area > 10_000 { Some(s.best) } else { None }
}
