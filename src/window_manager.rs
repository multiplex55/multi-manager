use crate::workspace::Workspace;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Static hotkeys map
static HOTKEYS: Lazy<Mutex<HashMap<i32, usize>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Checks if a hotkey is pressed based on the key sequence string.
///
/// # Arguments
/// - `key_sequence`: The key sequence string (e.g., "Ctrl+Alt+H") to check.
///
/// # Returns
/// - `true` if the hotkey is currently pressed.
/// - `false` otherwise.
///
/// # Example
/// ```
/// if is_hotkey_pressed("Ctrl+Shift+P") {
///     println!("Hotkey pressed!");
/// }
/// ```
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

/// Registers a global hotkey for a workspace.
///
/// # Arguments
/// - `id`: The unique identifier for the hotkey.
/// - `key_sequence`: The key sequence string (e.g., "Ctrl+Alt+H") to register.
///
/// # Returns
/// - `true` if the hotkey was successfully registered.
/// - `false` otherwise.
///
/// # Example
/// ```
/// if register_hotkey(1, "Ctrl+Shift+P") {
///     println!("Hotkey registered!");
/// }
/// ```
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

/// Unregisters a global hotkey based on its ID.
///
/// # Arguments
/// - `id`: The unique identifier of the hotkey to unregister.
///
/// # Example
/// ```
/// unregister_hotkey(1);
/// ```
pub fn unregister_hotkey(id: i32) {
    unsafe {
        if UnregisterHotKey(None, id).is_ok() {
            info!("Successfully unregistered hotkey with ID {}.", id);
            // Remove the hotkey from the HOTKEYS map
            let mut hotkeys = HOTKEYS.lock().unwrap();
            hotkeys.remove(&id);
        } else {
            warn!("Failed to unregister hotkey with ID {}.", id);
        }
    }
}

/// Toggles workspace windows between their home and target locations.
///
/// # Arguments
/// - `workspace`: The workspace to toggle windows for.
///
/// - If all windows are at their home positions, they are moved to their target positions.
/// - If any window is not at its home or target position, it is moved to its home position.
///
/// # Example
/// ```
/// toggle_workspace_windows(&mut workspace);
/// ```
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

        // Move the window
        if let Err(e) = move_window(
            HWND(window.id as *mut std::ffi::c_void),
            target_position.0,
            target_position.1,
            target_position.2,
            target_position.3,
        ) {
            warn!("Failed to move window '{}': {}", window.title, e);
        } else {
            info!(
                "Moved window '{}' to position: {:?}",
                window.title, target_position
            );
        }

        // Activate the window
        unsafe {
            let hwnd = HWND(window.id as *mut std::ffi::c_void);
            if SetForegroundWindow(hwnd).as_bool() {
                info!("Activated window '{}'", window.title);
            } else {
                warn!("Failed to activate window '{}'", window.title);
            }
        }
    }
}

