use crate::gui::App;
use crate::window_manager::get_window_position;
use crate::window_manager::listen_for_keys_with_dialog_and_window;
use crate::window_manager::move_window;
use crate::window_manager::register_hotkey;
use eframe::egui;
use log::debug;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use windows::Win32::Foundation::HWND;
use crate::window_manager::*;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

/// Represents a workspace, which groups multiple windows and allows toggling between specific positions.
///
/// # Fields
/// - `name`: The name of the workspace.
/// - `hotkey`: An optional hotkey assigned to the workspace for activation.
/// - `windows`: A list of windows belonging to this workspace.
/// - `disabled`: A flag indicating whether the workspace is disabled.
#[derive(Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub hotkey: Option<String>,
    pub windows: Vec<Window>,
    pub disabled: bool,
    pub valid: bool,
}

impl Workspace {
    /// Sets the hotkey for the workspace.
    ///
    /// Validates the provided hotkey and registers it for the workspace if valid.
    ///
    /// # Arguments
    /// - `hotkey`: The key combination to assign as the workspace hotkey (e.g., "Ctrl+Alt+H").
    ///
    /// # Returns
    /// - `Ok(())` if the hotkey is valid and successfully set.
    /// - `Err` with an error message if the hotkey is invalid.
    ///
    /// # Example
    /// ```
    /// let mut workspace = Workspace::new("Example");
    /// if let Err(e) = workspace.set_hotkey("Ctrl+Shift+P") {
    ///     println!("Failed to set hotkey: {}", e);
    /// }
    /// ```
    pub fn set_hotkey(&mut self, hotkey: &str) -> Result<(), String> {
        if is_valid_key_combo(hotkey) {
            self.hotkey = Some(hotkey.to_string());
            Ok(())
        } else {
            Err(format!("Invalid hotkey: '{}'", hotkey))
        }
    }
    /// Returns the header text with color coding based on the workspace state.
    pub fn get_header_text(&self) -> egui::RichText {
        if self.disabled {
            egui::RichText::new(&self.name).color(egui::Color32::ORANGE)
        } else if self.valid {
            egui::RichText::new(&self.name).color(egui::Color32::GREEN)
        } else {
            egui::RichText::new(&self.name).color(egui::Color32::RED)
        }
    }

    /// Renders the workspace details, such as hotkey and windows.
    pub fn render_details(&mut self, ui: &mut egui::Ui) {
        // Hotkey section
        ui.horizontal(|ui| {
            ui.label("Hotkey:");
            let mut temp_hotkey = self.hotkey.clone().unwrap_or_else(|| "None".to_string());
            debug!("temp hotkey before edit: {}", temp_hotkey);
            if ui.text_edit_singleline(&mut temp_hotkey).changed() {
                match self.set_hotkey(&temp_hotkey) {
                    Ok(_) => {
                        self.hotkey = Some(temp_hotkey); // Update the workspace's hotkey
                        ui.colored_label(egui::Color32::GREEN, "Valid");
                        debug!(
                            "Hotkey updated to: {}",
                            self.hotkey.as_deref().unwrap_or("None")
                        );
                    }
                    Err(err) => {
                        ui.colored_label(egui::Color32::RED, "Invalid");
                        debug!("Hotkey validation failed: {}", err);
                    }
                }
            }
        });

        // Render windows
        let mut window_to_delete = None;
        for (i, window) in self.windows.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // Display window title
                ui.label(&window.title);

                // Add delete button
                if ui.button("Delete").clicked() {
                    window_to_delete = Some(i);
                }

                let exists = unsafe { IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool() }; 
                // Add the colored indicator for HWND validity
                if exists {
                    // Define the label and capture its response
                    let label_response = ui.colored_label(
                        egui::Color32::GREEN,
                        format!("HWND: {:?}", window.id),
                    );
                
                    // Create a unique ID for the popup menu
                    let popup_id = egui::Id::new(format!("hwnd_context_menu_workspace_{}_{}", i,window.id));

                
                    // Handle right-click to toggle popup visibility
                    if label_response.hovered() && ui.input(|i| i.pointer.secondary_clicked()) && !ui.memory(|mem| mem.is_popup_open(popup_id)) {
                        ui.memory_mut(|mem| mem.open_popup(popup_id));
                        }
                
                    // Render the popup menu if it's open
                    egui::popup::popup_below_widget(
                        ui,
                        popup_id,
                        &label_response, // Pass the label_response here
                        egui::PopupCloseBehavior::CloseOnClickOutside, // Auto-close on outside click
                        |ui| {
                            ui.label("Options:");
                
                            // Add the "Force Recapture" button
                            if ui.button("Force Recapture").clicked() {
                                info!("Force Recapture triggered for HWND: {:?}", window.id);
                                if let Some("Enter") = listen_for_keys_with_dialog() {
                                    if let Some((new_hwnd, new_title)) = get_active_window() {
                                        // Update the HWND and title
                                        window.id = new_hwnd.0 as usize;
                                        window.title = new_title;
                                        info!(
                                            "Force Recaptured window '{}', new HWND: {:?}",
                                            window.title, new_hwnd
                                        );
                                    } else {
                                        warn!("Force Recapture canceled or no active window detected.");
                                    }
                                }
                    
                                // Explicitly close the popup after the action
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        },
                    );
                    
        } else {
            ui.colored_label(egui::Color32::RED, format!("HWND: {:?}", window.id));
            if ui.button("Recapture").clicked() {
                if let Some("Enter") = listen_for_keys_with_dialog() {
                    if let Some((new_hwnd, new_title)) = get_active_window() {
                        // Update the invalid window with the new HWND but retain home/target
                        window.id = new_hwnd.0 as usize;
                        window.title = new_title;
                        info!(
                            "Recaptured window '{}', new HWND: {:?}",
                            window.title, new_hwnd
                            );
                        } else {
                            warn!("Recapture canceled or no active window detected.");
                        }
                    }
                }
            }
                
            });

