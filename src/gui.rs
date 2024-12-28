use crate::window_manager::get_active_window;
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
                if let Some(hwnd) = get_active_window() {
                    let new_window = Window {
                        id: hwnd.0 as usize,
                        title: format!("Window {:?}", hwnd.0), // Replace with actual title fetching
                        home: (0, 0, 800, 600),                // Replace with actual dimensions
                        target: (0, 0, 800, 600),              // Replace with desired dimensions
                    };

                    let new_workspace = Workspace {
                        name: format!("Workspace {}", self.workspaces.len() + 1),
                        hotkey: None,
                        windows: vec![new_window],
                    };

                    self.workspaces.push(new_workspace);
                }
            }

            // List Workspaces
            ui.separator();
            let mut indices_to_remove = Vec::new();
            for (i, workspace) in self.workspaces.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&workspace.name);

                    // Delete Workspace Button
                    if ui.button("Delete").clicked() {
                        indices_to_remove.push(i);
                    }
                });
            }
            // Remove workspaces after iteration
            for &i in indices_to_remove.iter().rev() {
                self.workspaces.remove(i);
            }

            // Save Workspaces Button
            if ui.button("Save Workspaces").clicked() {
                save_workspaces(&self.workspaces, "workspaces.json");
            }
        });
    }
}
