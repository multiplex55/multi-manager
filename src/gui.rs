use crate::window_manager::*;
use crate::workspace::*;
use eframe::egui;
use eframe::{self, App as EframeApp};
use log::{info, warn};
use poll_promise::Promise;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;

#[derive(Clone)]
pub struct App {
    pub workspaces: Arc<Mutex<Vec<Workspace>>>,
    pub last_hotkey_info: Arc<Mutex<Option<(String, Instant)>>>,
    pub hotkey_promise: Arc<Mutex<Option<Promise<()>>>>,
}

pub fn run_gui(app: App) {
    // Load workspaces and initialize
    {
        let mut workspaces = app.workspaces.lock().unwrap();
        *workspaces = load_workspaces("workspaces.json");
    }

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    // Start hotkey checker in a background thread with PollPromise
    let app_for_promise = app.clone();
    let hotkey_promise = Promise::spawn_thread("Hotkey Checker", move || loop {
        check_hotkeys(&app_for_promise);
        thread::sleep(Duration::from_millis(250));
    });

    *app.hotkey_promise.lock().unwrap() = Some(hotkey_promise);

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
                    let workspaces = self.workspaces.lock().unwrap();
                    new_workspace_to_add = Some(Workspace {
                        name: format!("Workspace {}", workspaces.len() + 1),
                        hotkey: None,
                        windows: Vec::new(),
                    });
                }
            });

            // Display debug info for the last detected hotkey
            if let Some((hotkey, timestamp)) = self.last_hotkey_info.lock().unwrap().clone() {
                ui.label(format!(
                    "Last Hotkey Detected: {} at {:?}",
                    hotkey,
                    timestamp.elapsed()
                ));
            } else {
                ui.label("No hotkey detected yet.");
            }

            ui.separator();

            let mut workspaces = self.workspaces.lock().unwrap();
            for (i, workspace) in workspaces.iter_mut().enumerate() {
                egui::CollapsingHeader::new(&workspace.name)
                    .id_salt(i)
                    .default_open(true)
                    .show(ui, |ui| {
                        use regex::Regex;

                        ui.horizontal(|ui| {
                            ui.label("Hotkey:");
                        
                            let mut current_hotkey = workspace.hotkey.clone().unwrap_or_else(|| "None".to_string());
                            let mut temp_hotkey = current_hotkey.clone();
                        
                            let response = ui.text_edit_singleline(&mut temp_hotkey);
                        
                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                // Validate the entered hotkey
                                let valid_hotkey_pattern = r"(?i)^(ctrl|alt|shift|win)(\+(ctrl|alt|shift|win))*\+([a-z]|f[1-9]|f1[0-9]|f2[0-4]|numpad[0-9]|up|down|left|right|backspace|tab|enter|pause|capslock|escape|space|pageup|pagedown|end|home|insert|delete|oem_(plus|comma|minus|period|1|2|3|4|5|6|7)|printscreen|scrolllock|numlock|left(ctrl|shift|alt)|right(ctrl|shift|alt))$|^(?:[a-z]|f[1-9]|f1[0-9]|f2[0-4]|numpad[0-9]|up|down|left|right|backspace|tab|enter|pause|capslock|escape|space|pageup|pagedown|end|home|insert|delete|oem_(plus|comma|minus|period|1|2|3|4|5|6|7)|printscreen|scrolllock|numlock|left(ctrl|shift|alt)|right(ctrl|shift|alt))$";
                        
                                if let Ok(hotkey_regex) = Regex::new(valid_hotkey_pattern) {
                                    if hotkey_regex.is_match(&temp_hotkey) {
                                        // Valid hotkey: update the workspace and register it
                                        if let Some(_existing_hotkey) = &workspace.hotkey {
                                            unregister_hotkey(i as i32); // Unregister the previous hotkey
                                        }
                        
                                        if !register_hotkey(i as i32, &temp_hotkey) {
                                            warn!("Failed to set hotkey for workspace '{}'.", workspace.name);
                                        } else {
                                            workspace.hotkey = Some(temp_hotkey.clone());
                                            info!("Set hotkey '{}' for workspace '{}'.", temp_hotkey, workspace.name);
                                        }
                                    } else {
                                        // Invalid hotkey: revert to the previous value
                                        warn!("Invalid hotkey string: '{}'. Keeping previous value.", temp_hotkey);
                                        temp_hotkey = current_hotkey.clone(); // Restore the previous value
                                    }
                                }
                            }
                        
                            // Update the textbox with the latest valid hotkey
                            current_hotkey = temp_hotkey.clone();
                            workspace.hotkey = Some(current_hotkey.clone());
                        });
                        
                        let mut window_to_delete = None;
                        for (j, window) in workspace.windows.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(&window.title);

                                if ui.button("Move to Home").clicked() {
                                    if let Err(e) = move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.home.0,
                                        window.home.1,
                                        window.home.2,
                                        window.home.3,
                                    ) {
                                        warn!("Error moving window '{}': {}", window.title, e);
                                    }
                                }

                                if ui.button("Move to Target").clicked() {
                                    if let Err(e) = move_window(
                                        HWND(window.id as *mut std::ffi::c_void),
                                        window.target.0,
                                        window.target.1,
                                        window.target.2,
                                        window.target.3,
                                    ) {
                                        warn!("Error moving window '{}': {}", window.title, e);
                                    }
                                }

                                if ui.button("Delete").clicked() {
                                    window_to_delete = Some(j);
                                    info!("Deleting window '{}'", window.title);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Home:");
                                ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));

                                if ui.button("Capture Home").clicked() {
                                    if let Some((hwnd, _)) = get_active_window() {
                                        if let Ok((x, y, w, h)) = get_window_position(hwnd) {
                                            window.home = (x, y, w, h);
                                            info!(
                                                "Captured current window position for Home: {:?}",
                                                window.home
                                            );
                                        }
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Target:");
                                ui.add(egui::DragValue::new(&mut window.target.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.target.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.target.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.target.3).prefix("h: "));

                                if ui.button("Capture Target").clicked() {
                                    if let Some((hwnd, _)) = get_active_window() {
                                        if let Ok((x, y, w, h)) = get_window_position(hwnd) {
                                            window.target = (x, y, w, h);
                                            info!(
                                                "Captured current window position for Target: {:?}",
                                                window.target
                                            );
                                        }
                                    }
                                }
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
                                        title: title.clone(),
                                        home: (0, 0, 800, 600),
                                        target: (0, 0, 800, 600),
                                    });
                                    info!("Captured active window: '{}'.", title);
                                }
                            } else {
                                warn!("Capture canceled.");
                            }
                        }

                        if ui.button("Delete Workspace").clicked() {
                            workspace_to_delete = Some(i);
                            info!("Deleting workspace '{}'.", workspace.name);
                        }
                    });
            }

            if let Some(new_workspace) = new_workspace_to_add {
                workspaces.push(new_workspace);
            }

            if let Some(index) = workspace_to_delete {
                workspaces.remove(index);
            }
        });

        if save_workspaces_flag {
            save_workspaces(&self.workspaces.lock().unwrap(), "workspaces.json");
            info!("Workspaces saved to file.");
        }
    }
}

fn check_hotkeys(app: &App) {
    let mut workspaces_to_toggle = Vec::new();
    let workspaces = app.workspaces.lock().unwrap();

    for (i, workspace) in workspaces.iter().enumerate() {
        if let Some(ref hotkey) = workspace.hotkey {
            if is_hotkey_pressed(hotkey) {
                info!(
                    "Activating workspace '{}' via hotkey '{}'.",
                    workspace.name, hotkey
                );
                workspaces_to_toggle.push(i);

                let mut last_hotkey_info = app.last_hotkey_info.lock().unwrap();
                *last_hotkey_info = Some((hotkey.clone(), Instant::now()));
            }
        }
    }

    drop(workspaces); // Release lock before toggling

    let mut workspaces = app.workspaces.lock().unwrap();
    for index in workspaces_to_toggle {
        if let Some(workspace) = workspaces.get_mut(index) {
            toggle_workspace_windows(workspace);
        }
    }
}
