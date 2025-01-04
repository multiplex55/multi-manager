use crate::window_manager::*;
use crate::workspace::*;
use crate::utils::*;
use eframe::egui;
use eframe::{self, App as EframeApp};
use log::{info, warn};
use poll_promise::Promise;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

#[derive(Clone)]
pub struct App {
    pub workspaces: Arc<Mutex<Vec<Workspace>>>,
    pub last_hotkey_info: Arc<Mutex<Option<(String, Instant)>>>,
    pub hotkey_promise: Arc<Mutex<Option<Promise<()>>>>,
    pub initial_validation_done: Arc<Mutex<bool>>, // New flag for initial validation
}

/// Launches the application GUI and manages the lifecycle of the application.
///
/// - Loads workspaces from the saved file (`workspaces.json`).
/// - Starts a background thread to monitor hotkey presses.
/// - Launches the GUI using `eframe`.
///
/// # Arguments
/// * `app` - The application state encapsulated in an `App` struct.
pub fn run_gui(app: App) {
    // Load workspaces and initialize
    {
        let mut workspaces = app.workspaces.lock().unwrap();
        *workspaces = load_workspaces("workspaces.json");
    }
    app.validate_initial_hotkeys(); // Perform initial validation of hotkeys

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    // Start hotkey checker in a background thread with PollPromise
    let app_for_promise = app.clone();
    let hotkey_promise = Promise::spawn_thread("Hotkey Checker", move || loop {
        check_hotkeys(&app_for_promise);
        thread::sleep(Duration::from_millis(100));

    });

    *app.hotkey_promise.lock().unwrap() = Some(hotkey_promise);
    let _ = eframe::run_native("Multi Manager", options, Box::new(|_cc| Ok(Box::new(app))));
}

impl EframeApp for App {
    /// Updates the GUI layout and handles user interactions.
    ///
    /// - Displays the list of workspaces and their details.
    /// - Allows users to add, edit, disable, or delete workspaces.
    /// - Handles workspace hotkey assignment and validation.
    /// - Saves workspaces to a file when requested.
    ///
    /// # Arguments
    /// * `ctx` - The egui context for rendering the GUI.
    /// * `_frame` - The frame provided by eframe for managing the application window.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let mut workspace_to_delete = None;
        let mut save_workspaces_flag = false;
        let mut new_workspace_to_add: Option<Workspace> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Multi Manager");

