mod gui;
mod window_manager;
mod workspace;

use std::env;
use std::sync::{Arc, Mutex};

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    // Initialize the app with an empty workspace list and a thread-safe hotkey thread flag.
    let app = gui::App {
        workspaces: Vec::new(),
        current_workspace: None,
        hotkey_thread_running: Arc::new(Mutex::new(false)),
    };

    // Run the GUI
    gui::run_gui(app);
}
