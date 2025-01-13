use crate::gui::App;
use crate::hotkey::Hotkey;
use crate::window_manager::get_window_position;
use crate::window_manager::listen_for_keys_with_dialog_and_window;
use crate::window_manager::move_window;
use crate::window_manager::*;
use eframe::egui;
use log::debug;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use windows::Win32::Foundation::HWND;
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
    pub hotkey: Option<Hotkey>,
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
        match Hotkey::new(hotkey) {
            Ok(new_hotkey) => {
                self.hotkey = Some(new_hotkey);
                Ok(())
            }
            Err(e) => Err(e),
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

            let mut temp_hotkey = self
                .hotkey
                .as_ref()
                .map(|h| h.key_sequence.clone())
                .unwrap_or_else(|| "None".to_string());

            if ui.text_edit_singleline(&mut temp_hotkey).changed() {
                match self.set_hotkey(&temp_hotkey) {
                    Ok(_) => {
                        let valid_label = ui.colored_label(egui::Color32::GREEN, "Valid");
                        Self::attach_context_menu(
                            ui,
                            &valid_label,
                            "Valid Hotkey Options",
                            &temp_hotkey,
                        );
                        info!("Hotkey '{}' is valid and set.", temp_hotkey);
                    }
                    Err(_) => {
                        let invalid_label = ui.colored_label(egui::Color32::RED, "Invalid");
                        Self::attach_context_menu(
                            ui,
                            &invalid_label,
                            "Invalid Hotkey Options",
                            &temp_hotkey,
                        );
                        warn!("Hotkey '{}' is invalid.", temp_hotkey);
                    }
                }
            } else if is_valid_key_combo(&temp_hotkey) {
                let valid_label = ui.colored_label(egui::Color32::GREEN, "Valid");
                Self::attach_context_menu(ui, &valid_label, "Valid Hotkey Options", &temp_hotkey);
            } else {
                let invalid_label = ui.colored_label(egui::Color32::GRAY, "Edit to validate");
                Self::attach_context_menu(
                    ui,
                    &invalid_label,
                    "Invalid Hotkey Options",
                    &temp_hotkey,
                );
            }
        });

        // Create a copy of windows for iteration
        let windows: Vec<_> = self.windows.iter_mut().collect();
        let mut window_to_delete = None;

        for (i, window) in windows.into_iter().enumerate() {
            ui.horizontal(|ui| {
                // Display window title
                ui.label(&window.title);

                // Add delete button
                if ui.button("Delete").clicked() {
                    window_to_delete = Some(i);
                }

                // Handle HWND validity and right-click menu for individual windows
                let exists =
                    unsafe { IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool() };
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

    /// Attaches a context menu to a UI widget.
    ///
    /// This function creates a context menu (popup) that appears when the user right-clicks
    /// on the provided widget. The menu displays the specified options and can trigger
    /// actions based on the selected option.
    ///
    /// # Parameters
    /// - `ui`: The egui `Ui` instance to render the context menu.
    /// - `widget_response`: The response object of the widget to which the menu is attached.
    /// - `menu_title`: The title of the context menu.
    /// - `context_info`: Additional information to display in the context menu.
    ///
    /// # Example
    /// ```
    /// attach_context_menu(ui, &response, "Hotkey Options", "Ctrl+Shift+P");
    /// ```
    pub fn attach_context_menu(
        ui: &mut egui::Ui,
        widget_response: &egui::Response,
        menu_title: &str,
        context_info: &str,
    ) {
        // Create a unique popup ID based on the menu title and context info
        let popup_id = egui::Id::new(format!("{}_{}", menu_title, context_info));

        // Open the popup when the widget is right-clicked
        if widget_response.hovered() && ui.input(|i| i.pointer.secondary_clicked()) {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        // Render the popup menu
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            widget_response,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.label(menu_title);
                ui.separator();

                if ui.button("Option 1").clicked() {
                    println!("Option 1 clicked for: {}", context_info);
                    ui.close_menu();
                }
                if ui.button("Option 2").clicked() {
                    println!("Option 2 clicked for: {}", context_info);
                    ui.close_menu();
                }
            },
        );
    }

    /// Validates the state of a workspace.
    ///
    /// This function ensures that a workspace is in a valid state by checking:
    /// - If the assigned hotkey (if any) is in a valid format.
    /// - If the workspace contains at least one valid window (an HWND that corresponds to an existing window).
    ///
    /// The `valid` field of the workspace is updated accordingly.
    ///
    /// # Behavior
    /// - Checks the validity of the hotkey using the `is_valid_key_combo` function.
    /// - Verifies the existence of at least one valid window using the Win32 API `IsWindow`.
    /// - Updates the `valid` field of the `Workspace` struct to `true` if both checks pass.
    ///
    /// # Example
    /// ```rust
    /// let mut workspace = Workspace {
    ///     name: "Example".to_string(),
    ///     hotkey: Some("Ctrl+Alt+H".to_string()),
    ///     windows: vec![Window {
    ///         id: 12345,
    ///         title: "Example Window".to_string(),
    ///         home: (0, 0, 800, 600),
    ///         target: (100, 100, 800, 600),
    ///         valid: true,
    ///     }],
    ///     disabled: false,
    ///     valid: false,
    /// };
    /// workspace.validate_workspace();
    /// assert!(workspace.valid);
    /// ```
    ///
    /// # Dependencies
    /// - Relies on `is_valid_key_combo` for hotkey validation.
    /// - Uses the Win32 API `IsWindow` to check window validity.
    ///
    /// # Parameters
    /// - No parameters. Operates directly on the instance of the `Workspace`.
    ///
    /// # Side Effects
    /// - Updates the `valid` field of the `Workspace` struct.
    ///
    /// # Notes
    /// - This function should be called whenever the state of a workspace changes (e.g., hotkey or windows are modified).
    /// - The `disabled` state does not affect validation; it is treated independently.
    pub fn validate_workspace(&mut self) {
        self.valid = {
            let hotkey_valid = self
                .hotkey
                .as_ref()
                .map_or(false, |hotkey| is_valid_key_combo(&hotkey.key_sequence));

            let any_valid_window = self.windows.iter().any(|window| unsafe {
                IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool()
            });

            hotkey_valid && any_valid_window
        };
    }
}
/// Renders the controls for managing a specific window within a workspace.
///
/// This function creates an interface for interacting with a window's position settings.
/// It allows the user to view and modify the home and target positions of the window, as well as capture or move
/// the window to these positions. Each control is laid out in a horizontal UI group, with labels, input fields,
/// and buttons.
///
/// # Behavior
/// - Provides UI elements for adjusting and capturing the window's home and target positions.
/// - Allows moving the window to the home or target position using the `move_window` function.
/// - Enables capturing the current window position using the `get_window_position` function.
///
/// # Example
/// ```rust
/// render_window_controls(ui, &mut window);
/// ```
///
/// # Dependencies
/// - Relies on the `get_window_position` function to capture the current position of the window.
/// - Uses the `move_window` function to reposition the window.
///
/// # Parameters
/// - `ui: &mut egui::Ui`: The UI context to render the controls.
/// - `window: &mut Window`: The window instance for which controls are rendered.
///
/// # Side Effects
/// - Directly modifies the `home` and `target` fields of the `Window` struct based on user interaction.
/// - Calls Win32 API functions via `move_window` and `get_window_position` to interact with system windows.
///
/// # Error Conditions
/// - If the `move_window` function fails to reposition the window, a warning is logged.
///
/// # Notes
/// - Ensure the window's HWND is valid before attempting to move or capture its position.
/// - The `home` and `target` fields represent `(x, y, width, height)` tuples defining the window's position.
///
/// # Example UI Interaction
/// - Drag inputs allow numerical adjustment of the `home` and `target` fields.
/// - Buttons trigger actions to move or capture window positions.
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
/// This function serializes the list of `Workspace` objects into a JSON string
/// and writes it to a specified file. If the file does not exist, it is created.
/// If serialization or file writing fails, appropriate error messages are logged.
///
/// # Behavior
/// - Serializes the `workspaces` list into JSON format using `serde_json`.
/// - Writes the JSON string to the specified file path.
/// - Logs success or failure of the operation.
///
/// # Example
/// ```rust
/// let workspaces = vec![Workspace {
///     name: "Workspace 1".to_string(),
///     hotkey: Some("Ctrl+Alt+1".to_string()),
///     windows: vec![],
///     disabled: false,
///     valid: true,
/// }];
///
/// save_workspaces(&workspaces, "workspaces.json");
/// ```
///
/// # Dependencies
/// - Relies on `serde_json` for serialization.
/// - Uses Rust's standard `File` and `Write` traits for file handling.
///
/// # Parameters
/// - `workspaces: &[Workspace]`: A reference to the list of `Workspace` objects to be saved.
/// - `file_path: &str`: The path to the file where the serialized data will be written.
///
/// # Side Effects
/// - Creates or overwrites the specified file with the serialized workspace data.
///
/// # Error Conditions
/// - Logs an error if:
///   - Serialization fails (e.g., due to invalid data).
///   - File creation or writing fails (e.g., due to insufficient permissions).
///
/// # Notes
/// - Ensure the `workspaces` list is properly populated before calling this function.
/// - The function does not return errors but logs them for debugging purposes.
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
/// This function reads a JSON file containing workspace configurations and deserializes it into a vector of `Workspace` objects.
/// It also attempts to register any associated hotkeys using the provided `App` instance.
///
/// # Behavior
/// - Reads the specified file and parses its contents as JSON.
/// - Registers hotkeys for each workspace if the hotkey is valid and not already registered.
/// - Logs warnings for invalid or unregistered hotkeys.
/// - If the file is missing or invalid, returns an empty list.
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
/// let workspaces = load_workspaces("workspaces.json", &app);
/// ```
///
/// # Dependencies
/// - Relies on `serde_json` for deserialization.
/// - Uses the `register_hotkey` function to attempt hotkey registration.
///
/// # Parameters
/// - `file_path: &str`: The path to the JSON file containing workspace data.
/// - `app: &App`: Reference to the `App` instance for managing hotkey registration.
///
/// # Returns
/// - A `Vec<Workspace>` containing the loaded workspaces, with hotkeys registered where possible.
///
/// # Side Effects
/// - Modifies the `registered_hotkeys` field of the `App` instance by adding valid hotkeys.
///
/// # Error Conditions
/// - Logs warnings if:
///   - The file cannot be read.
///   - The JSON is invalid or cannot be deserialized.
///   - A hotkey cannot be registered.
///
/// # Notes
/// - Ensure the file exists and is in the correct JSON format.
/// - Hotkeys that fail registration are not removed from the workspace but are logged as invalid.
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
                        if let Some(ref mut hotkey) = workspace.hotkey {
                            if !hotkey.register(app, i as i32) {
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
