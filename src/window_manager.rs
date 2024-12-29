use crate::workspace::Workspace;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static HOTKEYS: Lazy<Mutex<HashMap<i32, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Registers a global hotkey for a workspace.
pub fn register_hotkey(id: i32, key_sequence: &str) -> bool {
    let mut modifiers: u32 = 0;
    let mut vk_code: Option<u32> = None;

    for part in key_sequence.split('+') {
        match part.to_lowercase().as_str() {
            "ctrl" => modifiers |= MOD_CONTROL.0,
            "alt" => modifiers |= MOD_ALT.0,
            "shift" => modifiers |= MOD_SHIFT.0,
            "win" => modifiers |= MOD_WIN.0,
            _ => {
                vk_code = virtual_key_from_string(part);
            }
        }
    }

    if let Some(vk) = vk_code {
        unsafe {
            if RegisterHotKey(None, id, HOT_KEY_MODIFIERS(modifiers), vk).is_ok() {
                let mut hotkeys = HOTKEYS.lock().unwrap();
                hotkeys.insert(id, vk as usize);
                return true;
            }
        }
    }

    eprintln!("Failed to register hotkey: {}", key_sequence);
    false
}

/// Unregisters all global hotkeys.
pub fn unregister_hotkeys() {
    unsafe {
        let mut hotkeys = HOTKEYS.lock().unwrap();
        for id in hotkeys.keys() {
            UnregisterHotKey(None, *id);
        }
        hotkeys.clear();
    }
}

/// Handles global hotkey events and toggles workspace windows.
pub fn handle_hotkey_events(workspaces: Arc<Mutex<Vec<Workspace>>>) {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                let hotkey_id = msg.wParam.0 as i32;
                let hotkeys = HOTKEYS.lock().unwrap();
                if let Some(workspace_id) = hotkeys.get(&hotkey_id) {
                    let mut workspaces = workspaces.lock().unwrap();
                    if let Some(workspace) = workspaces.get_mut(*workspace_id) {
                        toggle_workspace_windows(workspace);
                    }
                }
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Toggles the windows in a workspace between their home and target positions.
fn toggle_workspace_windows(workspace: &mut Workspace) {
    let all_at_home = workspace
        .windows
        .iter()
        .all(|w| is_window_at_position(w.id, w.home));

    for window in &workspace.windows {
        let position = if all_at_home {
            window.target
        } else {
            window.home
        };
        if let Err(e) = move_window(
            HWND(window.id as *mut std::ffi::c_void),
            position.0,
            position.1,
            position.2,
            position.3,
        ) {
            eprintln!("Error moving window {}: {}", window.title, e);
        }
    }
}

/// Moves a window to a specific position and size.
pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    unsafe {
        SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE)?;
        Ok(())
    }
}

/// Checks if a window is currently at a specific position and size.
pub fn is_window_at_position(window_id: usize, position: (i32, i32, i32, i32)) -> bool {
    if let Ok((x, y, w, h)) = get_window_position(HWND(window_id as *mut std::ffi::c_void)) {
        x == position.0 && y == position.1 && w == position.2 && h == position.3
    } else {
        false
    }
}

/// Retrieves the position and size of a window.
pub fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;
        Ok((
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ))
    }
}

/// Retrieves the currently active window and its title.
pub fn get_active_window() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            None
        } else {
            let mut buffer = [0u16; 256];
            let length = GetWindowTextW(hwnd, &mut buffer);
            let title = String::from_utf16_lossy(&buffer[..length as usize]);
            Some((hwnd, title))
        }
    }
}

pub fn listen_for_keys_with_dialog() -> Option<&'static str> {
    unsafe {
        let message = "Press Enter to confirm or Escape to cancel.";
        MessageBoxW(
            None,
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
                    None,
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

/// Converts a string to a virtual key code.
fn virtual_key_from_string(key: &str) -> Option<u32> {
    match key.to_uppercase().as_str() {
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "A" => Some(0x41),
        "B" => Some(0x42),
        "C" => Some(0x43),
        // Add more key mappings as needed.
        _ => None,
    }
}
