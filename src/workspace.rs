use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub hotkey: Option<String>,
    pub windows: Vec<Window>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: usize,
    pub title: String,
    pub home: (i32, i32, i32, i32),
    pub target: (i32, i32, i32, i32),
}

/// Saves workspaces to a JSON file.
pub fn save_workspaces(workspaces: &[Workspace], file_path: &str) {
    match serde_json::to_string(workspaces) {
        Ok(json) => match File::create(file_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(json.as_bytes()) {
                    error!("Failed to write workspaces to file '{}': {}", file_path, e);
                } else {
                    info!("Workspaces successfully saved to '{}'.", file_path);
                }
            }
            Err(e) => error!("Failed to create file '{}': {}", file_path, e),
        },
        Err(e) => error!("Failed to serialize workspaces: {}", e),
    }
}

/// Loads workspaces from a JSON file.
pub fn load_workspaces(file_path: &str) -> Vec<Workspace> {
    let mut content = String::new();
    match File::open(file_path) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut content) {
                error!("Failed to read file '{}': {}", file_path, e);
                return Vec::new();
            }

            match serde_json::from_str(&content) {
                Ok(workspaces) => {
                    info!("Successfully loaded workspaces from '{}'.", file_path);
                    workspaces
                }
                Err(e) => {
                    error!(
                        "Failed to deserialize workspaces from '{}': {}",
                        file_path, e
                    );
                    Vec::new()
                }
            }
        }
        Err(_) => {
            warn!(
                "File '{}' not found. Returning an empty workspace list.",
                file_path
            );
            Vec::new()
        }
    }
}
