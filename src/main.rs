mod gui;
mod window_manager;
mod workspace;

use log::info;
use std::env;
use std::sync::{Arc, Mutex};

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).expect("Failed to initialize log4rs");

    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    let app = gui::App {
        workspaces: Vec::new(),
        current_workspace: None,
        hotkey_thread_running: Arc::new(Mutex::new(false)),
        last_hotkey_info: Arc::new(Mutex::new(None)), // Initialize to None
    };

    gui::run_gui(app);
}
