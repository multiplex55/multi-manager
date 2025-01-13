#![windows_subsystem = "windows"]

mod gui;
mod hotkey_manager;
mod utils;
mod window_manager;
mod workspace;

use hotkey_manager::HotkeyManager;
use log::info;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write; // Fix for write_all error
use std::sync::{Arc, Mutex};

fn main() {
    // Ensure logging is initialized
    ensure_logging_initialized();

    // Backtrace for Debug
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");
    let hotkey_manager = Arc::new(HotkeyManager::new());

    // Initialize the application states
    let app = gui::App {
        app_title_name: "Multi Manager".to_string(),
        workspaces: Arc::new(Mutex::new(Vec::new())),
        last_hotkey_info: Arc::new(Mutex::new(None)), // Initialize to None
        hotkey_promise: Arc::new(Mutex::new(None)),   // Initialize the promise
        initial_validation_done: Arc::new(Mutex::new(false)), // Initialize flag to false
        registered_hotkeys: Arc::new(Mutex::new(HashMap::new())), // Initialize the map
        hotkey_manager,
    };

    // Launch GUI and set the taskbar icon after creating the window
    gui::run_gui(app);
}

/// Ensures a valid log4rs.yaml file exists and initializes logging.
fn ensure_logging_initialized() {
    // Attempt to initialize logging configuration
    if let Err(err) = log4rs::init_file("log4rs.yaml", Default::default()) {
        eprintln!("Failed to initialize log4rs: {}", err);

        // Create a default log4rs.yaml file
        let default_config = r#"
appenders:
  file:
    kind: file
    path: "multi_manager.log"
    append: false
    encoder:
      pattern: "{d} - {l} - {m}{n}"

root:
  level: info
  appenders:
    - file
"#;

        if let Err(e) = File::create("log4rs.yaml")
            .and_then(|mut file| file.write_all(default_config.as_bytes()))
        {
            eprintln!("Failed to create default log4rs.yaml: {}", e);
            std::process::exit(1); // Exit if we cannot create the default configuration
        }

        // Retry initializing log4rs with the newly created configuration file
        if let Err(e) = log4rs::init_file("log4rs.yaml", Default::default()) {
            eprintln!(
                "Failed to reinitialize log4rs with default configuration: {}",
                e
            );
            std::process::exit(1); // Exit if retry fails
        }
    }
}
