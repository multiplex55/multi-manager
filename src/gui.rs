use crate::utils::*;
use crate::window_manager::{check_hotkeys, register_hotkey};
use crate::workspace::*;
use eframe::egui;
use eframe::egui::ViewportBuilder;
use eframe::NativeOptions;
use eframe::{self, App as EframeApp};
use log::{info, warn};
use poll_promise::Promise;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct App {
    pub app_title_name: String,
    pub workspaces: Arc<Mutex<Vec<Workspace>>>,
    pub last_hotkey_info: Arc<Mutex<Option<(String, Instant)>>>,
    pub hotkey_promise: Arc<Mutex<Option<Promise<()>>>>,
    pub initial_validation_done: Arc<Mutex<bool>>,
    pub registered_hotkeys: Arc<Mutex<HashMap<String, usize>>>,
}

pub struct WorkspaceControlContext<'a> {
    pub workspace_to_delete: &'a mut Option<usize>,
    pub move_up_index: &'a mut Option<usize>,
    pub move_down_index: &'a mut Option<usize>,
    pub workspaces_len: usize,
    pub index: usize,
}

/// Launches the application GUI and initializes background processes.
pub fn run_gui(app: App) {
    {
        let mut workspaces = app.workspaces.lock().unwrap();
        *workspaces = load_workspaces("workspaces.json", &app);
    }

    app.validate_initial_hotkeys();

    let app_for_promise = app.clone();
    let hotkey_promise = Promise::spawn_thread("Hotkey Checker", move || loop {
        check_hotkeys(&app_for_promise);
        thread::sleep(Duration::from_millis(100));
    });
    *app.hotkey_promise.lock().unwrap() = Some(hotkey_promise);

    let icon_data = include_bytes!("../resources/app_icon.ico");
    let image = image::load_from_memory(icon_data)
        .expect("Failed to load embedded icon")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let icon_rgba = image.into_raw();

    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_icon(egui::IconData {
            rgba: icon_rgba,
            width,
            height,
        }),
        ..Default::default()
    };

    eframe::run_native(
        &app.app_title_name.clone(),
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .expect("Failed to run GUI");
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut save_flag = false;
        let mut new_workspace: Option<Workspace> = None;
        let mut workspace_to_delete: Option<usize> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui, &mut save_flag, &mut new_workspace);
            ui.separator();
            self.render_workspace_list(ui, &mut workspace_to_delete);
        });

        if save_flag {
            self.save_workspaces();
        }
        if let Some(ws) = new_workspace {
            self.add_workspace(ws);
        }
        if let Some(index) = workspace_to_delete {
            self.delete_workspace(index);
        }
    }
}

impl App {
    /// Renders the header with Save and Add Workspace buttons.
    fn render_header(
        &self,
        ui: &mut egui::Ui,
        save_flag: &mut bool,
        new_workspace: &mut Option<Workspace>,
    ) {
        ui.heading(&self.app_title_name);
        ui.horizontal(|ui| {
            if ui.button("Save Workspaces").clicked() {
                *save_flag = true;
                show_message_box("Workspaces saved successfully!", "Save");
            }
            if ui.button("Add New Workspace").clicked() {
                let workspaces = self.workspaces.lock().unwrap();
                *new_workspace = Some(Workspace {
                    name: format!("Workspace {}", workspaces.len() + 1),
                    hotkey: None,
                    windows: Vec::new(),
                    disabled: false,
                    valid: false,
                });
            }
        });
    }

    /// Renders the list of workspaces.
    fn render_workspace_list(
        &mut self,
        ui: &mut egui::Ui,
        workspace_to_delete: &mut Option<usize>,
    ) {
        let mut move_up_index: Option<usize> = None;
        let mut move_down_index: Option<usize> = None;

        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut workspaces = self.workspaces.lock().unwrap();
                let workspaces_len = workspaces.len();

                for (i, workspace) in workspaces.iter_mut().enumerate() {
                    workspace.validate_workspace();
                    let header_text = workspace.get_header_text();
                    let header_id = egui::Id::new(format!("workspace_{}_header", i));

                    egui::CollapsingHeader::new(header_text)
                        .id_salt(header_id)
                        .default_open(true)
                        .show(ui, |ui| {
                            workspace.render_details(ui);

                            let mut context = WorkspaceControlContext {
                                workspace_to_delete,
                                move_up_index: &mut move_up_index,
                                move_down_index: &mut move_down_index,
                                workspaces_len,
                                index: i,
                            };

                            self.render_workspace_controls(ui, workspace, &mut context);
                        });
                }
            });

        if let Some(i) = move_up_index {
            let mut workspaces = self.workspaces.lock().unwrap();
            if i > 0 {
                workspaces.swap(i, i - 1);
            }
        }

        if let Some(i) = move_down_index {
            let mut workspaces = self.workspaces.lock().unwrap();
            if i < workspaces.len() - 1 {
                workspaces.swap(i, i + 1);
            }
        }
    }

    /// Renders the controls for a workspace, such as move up, move down, and delete.
    fn render_workspace_controls(
        &self,
        ui: &mut egui::Ui,
        workspace: &mut Workspace,
        context: &mut WorkspaceControlContext,
    ) {
        // Workspace disable checkbox
        ui.horizontal(|ui| {
            ui.checkbox(&mut workspace.disabled, "Disable Workspace");
        });

        ui.horizontal(|ui| {
            if context.index > 0 && ui.button("Move ⏶").clicked() {
                *context.move_up_index = Some(context.index);
            }
            if context.index < context.workspaces_len - 1 && ui.button("Move ⏷").clicked() {
                *context.move_down_index = Some(context.index);
            }
            if ui.button("Delete Workspace").clicked() {
                let confirmation_message = format!(
                    "Are you sure you want to delete workspace '{}'? This action cannot be undone.",
                    context.index
                );
                if show_confirmation_box(&confirmation_message, "Confirm Deletion") {
                    *context.workspace_to_delete = Some(context.index);
                }
            }
        });
    }

    /// Saves the workspaces to the JSON file.
    fn save_workspaces(&self) {
        let workspaces = self.workspaces.lock().unwrap();
        save_workspaces(&workspaces, "workspaces.json");
        info!("Workspaces saved successfully.");
    }

    /// Adds a new workspace to the list.
    fn add_workspace(&self, workspace: Workspace) {
        let mut workspaces = self.workspaces.lock().unwrap();
        workspaces.push(workspace);
    }

    /// Deletes a workspace at the given index.
    fn delete_workspace(&self, index: usize) {
        let mut workspaces = self.workspaces.lock().unwrap();
        workspaces.remove(index);
    }

    /// Validates the hotkeys for all workspaces.
    fn validate_initial_hotkeys(&self) {
        let mut initial_validation_done = self.initial_validation_done.lock().unwrap();
        if !*initial_validation_done {
            let mut workspaces = self.workspaces.lock().unwrap();
            for (i, workspace) in workspaces.iter_mut().enumerate() {
                if let Some(hotkey) = &workspace.hotkey {
                    if !register_hotkey(self, i as i32, hotkey) {
                        warn!(
                            "Failed to register hotkey '{}' for workspace '{}'",
                            hotkey, workspace.name
                        );
                    }
                }
            }
            *initial_validation_done = true;
        }
    }
}