            // Render controls for individual window
            render_window_controls(ui, window);
        }

        if let Some(index) = window_to_delete {
            self.windows.remove(index);
        }

        // Capture active window button
        if ui.button("Capture Active Window").clicked() {
            if let Some(("Enter", hwnd, title)) = listen_for_keys_with_dialog_and_window() {
                self.windows.push(Window {
                    id: hwnd.0 as usize,
                    title,
                    home: (0, 0, 800, 600),
                    target: (0, 0, 800, 600),
                    valid: true,
                });
            }
        }
    }
}

/// Renders the controls for a specific window within the workspace.
pub fn render_window_controls(ui: &mut egui::Ui, window: &mut Window) {
    // Home position controls
    ui.horizontal(|ui| {
        ui.label("Home:");
        ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
        ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
        ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
        ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));
        if ui.button("Capture Home").clicked() {
            if let Ok((x, y, w, h)) = get_window_position(HWND(window.id as *mut _)) {
                window.home = (x, y, w, h);
            }
        }
        if ui.button("Move to Home").clicked() {
            if let Err(e) = move_window(
                HWND(window.id as *mut _),
                window.home.0,
                window.home.1,
                window.home.2,
                window.home.3,
            ) {
                warn!("Failed to move window to home: {}", e);
            }
        }
    });

    // Target position controls
    ui.horizontal(|ui| {
        ui.label("Target:");
        ui.add(egui::DragValue::new(&mut window.target.0).prefix("x: "));
        ui.add(egui::DragValue::new(&mut window.target.1).prefix("y: "));
        ui.add(egui::DragValue::new(&mut window.target.2).prefix("w: "));
        ui.add(egui::DragValue::new(&mut window.target.3).prefix("h: "));
        if ui.button("Capture Target").clicked() {
            if let Ok((x, y, w, h)) = get_window_position(HWND(window.id as *mut _)) {
                window.target = (x, y, w, h);
            }
        }
        if ui.button("Move to Target").clicked() {
            if let Err(e) = move_window(
                HWND(window.id as *mut _),
                window.target.0,
                window.target.1,
                window.target.2,
                window.target.3,
            ) {
                warn!("Failed to move window to target: {}", e);
            }
        }
    });
}
/// Represents a window tracked within a workspace.
///
/// # Fields
/// - `id`: The unique identifier (HWND) of the window.
/// - `title`: The title of the window.
/// - `home`: The home position `(x, y, width, height)` of the window.
/// - `target`: The target position `(x, y, width, height)` of the window.
#[derive(Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: usize,
    pub title: String,
    pub home: (i32, i32, i32, i32),
    pub target: (i32, i32, i32, i32),
    pub valid: bool,
}

