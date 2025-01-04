mod gui;
mod utils;
mod window_manager;
mod workspace;

use log::info;
use std::env;
use std::sync::{Arc, Mutex};

/// The entry point of the Multi Manager application.
///
/// - Initializes logging using `log4rs`.
/// - Configures the environment for debugging (e.g., enabling backtraces).
/// - Initializes the application state and launches the GUI.
///
/// # Panics
/// - Panics if the `log4rs` configuration file (`log4rs.yaml`) cannot be loaded.
///
/// # Environment Variables
/// - Sets `RUST_BACKTRACE` to `1` for enabling detailed error stack traces.
fn main() {
    // Initalize logging configuration
    log4rs::init_file("log4rs.yaml", Default::default()).expect("Failed to initialize log4rs");

    // Backtrace for Debug
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    // Initialize the application states
    let app = gui::App {
        workspaces: Arc::new(Mutex::new(Vec::new())),
        last_hotkey_info: Arc::new(Mutex::new(None)), // Initialize to None
        hotkey_promise: Arc::new(Mutex::new(None)),   // Initialize the promise
        initial_validation_done: Arc::new(Mutex::new(false)), // Initialize flag to false
    };

    // Launch GUI
    gui::run_gui(app);
}
