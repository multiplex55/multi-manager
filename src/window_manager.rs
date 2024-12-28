use std::ptr;
use std::sync::{Arc, Mutex};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, VK_ESCAPE, VK_RETURN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowTextW, MessageBoxW, PeekMessageW, SetWindowPos,
    HWND_TOP, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MSG, PEEK_MESSAGE_REMOVE_TYPE,
    SWP_NOACTIVATE, SWP_NOZORDER, WM_HOTKEY,
};

use crate::gui::App;

pub fn get_active_window() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut buffer = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buffer);
        let title = String::from_utf16_lossy(&buffer[..len as usize]);

        Some((hwnd, title))
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn listen_for_keys_with_dialog() -> Option<&'static str> {
    unsafe {
        let message = "Press Enter to confirm or Escape to cancel.";
        MessageBoxW(
            HWND(ptr::null_mut()),
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Action Required"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );

        loop {
            if GetAsyncKeyState(VK_RETURN.0 as i32) < 0 {
                return Some("Enter");
            }
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 {
                MessageBoxW(
                    HWND(ptr::null_mut()),
                    PCWSTR(
                        "Action canceled by user."
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    PCWSTR(
                        "Canceled"
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    MB_OK | MB_ICONWARNING,
                );
                return Some("Esc");
            }
        }
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
            Err("Failed to get window position")
        }
    }
}

pub fn capture_hotkey_dialog() -> Option<String> {
    unsafe {
        let mut hotkey = String::new();
        MessageBoxW(
            HWND(ptr::null_mut()),
            PCWSTR(
                "Press keys for the hotkey (simulation only)."
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Set Hotkey"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );
        hotkey.push_str("Ctrl+Shift+H"); // Placeholder
        Some(hotkey)
    }
}

pub fn register_hotkey_listener(app: Arc<Mutex<App>>) {
    std::thread::spawn(move || {
        loop {
            let mut msg = MSG::default();
            unsafe {
                if PeekMessageW(&mut msg, HWND::default(), 0, 0, PEEK_MESSAGE_REMOVE_TYPE(1))
                    .as_bool()
                    && msg.message == WM_HOTKEY
                {
                    let workspace_id = msg.wParam.0 as usize; // Assuming workspace ID is stored as wParam
                    if let Ok(mut app) = app.lock() {
                        if let Some(workspace) = app.workspaces.get(workspace_id) {
                            for window in &workspace.windows {
                                let hwnd = HWND(window.id as *mut _);
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
        }
    });
}

pub fn register_hotkey(workspace_id: usize, hotkey: &str) -> Result<(), &'static str> {
    let modifiers = HOT_KEY_MODIFIERS(0); // Replace with actual modifier logic (e.g., MOD_CONTROL | MOD_SHIFT)
    let key_code = 0x48; // Replace with actual key code logic (e.g., 'H' key)
    unsafe {
        if RegisterHotKey(HWND::default(), workspace_id as i32, modifiers, key_code).is_ok() {
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
