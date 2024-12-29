use crate::window_manager::{
    get_active_window, handle_hotkey_events, listen_for_keys_with_dialog, move_window,
    register_hotkey,
};
use crate::workspace::{load_workspaces, save_workspaces, Window, Workspace};
use eframe::egui;
use eframe::{self, App as EframeApp};
use std::sync::{Arc, Mutex};
use std::thread;
use windows::Win32::Foundation::HWND;

#[derive(Clone)]
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub current_workspace: Option<usize>,
    pub hotkey_thread_running: Arc<Mutex<bool>>,
}

pub fn run_gui(mut app: App) {
    app.workspaces = load_workspaces("workspaces.json");

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    if let Ok(mut running) = app.hotkey_thread_running.lock() {
        if !*running {
            *running = true;
            let workspaces = Arc::new(Mutex::new(app.workspaces.clone()));
            thread::spawn({
                let running_flag = app.hotkey_thread_running.clone();
                move || {
                    handle_hotkey_events(workspaces.clone());
                    *running_flag.lock().unwrap() = false;
                }
            });
        }
    }

    let _ = eframe::run_native("Multi Manager", options, Box::new(|_cc| Ok(Box::new(app))));
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut workspace_to_delete = None;
        let mut save_workspaces_flag = false;
        let mut new_workspace_to_add: Option<Workspace> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Multi Manager");

            ui.horizontal(|ui| {
                if ui.button("Save Workspaces").clicked() {
                    save_workspaces_flag = true;
                }

                if ui.button("Add New Workspace").clicked() {
                    new_workspace_to_add = Some(Workspace {
                        name: format!("Workspace {}", self.workspaces.len() + 1),
                        hotkey: None,
                        windows: Vec::new(),
                    });
                }
            });

            ui.separator();

            for (i, workspace) in self.workspaces.iter_mut().enumerate() {
                egui::CollapsingHeader::new(&workspace.name)
                    .id_salt(i)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Hotkey:");
                            if let Some(hotkey) = &workspace.hotkey {
                                ui.label(hotkey);
                            } else {
                                ui.label("None");
                            }

                            if ui.button("Set Hotkey").clicked() {
                                workspace.hotkey = Some("Ctrl+Alt+H".to_string());
                                register_hotkey(i as i32, workspace.hotkey.as_ref().unwrap());
                            }
                        });

                        let mut window_to_delete = None;
                        for (j, window) in workspace.windows.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(&window.title);

                                if ui.button("Move to Target").clicked() {
                                    if let Err(e) = move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.target.0,
                                        window.target.1,
                                        window.target.2,
                                        window.target.3,
                                    ) {
                                        ui.label(format!("Error: {}", e));
                                    }
                                }

                                if ui.button("Move to Home").clicked() {
                                    if let Err(e) = move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.home.0,
                                        window.home.1,
                                        window.home.2,
                                        window.home.3,
                                    ) {
                                        ui.label(format!("Error: {}", e));
                                    }
                                }

                                if ui.button("Delete").clicked() {
                                    window_to_delete = Some(j);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Home:");
                                ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Target:");
                                ui.add(egui::DragValue::new(&mut window.target.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.target.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.target.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.target.3).prefix("h: "));
                            });
                        }

                        if let Some(index) = window_to_delete {
                            workspace.windows.remove(index);
                        }

                        if ui.button("Capture Active Window").clicked() {
                            if let Some("Enter") = listen_for_keys_with_dialog() {
                                if let Some((hwnd, title)) = get_active_window() {
                                    workspace.windows.push(Window {
                                        id: hwnd.0 as usize,
                                        title,
                                        home: (0, 0, 800, 600),
                                        target: (0, 0, 800, 600),
                                    });
                                }
                            } else {
                                ui.label("Capture canceled.");
                            }
                        }

                        if ui.button("Delete Workspace").clicked() {
                            workspace_to_delete = Some(i);
                        }
                    });
            }

            if let Some(new_workspace) = new_workspace_to_add {
                self.workspaces.push(new_workspace);
            }

            if let Some(index) = workspace_to_delete {
                self.workspaces.remove(index);
            }
        });

        if save_workspaces_flag {
            save_workspaces(&self.workspaces, "workspaces.json");
        }
    }
}