/// Checks if a window is at the specified position.
///
/// # Arguments
/// - `hwnd`: The handle to the window.
/// - `x`, `y`: The top-left coordinates of the position.
/// - `w`, `h`: The width and height of the position.
///
/// # Returns
/// - `true` if the window matches the specified position.
/// - `false` otherwise.
///
/// # Example
/// ```
/// if is_window_at_position(hwnd, 0, 0, 800, 600) {
///     println!("Window is at the correct position.");
/// }
/// ```
fn is_window_at_position(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> bool {
    if let Ok((wx, wy, ww, wh)) = get_window_position(hwnd) {
        wx == x && wy == y && ww == w && wh == h
    } else {
        false
    }
}

/// Retrieves the current position and size of a window.
///
/// # Arguments
/// - `hwnd`: The handle to the window.
///
/// # Returns
/// - A tuple `(x, y, width, height)` representing the window's position and size.
/// - `Err` if the window position cannot be retrieved.
///
/// # Example
/// ```
/// if let Ok((x, y, w, h)) = get_window_position(hwnd) {
///     println!("Window position: ({}, {}, {}, {})", x, y, w, h);
/// }
/// ```
pub fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32)> {
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

/// Converts a string to a virtual key code.
///
/// # Arguments
/// - `key`: The key string (e.g., "A", "F1", "Ctrl").
///
/// # Returns
/// - The virtual key code as `Option<u32>`.
///
/// # Example
/// ```
/// if let Some(vk) = virtual_key_from_string("Ctrl") {
///     println!("Virtual key code: {}", vk);
/// }
/// ```
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

        // Number keys
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
        "NUMPADMULTIPLY" => Some(0x6A),
        "NUMPADADD" => Some(0x6B),
        "NUMPADSEPARATOR" => Some(0x6C),
        "NUMPADSUBTRACT" => Some(0x6D),
        "NUMPADDOT" => Some(0x6E),
        "NUMPADDIVIDE" => Some(0x6F),

        // Arrow keys
        "UP" => Some(0x26),
        "DOWN" => Some(0x28),
        "LEFT" => Some(0x25),
        "RIGHT" => Some(0x27),

        // Special keys
        "BACKSPACE" => Some(0x08),
        "TAB" => Some(0x09),
        "ENTER" => Some(0x0D),
        "SHIFT" => Some(0x10),
        "CTRL" => Some(0x11),
        "ALT" => Some(0x12),
        "PAUSE" => Some(0x13),
        "CAPSLOCK" => Some(0x14),
        "ESCAPE" => Some(0x1B),
        "SPACE" => Some(0x20),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "END" => Some(0x23),
        "HOME" => Some(0x24),
        "INSERT" => Some(0x2D),
        "DELETE" => Some(0x2E),

        // Symbols
        "OEM_PLUS" => Some(0xBB),   // '+' key
        "OEM_COMMA" => Some(0xBC),  // ',' key
        "OEM_MINUS" => Some(0xBD),  // '-' key
        "OEM_PERIOD" => Some(0xBE), // '.' key
        "OEM_1" => Some(0xBA),      // ';:' key
        "OEM_2" => Some(0xBF),      // '/?' key
        "OEM_3" => Some(0xC0),      // '`~' key
        "OEM_4" => Some(0xDB),      // '[{' key
        "OEM_5" => Some(0xDC),      // '\|' key
        "OEM_6" => Some(0xDD),      // ']}' key
        "OEM_7" => Some(0xDE),      // ''"' key

        // Additional keys
        "PRINTSCREEN" => Some(0x2C),
        "SCROLLLOCK" => Some(0x91),
        "NUMLOCK" => Some(0x90),
        "LEFTSHIFT" => Some(0xA0),
        "RIGHTSHIFT" => Some(0xA1),
        "LEFTCTRL" => Some(0xA2),
        "RIGHTCTRL" => Some(0xA3),
        "LEFTALT" => Some(0xA4),
        "RIGHTALT" => Some(0xA5),

        _ => None,
    }
}

/// Retrieves the currently active window and its title.
///
/// # Returns
/// - A tuple `(HWND, String)` representing the active window's handle and title.
/// - `None` if no active window is found.
///
/// # Example
/// ```
/// if let Some((hwnd, title)) = get_active_window() {
///     println!("Active window: {} ({:?})", title, hwnd);
/// }
/// ```
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

/// Moves a window to a specific position and size.
///
/// # Arguments
/// - `hwnd`: The handle to the window.
/// - `x`, `y`: The new top-left position of the window.
/// - `w`, `h`: The new width and height of the window.
///
/// # Returns
/// - `Ok(())` if the window was successfully moved.
/// - `Err` otherwise.
///
/// # Example
/// ```
/// if let Err(e) = move_window(hwnd, 100, 100, 800, 600) {
///     println!("Failed to move window: {}", e);
/// }
/// ```
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

/// Listens for key input to confirm or cancel an action.
///
/// # Returns
/// - `"Enter"` if the Enter key is pressed.
/// - `"Esc"` if the Escape key is pressed.
///
/// # Example
/// ```
/// if let Some(action) = listen_for_keys_with_dialog() {
///     println!("User selected action: {}", action);
/// }
/// ```
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
