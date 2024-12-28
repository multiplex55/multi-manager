use crate::window_manager::{
    capture_hotkey_dialog, get_active_window, get_window_position, move_window,
};
use crate::workspace::{save_workspaces, Window, Workspace};
use eframe::egui;
use eframe::{self, App as EframeApp};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use windows::Win32::Foundation::HWND;

#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub current_workspace: Option<usize>,
}

pub fn run_gui(mut app: App) {
    if let Ok(data) = fs::read_to_string("workspaces.json") {
        if let Ok(workspaces) = serde_json::from_str::<Vec<Workspace>>(&data) {
            app.workspaces = workspaces;
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    let _ = eframe::run_native("Multi Manager", options, Box::new(|_cc| Ok(Box::new(app))));
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut workspace_to_delete = None;
        let mut save_workspaces_flag = false;

        let hotkey_capture_result = Arc::new(Mutex::new(None::<String>));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Multi Manager");

            // Global buttons
            ui.horizontal(|ui| {
                if ui.button("Save Workspaces").clicked() {
                    save_workspaces_flag = true;
                }

                if ui.button("Add New Workspace").clicked() {
                    let new_workspace = Workspace {
                        name: format!("Workspace {}", self.workspaces.len() + 1),
                        hotkey: None,
                        windows: Vec::new(),
                    };
                    self.workspaces.push(new_workspace);
                }
            });

            ui.separator();

            // Loop through each workspace
            for (i, workspace) in self.workspaces.iter_mut().enumerate() {
                egui::CollapsingHeader::new(&workspace.name)
                    .id_salt(i)
                    .default_open(true)
                    .show(ui, |ui| {
                        // Hotkey settings
                        ui.horizontal(|ui| {
                            ui.label("Hotkey:");
                            if let Some(hotkey) = &workspace.hotkey {
                                ui.label(hotkey);
                            } else {
                                ui.label("None");
                            }

                            // Set Hotkey button
                            if ui.button("Set Hotkey").clicked() {
                                let result = hotkey_capture_result.clone();
                                std::thread::spawn(move || {
                                    capture_hotkey_dialog(result);
                                });
                            }

                            // Apply captured hotkey
                            if let Some(hotkey) = hotkey_capture_result.lock().unwrap().take() {
                                workspace.hotkey = Some(hotkey);
                            }
                        });

                        // Manage windows
                        let mut window_to_delete = None;
                        for (j, window) in workspace.windows.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(&window.title);

                                if ui.button("Move to Target").clicked() {
                                    move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.target.0,
                                        window.target.1,
                                        window.target.2,
                                        window.target.3,
                                    );
                                }

                                if ui.button("Move to Home").clicked() {
                                    move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.home.0,
                                        window.home.1,
                                        window.home.2,
                                        window.home.3,
                                    );
                                }

                                if ui.button("Delete").clicked() {
                                    window_to_delete = Some(j);
                                }
                            });

                            // Home and Target settings
                            ui.horizontal(|ui| {
                                ui.label("Home:");
                                ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));
                                if ui.button("Set Home").clicked() {
                                    if let Ok((x, y, w, h)) = get_window_position(HWND(
                                        window.id as *mut std::ffi::c_void,
                                    )) {
                                        window.home = (x, y, w, h);
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Target:");
                                ui.add(egui::DragValue::new(&mut window.target.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.target.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.target.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.target.3).prefix("h: "));
                                if ui.button("Set Target").clicked() {
                                    if let Ok((x, y, w, h)) = get_window_position(HWND(
                                        window.id as *mut std::ffi::c_void,
                                    )) {
                                        window.target = (x, y, w, h);
                                    }
                                }
                            });
                        }

                        if let Some(index) = window_to_delete {
                            workspace.windows.remove(index);
                        }

                        if ui.button("Capture Active Window").clicked() {
                            if let Some(hwnd) = get_active_window() {
                                let title = format!("Window {:?}", hwnd.0);
                                let new_window = Window {
                                    id: hwnd.0 as usize,
                                    title,
                                    home: (0, 0, 800, 600),
                                    target: (0, 0, 800, 600),
                                };
                                workspace.windows.push(new_window);
                            }
                        }

                        if ui.button("Delete Workspace").clicked() {
                            workspace_to_delete = Some(i);
                        }
                    });
            }

            if let Some(index) = workspace_to_delete {
                self.workspaces.remove(index);
            }
        });

        if save_workspaces_flag {
            let workspaces = self.workspaces.clone();
            save_workspaces(&workspaces, "workspaces.json");
        }
    }
}
