use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER,
};

pub fn get_active_window() -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        Some(hwnd)
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}