            ui.horizontal(|ui| {
                if ui.button("Save Workspaces").clicked() {
                    save_workspaces_flag = true;
                    show_message_box("Save Workspaces Successful","Workspace Result");
                }

                if ui.button("Add New Workspace").clicked() {
                    let workspaces = self.workspaces.lock().unwrap();
                    new_workspace_to_add = Some(Workspace {
                        name: format!("Workspace {}", workspaces.len() + 1),
                        hotkey: None,
                        windows: Vec::new(),
                        disabled: false,
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
                let header_id = egui::Id::new(format!("workspace_{}_header", i));
                let mut is_renaming = ui
                    .memory_mut(|mem| mem.data.get_temp::<bool>(header_id).unwrap_or(false));

                let mut new_name = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp::<String>(header_id.with("wrkspce_name"))
                        .unwrap_or_else(|| workspace.name.clone())
                });

                // Check if the workspace is valid
                let is_workspace_valid = {
                    let hotkey_valid = workspace.hotkey.as_ref().map_or(false, |hotkey| is_valid_key_combo(hotkey));
                    let windows_valid = workspace
                        .windows
                        .iter()
                        .all(|window| unsafe { IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool() });
                    hotkey_valid && windows_valid
                };   

                // Set header text color based on validity
                let header_text = if workspace.disabled{
                    egui::RichText::new(&workspace.name).color(egui::Color32::ORANGE)
                }
                else if is_workspace_valid {
                    egui::RichText::new(&workspace.name).color(egui::Color32::GREEN)
                } else {
                    egui::RichText::new(&workspace.name).color(egui::Color32::RED)
                };

                ui.horizontal(|ui| {

                let header_response = egui::CollapsingHeader::new(header_text)
                    .id_salt(i)
                    .default_open(true)
                    .show(ui, |ui| {
                        use egui::{self, Color32};
                        use log::{info, warn};

                        ui.horizontal(|ui| {
                            ui.label("Hotkey:");

                            // Retrieve or initialize the temporary hotkey
                            let workspace_id = i; // Unique ID for the workspace
                            let id = egui::Id::new(workspace_id); // Convert workspace index to egui Id
                            let mut temp_hotkey = ui.memory_mut(|mem| {
                                mem.data.get_temp::<String>(id).unwrap_or_else(|| {
                                    workspace
                                        .hotkey
                                        .clone()
                                        .unwrap_or_else(|| "None".to_string())
                                    // info!(
                                    //     "Initializing temp_hotkey for workspace '{}': {}",
                                    //     workspace.name, hotkey
                                    // );
                                })
                            });

                            // Editable text field for the hotkey
                            let response = ui.text_edit_singleline(&mut temp_hotkey);

                            if response.changed() {
                                // Save temporary changes back to memory
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp::<String>(id, temp_hotkey.clone())
                                });
                                info!(
                                    "Text changed for workspace '{}', new temp_hotkey: {}",
                                    workspace.name, temp_hotkey
                                );

                                // Reset validation result on text change
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp::<Option<bool>>(id,None)
                                });
                            }

                            let validation_result = match workspace.set_hotkey(&temp_hotkey) {
                                Ok(_) => {
                                    // info!(
                                    //     "Validation succeeded for workspace '{}': {}",
                                    //     workspace.name, temp_hotkey
                                    // );
                                    Some(true)
                                }
                                Err(err) => {
                                    warn!(
                                        "Validation failed for workspace '{}': {}",
                                        workspace.name, err
                                    );
                                    Some(false)
                                }
                            };

                            // Display validation result indicator
                            match validation_result {
                                Some(true) => ui.colored_label(Color32::GREEN, "Valid"),
                                Some(false) => ui.colored_label(Color32::RED, "Invalid"),
                                None => ui.label("Awaiting validation..."),
                            }
                        });

                        let mut window_to_delete = None;
                        for (j, window) in workspace.windows.iter_mut().enumerate() {
                            let hwnd = HWND(window.id as *mut std::ffi::c_void); // Move HWND declaration outside the loop
                            let exists = unsafe { IsWindow(hwnd).as_bool() };    // Check if the window exists
                        
                            ui.horizontal(|ui| {
                                ui.label(&window.title);
                        
                                if ui.button("Delete").clicked() {
                                    window_to_delete = Some(j);
                                    info!("Deleting window '{}'", window.title);
                                }

                                // Add the colored indicator for HWND validity
                                if exists {
                                    ui.colored_label(egui::Color32::GREEN, format!("HWND: {:?}", window.id));
                                } else {
                                    ui.colored_label(egui::Color32::RED, format!("HWND: {:?}", window.id));
                                }
                            });
                        
                            ui.horizontal(|ui| {
                                ui.label("Home:");
                                ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
                                ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
                                ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
                                ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));
                        
                                if ui.button("Capture Home").clicked() {
                                    if let Ok((x, y, w, h)) = get_window_position(hwnd) {
                                        window.home = (x, y, w, h);
                                        info!(
                                            "Captured window position for Home using window ID {:?}: {:?}",
                                            window.id, window.home
                                        );
                                    } else {
                                        warn!(
                                            "Failed to capture window position for Home using window ID {:?}",
                                            window.id
                                        );
                                    }
                                }
                        
                                if ui.button("Move to Home").clicked() {
                                    if let Err(e) = move_window(hwnd, window.home.0, window.home.1, window.home.2, window.home.3) {
                                        warn!("Error moving window '{}': {}", window.title, e);
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
                                    if let Ok((x, y, w, h)) = get_window_position(hwnd) {
                                        window.target = (x, y, w, h);
                                        info!(
                                            "Captured window position for Target using window ID {:?}: {:?}",
                                            window.id, window.target
                                        );
                                    } else {
                                        warn!(
                                            "Failed to capture window position for Target using window ID {:?}",
                                            window.id
                                        );
                                    }
                                }
                        
                                if ui.button("Move to Target").clicked() {
                                    if let Err(e) = move_window(hwnd, window.target.0, window.target.1, window.target.2, window.target.3) {
                                        warn!("Error moving window '{}': {}", window.title, e);
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

                        ui.horizontal(|ui| {
                            // Checkbox for "Disable"
                            ui.checkbox(&mut workspace.disabled, "Disable Workspace");
                    
                            if workspace.disabled {
                                unregister_hotkey(i as i32); // Unregister hotkey if disabled
                            } else if let Some(hotkey) = &workspace.hotkey {
                                register_hotkey(i as i32, hotkey); // Re-register hotkey if re-enabled
                            }

                            if ui.button("Delete Workspace").clicked() {
                                // Temporary comment: Add confirmation dialog before deleting the workspace
                                let confirmation_message = format!(
                                    "Are you sure you want to delete the workspace \n'{}'?\n\nThis action cannot be undone.",
                                    workspace.name
                                );
                                if show_confirmation_box(&confirmation_message, "Confirm Deletion") {
                                    workspace_to_delete = Some(i);
                                    info!("Deleting workspace '{}'.", workspace.name);
                                }
                            }
                        });

                        ui.separator();
        
                    });
                    if header_response.header_response.hovered() && ui.input(|i| i.pointer.secondary_clicked()) {
                        // Right-click detected on header
                        is_renaming = true;
                        ui.memory_mut(|mem| mem.data.insert_temp(header_id, is_renaming));
                    }
    
                    // Show a popup window for renaming the workspace
                    if is_renaming {
                        egui::Window::new("Rename Workspace")
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]) // Center the popup
                            .collapsible(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                ui.label("Enter a new name for the workspace:");
                                let response = ui.text_edit_singleline(&mut new_name);
    
                                if response.changed() {
                                    ui.memory_mut(|mem| {
                                        mem.data.insert_temp(header_id.with("wrkspce_name"), new_name.clone());
                                    });
                                }
    
                                ui.horizontal(|ui| {
                                    if ui.button("Ok").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                        // Save the new name and close the popup
                                        workspace.name = new_name.clone();
                                        is_renaming = false;
                                        ui.memory_mut(|mem| mem.data.insert_temp(header_id, is_renaming));
                                    }
    
                                    if ui.button("Cancel").clicked() {
                                        // Cancel renaming and close the popup
                                        is_renaming = false;
                                        ui.memory_mut(|mem| mem.data.insert_temp(header_id, is_renaming));
                                    }
                                });
                            });
                    }
                });
            }//Per Workspace

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

