#![windows_subsystem = "windows"]

mod gui;
mod utils;
mod window_manager;
mod workspace;

use log::info;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write; // Fix for write_all error
use std::sync::{Arc, Mutex};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    LoadImageW, SetClassLongPtrW, GCLP_HICON, IMAGE_ICON, LR_DEFAULTSIZE,
};

fn main() {
    // Ensure logging is initialized
    ensure_logging_initialized();

    // Backtrace for Debug
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    // Initialize the application states
    let app = gui::App {
        app_title_name: "Multi Manager".to_string(),
        workspaces: Arc::new(Mutex::new(Vec::new())),
        last_hotkey_info: Arc::new(Mutex::new(None)), // Initialize to None
        hotkey_promise: Arc::new(Mutex::new(None)),   // Initialize the promise
        initial_validation_done: Arc::new(Mutex::new(false)), // Initialize flag to false
        registered_hotkeys: Arc::new(Mutex::new(HashMap::new())), // Initialize the map
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

/// Sets the taskbar icon for the application.
pub fn set_taskbar_icon(hwnd: HWND, icon_path: &str) {
    unsafe {
        let wide_path: Vec<u16> = icon_path.encode_utf16().chain(std::iter::once(0)).collect();
        let icon = LoadImageW(
            None,
            PCWSTR(wide_path.as_ptr()),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE,
        )
        .unwrap_or(windows::Win32::Foundation::HANDLE(std::ptr::null_mut())); // Fix: Use `windows::Win32::Foundation::HANDLE`.

        if !icon.0.is_null() {
            // Fix: Access `.0` for pointer check
            SetClassLongPtrW(hwnd, GCLP_HICON, icon.0 as isize);
            info!("Taskbar icon set successfully.");
        } else {
            eprintln!("Failed to load icon: {}", icon_path);
        }
    }
}
