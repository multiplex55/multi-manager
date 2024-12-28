use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_SHIFT, VK_H,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, PeekMessageW, SetWindowPos, HWND_TOP, MSG,
    PEEK_MESSAGE_REMOVE_TYPE, SWP_NOACTIVATE, SWP_NOZORDER, WM_HOTKEY,
};

use crate::gui::App;

pub fn get_active_window() -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32), &'static str> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let x = rect.left;
            let y = rect.top;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            Ok((x, y, w, h))
        } else {
            Err("Failed to retrieve window position.")
        }
    }
}

pub fn capture_hotkey_dialog() -> Option<String> {
    println!("Press Ctrl+Shift+H to register hotkey.");
    Some("Ctrl+Shift+H".to_string()) // Placeholder
}

pub fn register_hotkey(workspace_id: usize, hotkey: &str) -> Result<(), &'static str> {
    let modifiers = MOD_CONTROL | MOD_SHIFT; // Use the modifiers directly
    let key = VK_H.0.into(); // Convert VK_H to the expected type

    unsafe {
        if RegisterHotKey(HWND::default(), workspace_id as i32, modifiers, key).is_ok() {
            Ok(())
        } else {
            Err("Failed to register hotkey")
        }
    }
}

pub fn unregister_hotkey(workspace_id: usize) {
    unsafe {
        UnregisterHotKey(HWND::default(), workspace_id as i32);
    }
}

pub fn register_hotkey_listener(app: Arc<Mutex<App>>) {
    std::thread::spawn(move || {
        let mut msg = MSG::default();

        loop {
            unsafe {
                if PeekMessageW(&mut msg, HWND::default(), 0, 0, PEEK_MESSAGE_REMOVE_TYPE(1))
                    .as_bool()
                    && msg.message == WM_HOTKEY
                {
                    let workspace_id = msg.wParam.0 as usize;

                    let app = app.lock().unwrap();
                    if let Some(workspace) = app.workspaces.get(workspace_id) {
                        for window in &workspace.windows {
                            let hwnd = HWND(window.id as *mut std::ffi::c_void);
                            move_window(
                                hwnd,
                                window.target.0,
                                window.target.1,
                                window.target.2,
                                window.target.3,
                            );
                        }
                    }
                }
            }
        }
    });
}
