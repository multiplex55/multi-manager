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
        // Function keys
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "F4" => Some(0x73),
        "F5" => Some(0x74),
        "F6" => Some(0x75),
        "F7" => Some(0x76),
        "F8" => Some(0x77),
        "F9" => Some(0x78),
        "F10" => Some(0x79),
        "F11" => Some(0x7A),
        "F12" => Some(0x7B),
        "F13" => Some(0x7C),
        "F14" => Some(0x7D),
        "F15" => Some(0x7E),
        "F16" => Some(0x7F),
        "F17" => Some(0x80),
        "F18" => Some(0x81),
        "F19" => Some(0x82),
        "F20" => Some(0x83),
        "F21" => Some(0x84),
        "F22" => Some(0x85),
        "F23" => Some(0x86),
        "F24" => Some(0x87),

        // Alphabet keys
        "A" => Some(0x41),
        "B" => Some(0x42),
        "C" => Some(0x43),
        "D" => Some(0x44),
        "E" => Some(0x45),
        "F" => Some(0x46),
        "G" => Some(0x47),
        "H" => Some(0x48),
        "I" => Some(0x49),
        "J" => Some(0x4A),
        "K" => Some(0x4B),
        "L" => Some(0x4C),
        "M" => Some(0x4D),
        "N" => Some(0x4E),
        "O" => Some(0x4F),
        "P" => Some(0x50),
        "Q" => Some(0x51),
        "R" => Some(0x52),
        "S" => Some(0x53),
        "T" => Some(0x54),
        "U" => Some(0x55),
        "V" => Some(0x56),
        "W" => Some(0x57),
        "X" => Some(0x58),
        "Y" => Some(0x59),
        "Z" => Some(0x5A),

        // Numeric keys
        "0" => Some(0x30),
        "1" => Some(0x31),
        "2" => Some(0x32),
        "3" => Some(0x33),
        "4" => Some(0x34),
        "5" => Some(0x35),
        "6" => Some(0x36),
        "7" => Some(0x37),
        "8" => Some(0x38),
        "9" => Some(0x39),

        // Numpad keys
        "NUMPAD0" => Some(0x60),
        "NUMPAD1" => Some(0x61),
        "NUMPAD2" => Some(0x62),
        "NUMPAD3" => Some(0x63),
        "NUMPAD4" => Some(0x64),
        "NUMPAD5" => Some(0x65),
        "NUMPAD6" => Some(0x66),
        "NUMPAD7" => Some(0x67),
        "NUMPAD8" => Some(0x68),
        "NUMPAD9" => Some(0x69),
        "NUMPAD_ADD" => Some(0x6B),
        "NUMPAD_SUBTRACT" => Some(0x6D),
        "NUMPAD_MULTIPLY" => Some(0x6A),
        "NUMPAD_DIVIDE" => Some(0x6F),
        "NUMPAD_DECIMAL" => Some(0x6E),

        // Special keys
        "ESCAPE" => Some(0x1B),
        "ENTER" => Some(0x0D),
        "TAB" => Some(0x09),
        "SPACE" => Some(0x20),
        "BACKSPACE" => Some(0x08),
        "DELETE" => Some(0x2E),
        "INSERT" => Some(0x2D),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGE_UP" => Some(0x21),
        "PAGE_DOWN" => Some(0x22),
        "ARROW_UP" => Some(0x26),
        "ARROW_DOWN" => Some(0x28),
        "ARROW_LEFT" => Some(0x25),
        "ARROW_RIGHT" => Some(0x27),

        // Symbols
        "GRAVE" => Some(0xC0),         // `
        "MINUS" => Some(0xBD),         // -
        "EQUALS" => Some(0xBB),        // =
        "LEFT_BRACKET" => Some(0xDB),  // [
        "RIGHT_BRACKET" => Some(0xDD), // ]
        "BACKSLASH" => Some(0xDC),     // \
        "SEMICOLON" => Some(0xBA),     // ;
        "APOSTROPHE" => Some(0xDE),    // '
        "COMMA" => Some(0xBC),         // ,
        "PERIOD" => Some(0xBE),        // .
        "SLASH" => Some(0xBF),         // /

        _ => None,
    }
}
