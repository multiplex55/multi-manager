use crate::workspace::{save_workspaces, Workspace};
use eframe::egui;
use eframe::{self, App as EframeApp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub current_workspace: Option<usize>,
}

pub fn run_gui(app: App) {
    // let options = eframe::NativeOptions {
    //     initial_window_size: Some(egui::vec2(800.0, 600.0)), // Ensure proper egui import
    //     ..Default::default()
    // };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Multi Manager",
        options,
        Box::new(|_cc| Ok(Box::new(app))), // Wrap in `Ok` to match expected type
    );
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Workspace Manager");
            if ui.button("Save Workspaces").clicked() {
                save_workspaces(&self.workspaces, "workspaces.json");
            }
        });
    }
}
