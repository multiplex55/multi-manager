use crate::window_manager::register_hotkey;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub hotkey: Option<String>,
    pub windows: Vec<Window>,
}

impl Workspace {
    /// Sets the hotkey for the workspace after validation.
    /// Returns `Ok(())` if the hotkey is valid and assigned, or an `Err(String)` with an error message if it fails.
    pub fn set_hotkey(&mut self, hotkey: &str) -> Result<(), String> {
        if is_valid_key_combo(hotkey) {
            self.hotkey = Some(hotkey.to_string());
            Ok(())
        } else {
            Err(format!("Invalid hotkey: '{}'", hotkey))
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: usize,
    pub title: String,
    pub home: (i32, i32, i32, i32),
    pub target: (i32, i32, i32, i32),
}

fn is_valid_key_combo(input: &str) -> bool {
    let pattern = r"^(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:F(?:[1-9]|1[0-2]|1[3-9]|2[0-4])|[A-Z]|[0-9]|NUMPAD[0-9]|NUMPAD(?:MULTIPLY|ADD|SEPARATOR|SUBTRACT|DOT|DIVIDE)|UP|DOWN|LEFT|RIGHT|BACKSPACE|TAB|ENTER|PAUSE|CAPSLOCK|ESCAPE|SPACE|PAGEUP|PAGEDOWN|END|HOME|INSERT|DELETE|OEM_(?:PLUS|COMMA|MINUS|PERIOD|[1-7])|PRINTSCREEN|SCROLLLOCK|NUMLOCK|LEFT(?:SHIFT|CTRL|ALT)|RIGHT(?:SHIFT|CTRL|ALT))$";
    let re = Regex::new(pattern).unwrap();
    re.is_match(input)
}

/// Saves the current workspaces to a file in JSON format.
pub fn save_workspaces(workspaces: &[Workspace], file_path: &str) {
    match serde_json::to_string(workspaces) {
        Ok(json) => {
            if let Err(e) =
                File::create(file_path).and_then(|mut file| file.write_all(json.as_bytes()))
            {
                error!("Failed to save workspaces to '{}': {}", file_path, e);
            } else {
                info!("Workspaces successfully saved to '{}'.", file_path);
            }
        }
        Err(e) => {
            error!("Failed to serialize workspaces: {}", e);
        }
    }
}

/// Loads workspaces from a JSON file. Returns an empty vector if the file does not exist or cannot be read.
pub fn load_workspaces(file_path: &str) -> Vec<Workspace> {
    let mut content = String::new();
    match File::open(file_path) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut content) {
                error!("Failed to read file '{}': {}", file_path, e);
                return Vec::new();
            }
            match serde_json::from_str::<Vec<Workspace>>(&content) {
                Ok(mut workspaces) => {
                    info!("Successfully loaded workspaces from '{}'.", file_path);

                    for (i, workspace) in workspaces.iter_mut().enumerate() {
                        if let Some(ref hotkey) = workspace.hotkey {
                            if !register_hotkey(i as i32, hotkey) {
                                warn!(
                                    "Failed to register hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            } else {
                                info!(
                                    "Registered hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            }
                        }
                    }

                    workspaces
                }
                Err(e) => {
                    warn!(
                        "Failed to parse JSON in '{}': {}. Returning empty workspace list.",
                        file_path, e
                    );
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!(
                "File '{}' not found or cannot be opened: {}. Returning empty workspace list.",
                file_path, e
            );
            Vec::new()
        }
    }
}
