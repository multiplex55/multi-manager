use log::{info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone)]
pub struct HotkeyManager {
    registered_hotkeys: Arc<Mutex<HashMap<String, usize>>>,
}

impl HotkeyManager {
    /// Creates a new instance of `HotkeyManager`.
    pub fn new() -> Self {
        Self {
            registered_hotkeys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a global hotkey.
    pub fn register(&self, id: usize, key_sequence: &str) -> bool {
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
                if RegisterHotKey(None, id as i32, HOT_KEY_MODIFIERS(modifiers), vk).is_ok() {
                    self.registered_hotkeys
                        .lock()
                        .unwrap()
                        .insert(key_sequence.to_string(), id);
                    info!("Hotkey registered: '{}' with ID {}", key_sequence, id);
                    return true;
                } else {
                    warn!("Failed to register hotkey: '{}'", key_sequence);
                }
            }
        }

        false
    }

    /// Unregisters a global hotkey.
    pub fn unregister(&self, id: usize) {
        unsafe {
            if UnregisterHotKey(None, id as i32).is_ok() {
                let mut hotkeys = self.registered_hotkeys.lock().unwrap();
                if let Some(key) = hotkeys
                    .iter()
                    .find_map(|(k, &v)| (v == id).then(|| k.clone()))
                {
                    hotkeys.remove(&key);
                    info!("Hotkey unregistered: '{}' with ID {}", key, id);
                }
            } else {
                warn!("Failed to unregister hotkey with ID {}", id);
            }
        }
    }

    /// Checks if a hotkey is registered.
    pub fn is_registered(&self, key_sequence: &str) -> bool {
        self.registered_hotkeys
            .lock()
            .unwrap()
            .contains_key(key_sequence)
    }

    /// Returns a copy of the registered hotkeys map.
    pub fn get_registered_hotkeys(&self) -> HashMap<String, usize> {
        self.registered_hotkeys.lock().unwrap().clone()
    }
}

/// Converts a string to a virtual key code.
fn virtual_key_from_string(key: &str) -> Option<u32> {
    match key.to_uppercase().as_str() {
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        // ... add other mappings here ...
        _ => None,
    }
}
