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

pub fn save_workspaces(workspaces: &[Workspace], file_path: &str) {
    let json = serde_json::to_string(workspaces).expect("Failed to serialize workspaces");
    File::create(file_path)
        .and_then(|mut file| file.write_all(json.as_bytes()))
        .expect("Failed to save workspaces to file");
}

pub fn load_workspaces(file_path: &str) -> Vec<Workspace> {
    let mut content = String::new();
    if let Ok(mut file) = File::open(file_path) {
        file.read_to_string(&mut content)
            .expect("Failed to read file");
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    }
}