impl App {
    /// Validates all hotkeys associated with the workspaces during application startup.
    ///
    /// - Ensures that hotkeys are valid and registerable.
    /// - Logs the status of each hotkey validation.
    ///
    /// This function is intended to run only once at startup.
    fn validate_initial_hotkeys(&self) {
        let mut initial_validation_done = self.initial_validation_done.lock().unwrap();
        if !*initial_validation_done {
            let mut workspaces = self.workspaces.lock().unwrap();
            for workspace in &mut *workspaces {
                if let Some(hotkey) = workspace.hotkey.clone() {
                    if workspace.set_hotkey(&hotkey).is_ok() {
                        info!("Initial validation succeeded for hotkey '{}'.", hotkey);
                    } else {
                        warn!("Initial validation failed for hotkey '{}'.", hotkey);
                    }
                }
            }
            *initial_validation_done = true; // Ensure this runs only once
        }
    }
}

/// Checks for pressed hotkeys and toggles the associated workspaces.
///
/// - Iterates through all registered workspaces.
/// - Skips disabled workspaces.
/// - Toggles windows for any workspace whose hotkey is pressed.
///
/// # Arguments
/// * `app` - The application state encapsulated in an `App` struct.
fn check_hotkeys(app: &App) {
    let mut workspaces_to_toggle = Vec::new();
    let workspaces = app.workspaces.lock().unwrap();

    for (i, workspace) in workspaces.iter().enumerate() {

        if workspace.disabled {
            continue;
        }

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
