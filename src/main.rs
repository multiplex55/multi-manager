mod gui;
mod window_manager;
mod workspace;

fn main() {
    let app = gui::App {
        workspaces: vec![],
        current_workspace: None,
    };

    gui::run_gui(app);
}