/// Validates if a key combination string is in a valid format.
///
/// # Arguments
/// - `input`: The key combination string to validate (e.g., "Ctrl+Alt+H").
///
/// # Returns
/// - `true` if the key combination is valid.
/// - `false` otherwise.
///
/// # Example
/// ```
/// if is_valid_key_combo("Ctrl+Shift+P") {
///     println!("Valid key combo.");
/// } else {
///     println!("Invalid key combo.");
/// }
/// ```
pub fn is_valid_key_combo(input: &str) -> bool {
    let pattern = r"^(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:F(?:[1-9]|1[0-2]|1[3-9]|2[0-4])|[A-Z]|[0-9]|NUMPAD[0-9]|NUMPAD(?:MULTIPLY|ADD|SEPARATOR|SUBTRACT|DOT|DIVIDE)|UP|DOWN|LEFT|RIGHT|BACKSPACE|TAB|ENTER|PAUSE|CAPSLOCK|ESCAPE|SPACE|PAGEUP|PAGEDOWN|END|HOME|INSERT|DELETE|OEM_(?:PLUS|COMMA|MINUS|PERIOD|[1-7])|PRINTSCREEN|SCROLLLOCK|NUMLOCK|LEFT(?:SHIFT|CTRL|ALT)|RIGHT(?:SHIFT|CTRL|ALT))$";
    let re = Regex::new(pattern).unwrap();
    re.is_match(input)
}

/// Saves a list of workspaces to a JSON file.
///
/// # Arguments
/// - `workspaces`: A reference to the list of workspaces to save.
/// - `file_path`: The path to the file where the workspaces should be saved.
///
/// # Example
/// ```
/// save_workspaces(&workspaces, "workspaces.json");
/// ```/// Saves the current workspaces to a file in JSON format.
pub fn save_workspaces(workspaces: &[Workspace], file_path: &str) {
    match serde_json::to_string_pretty(workspaces) {
        Ok(json) => {
            if let Err(e) =
                File::create(file_path).and_then(|mut file| file.write_all(json.as_bytes()))
            {
                error!("Failed to save workspaces to '{}': {}", file_path, e);
            } else {
                info!("Workspaces successfully saved to '{}'.", file_path);
            }
        }
        Err(e) => {
            error!("Failed to serialize workspaces: {}", e);
        }
    }
}

/// Loads a list of workspaces from a JSON file.
///
/// This function reads a JSON file containing workspace configurations and attempts to register
/// the hotkeys associated with each workspace using the provided `App` instance. If the file does
/// not exist or contains invalid data, an empty workspace list is returned. Hotkeys that cannot be
/// registered are logged as warnings.
///
/// # Arguments
/// - `file_path`: The path to the file to load workspaces from.
/// - `app`: A reference to the `App` instance used to manage registered hotkeys.
///
/// # Returns
/// - A `Vec<Workspace>` containing the loaded workspaces, with their hotkeys registered if possible.
/// - An empty vector if the file is missing or invalid.
///
/// # Behavior
/// - If a workspace's hotkey is valid and not already registered, it is registered successfully.
/// - If a hotkey fails to register, a warning is logged but the workspace is still included in the list.
///
/// # Example
/// ```rust
/// let app = App {
///     workspaces: Arc::new(Mutex::new(Vec::new())),
///     last_hotkey_info: Arc::new(Mutex::new(None)),
///     hotkey_promise: Arc::new(Mutex::new(None)),
///     initial_validation_done: Arc::new(Mutex::new(false)),
///     registered_hotkeys: Arc::new(Mutex::new(HashMap::new())),
/// };
///
/// let workspaces = load_workspaces("workspaces.json", &app);
/// ```
pub fn load_workspaces(file_path: &str, app: &App) -> Vec<Workspace> {
    let mut content = String::new();
    match File::open(file_path) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut content) {
                error!("Failed to read file '{}': {}", file_path, e);
                return Vec::new();
            }
            match serde_json::from_str::<Vec<Workspace>>(&content) {
                Ok(mut workspaces) => {
                    info!("Successfully loaded workspaces from '{}'.", file_path);

                    for (i, workspace) in workspaces.iter_mut().enumerate() {
                        if let Some(ref hotkey) = workspace.hotkey {
                            if !register_hotkey(app, i as i32, hotkey) {
                                warn!(
                                    "Failed to register hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            } else {
                                info!(
                                    "Registered hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            }
                        }
                    }

                    workspaces
                }
                Err(e) => {
                    warn!(
                        "Failed to parse JSON in '{}': {}. Returning empty workspace list.",
                        file_path, e
                    );
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!(
                "File '{}' not found or cannot be opened: {}. Returning empty workspace list.",
                file_path, e
            );
            Vec::new()
        }
    }
}
