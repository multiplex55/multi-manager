use crate::workspace::Workspace;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Static hotkeys map
static HOTKEYS: Lazy<Mutex<HashMap<i32, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));
// Static map to track pressed hotkeys
static HOTKEY_PRESSED: Lazy<Mutex<Option<i32>>> = Lazy::new(|| Mutex::new(None));

/// Checks if a hotkey is pressed based on the key sequence string
pub fn is_hotkey_pressed(key_sequence: &str) -> bool {
    let mut modifiers_pressed = true;
    let mut vk_code: Option<u32> = None;

    for part in key_sequence.split('+') {
        match part.to_lowercase().as_str() {
            "ctrl" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
            },
            "alt" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_MENU.0 as i32) < 0;
            },
            "shift" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
            },
            "win" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_LWIN.0 as i32) < 0
                    || GetAsyncKeyState(VK_RWIN.0 as i32) < 0;
            },
            _ => vk_code = virtual_key_from_string(part),
        }
    }

    if let Some(vk) = vk_code {
        unsafe { modifiers_pressed && GetAsyncKeyState(vk as i32) < 0 }
    } else {
        false
    }
}

// Registers a global hotkey for a workspace
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
                hotkeys.insert(id, id as usize);
                info!("Registered hotkey '{}' with ID {}.", key_sequence, id);
                return true;
            } else {
                error!("Failed to register hotkey: '{}'.", key_sequence);
            }
        }
    } else {
        warn!("Invalid hotkey sequence: '{}'.", key_sequence);
    }

    false
}

// Unregisters a global hotkey
pub fn unregister_hotkey(id: i32) {
    unsafe {
        if let Ok(_) = UnregisterHotKey(None, id) {
            let mut hotkeys = HOTKEYS.lock().unwrap();
            hotkeys.remove(&id);
            info!("Unregistered hotkey with ID {}.", id);
        } else {
            warn!("Failed to unregister hotkey with ID {}.", id);
        }
    }
}

// Handles global hotkey events and toggles workspace windows
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
                        info!("Activated workspace: '{}'.", workspace.name);
                    } else {
                        warn!("Workspace ID '{}' not found.", workspace_id);
                    }
                } else {
                    warn!("Hotkey ID '{}' not registered.", hotkey_id);
                }
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

// Listens for low-level keyboard events
pub fn listen_for_keyboard_event(key_sequence: &str) -> Option<()> {
    info!(
        "Initializing listen_for_keyboard_event with key sequence: '{}'",
        key_sequence
    );

    // Parse the key sequence into modifiers and virtual key code
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
                match vk_code {
                    Some(code) => info!("Detected virtual key: '{}' -> code {}", part, code),
                    None => warn!("Failed to parse virtual key from part: '{}'", part),
                }
            }
        }
    }

    if vk_code.is_none() {
        error!("No valid virtual key code detected. Aborting listener setup.");
        return None;
    }

    let vk = vk_code.unwrap();
    info!(
        "Final parsed key sequence: modifiers={:#X}, vk_code={:#X}",
        modifiers, vk
    );

    unsafe {
        // Register the hotkey
        // Register the hotkey
        if RegisterHotKey(None, 1, HOT_KEY_MODIFIERS(modifiers), vk).is_err() {
            error!(
                "Failed to register hotkey for key sequence '{}'.",
                key_sequence
            );
            return None;
        }
        info!("Hotkey successfully registered for '{}'.", key_sequence);

        let mut msg = MSG::default();
        info!(
            "Entering message loop to listen for key sequence: '{}'",
            key_sequence
        );

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                info!("WM_HOTKEY message received: msg.lParam={:#X}", msg.lParam.0);

                MessageBoxW(
                    None,
                    PCWSTR(
                        "Keyboard sequence detected!"
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    PCWSTR(
                        "Hotkey Triggered"
                            .encode_utf16()
                            .chain(Some(0))
                            .collect::<Vec<_>>()
                            .as_ptr(),
                    ),
                    MB_OK | MB_ICONINFORMATION,
                );

                // Unregister the hotkey and return after detecting
                UnregisterHotKey(None, 1);
                info!("Hotkey unregistered after triggering.");
                return Some(());
            }
        }

        error!("Exited message loop unexpectedly.");
    }

    None
}

// Toggles workspace windows between home and target
pub fn toggle_workspace_windows(workspace: &mut Workspace) {
    let all_at_home = workspace.windows.iter().all(|w| {
        is_window_at_position(
            HWND(w.id as *mut std::ffi::c_void),
            w.home.0,
            w.home.1,
            w.home.2,
            w.home.3,
        )
    });

    for window in &workspace.windows {
        let target_position = if all_at_home {
            window.target
        } else {
            window.home
        };
        if let Err(e) = move_window(
            HWND(window.id as *mut std::ffi::c_void),
            target_position.0,
            target_position.1,
            target_position.2,
            target_position.3,
        ) {
            warn!("Failed to move window '{}': {}", window.title, e);
        }
    }
}

// Checks if a window is at the specified position
fn is_window_at_position(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> bool {
    if let Ok((wx, wy, ww, wh)) = get_window_position(hwnd) {
        wx == x && wy == y && ww == w && wh == h
    } else {
        false
    }
}

// Retrieves the current position and size of a window
fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Ok((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        } else {
            Err(windows::core::Error::from_win32())
        }
    }
}

// Displays a message box with the workspace name
fn display_message_box(workspace_name: &str) {
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(
                format!("Workspace triggered: {}", workspace_name)
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Workspace Hotkey"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

// Converts a string to a virtual key code
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
        _ => None,
    }
}

// Retrieves the currently active window and its title
pub fn get_active_window() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            warn!("No active window detected.");
            None
        } else {
            let mut buffer = [0u16; 256];
            let length = GetWindowTextW(hwnd, &mut buffer);
            let title = String::from_utf16_lossy(&buffer[..length as usize]);
            info!("Active window detected: '{}'.", title);
            Some((hwnd, title))
        }
    }
}

// Moves a window to a specific position and size
pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    unsafe {
        SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER)?;
        info!(
            "Moved window (HWND: {:?}) to position ({}, {}, {}, {}).",
            hwnd.0, x, y, w, h
        );
        Ok(())
    }
}

// Listens for key input to confirm or cancel an action
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
                info!("Enter key detected.");
                return Some("Enter");
            }
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 {
                warn!("Escape key detected.");
                return Some("Esc");
            }
        }
    }
}

// Retrieves the last pressed hotkey, resetting the state
pub fn get_last_pressed_hotkey() -> Option<i32> {
    let mut hotkey_pressed = HOTKEY_PRESSED.lock().unwrap();
    let result = *hotkey_pressed;
    *hotkey_pressed = None; // Reset the state
    result
}
