mod gui;
mod window_manager;
mod workspace;

use log::info;
use window_manager::listen_for_keyboard_event;

use std::env;
use std::sync::{Arc, Mutex};

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).expect("Failed to initialize log4rs");

    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    // Uncomment this line to test keyboard event listener
    // listen_for_keyboard_event("Ctrl+H");

    let app = gui::App {
        workspaces: Vec::new(),
        current_workspace: None,
        hotkey_thread_running: Arc::new(Mutex::new(false)),
    };

    gui::run_gui(app);
}
