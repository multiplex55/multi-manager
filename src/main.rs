mod gui;
mod window_manager;
mod workspace;

use std::env;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let app = gui::App {
        workspaces: vec![],
        current_workspace: None,
    };

    gui::run_gui(app);
}
