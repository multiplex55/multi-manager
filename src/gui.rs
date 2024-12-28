use crate::window_manager::listen_for_keys_with_dialog;
use crate::workspace::{save_workspaces, Window, Workspace};
use eframe::egui;
use eframe::{self, App as EframeApp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub current_workspace: Option<usize>,
}

pub fn run_gui(app: App) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Workspace Manager",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Workspace Manager");

            // Capture Window Button
            if ui.button("Capture Active Window").clicked() {
                match listen_for_keys_with_dialog() {
                    Some("Enter") => {
                        if let Some(hwnd) = crate::window_manager::get_active_window() {
                            let new_window = Window {
                                id: hwnd.0 as usize,
                                title: format!("Window {:?}", hwnd.0),
                                home: (0, 0, 800, 600),
                                target: (0, 0, 800, 600),
                            };

                            let new_workspace = Workspace {
                                name: format!("Workspace {}", self.workspaces.len() + 1),
                                hotkey: None,
                                windows: vec![new_window],
                            };

                            self.workspaces.push(new_workspace);
                        }
                    }
                    Some("Esc") => {
                        ui.label("Window capture canceled.");
                    }
                    _ => {}
                }
            }

            ui.separator();

            // Collect updates to avoid mutable/immutable borrow conflict
            let mut actions = Vec::new();

            for (i, workspace) in self.workspaces.iter().enumerate() {
                egui::CollapsingHeader::new(&workspace.name)
                    .id_salt(i) // Updated to use `id_salt`
                    .default_open(true)
                    .show(ui, |ui| {
                        for window in &workspace.windows {
                            ui.horizontal(|ui| {
                                ui.label(&window.title);
                                if ui.button("Remove").clicked() {
                                    // Mark the window for removal
                                    actions.push(("remove_window", i, Some(window.id)));
                                }
                            });
                        }

                        if ui.button("Delete Workspace").clicked() {
                            // Mark the workspace for deletion
                            actions.push(("delete_workspace", i, None));
                        }
                    });
            }

            // Apply collected updates
            for action in actions {
                match action {
                    ("remove_window", workspace_index, Some(window_id)) => {
                        if let Some(workspace) = self.workspaces.get_mut(workspace_index) {
                            workspace.windows.retain(|w| w.id != window_id);
                        }
                    }
                    ("delete_workspace", workspace_index, None) => {
                        self.workspaces.remove(workspace_index);
                    }
                    _ => {}
                }
            }

            ui.separator();

            // Save Workspaces Button
            if ui.button("Save Workspaces").clicked() {
                save_workspaces(&self.workspaces, "workspaces.json");
            }
        });
    }
}
