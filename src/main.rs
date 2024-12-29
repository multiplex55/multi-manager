mod gui;
mod window_manager;
mod workspace;

use log::info;
use log4rs;

use std::env;
use std::sync::{Arc, Mutex};

fn main() {
    // Initialize log4rs
    log4rs::init_file("log4rs.yaml", Default::default()).expect("Failed to initialize log4rs");

    // Set the environment variable to enable backtrace on panic
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    // Initialize the app with an empty workspace list and a thread-safe hotkey thread flag.
    let app = gui::App {
        workspaces: Vec::new(),
        current_workspace: None,
        hotkey_thread_running: Arc::new(Mutex::new(false)),
    };

    // Run the GUI
    gui::run_gui(app);
}
