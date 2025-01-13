use crate::hotkey_manager::HotkeyManager;
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
    pub hotkey_manager: Arc<HotkeyManager>,
}

pub struct WorkspaceControlContext<'a> {
    pub workspace_to_delete: &'a mut Option<usize>,
    pub move_up_index: &'a mut Option<usize>,
    pub move_down_index: &'a mut Option<usize>,
    pub workspaces_len: usize,
    pub index: usize,
}

//
/// This function is responsible for:
/// - Loading existing workspace configurations from a JSON file.
/// - Validating and registering hotkeys for the workspaces.
/// - Spawning a background thread to monitor hotkey presses.
/// - Initializing and running the GUI using the `eframe` framework.
///
/// # Example
/// ```rust
/// let app = App {
///     app_title_name: "Multi Manager".to_string(),
///     workspaces: Arc::new(Mutex::new(Vec::new())),
///     last_hotkey_info: Arc::new(Mutex::new(None)),
///     hotkey_promise: Arc::new(Mutex::new(None)),
///     initial_validation_done: Arc::new(Mutex::new(false)),
///     registered_hotkeys: Arc::new(Mutex::new(HashMap::new())),
/// };
/// run_gui(app);
/// ```
///
/// # Dependencies
/// - `eframe` for GUI rendering.
/// - `poll_promise` for asynchronous hotkey monitoring.
/// - `image` for loading the application icon.
///
/// # Parameters
/// - `app: App`: An instance of the `App` struct containing the application's state.
///
/// # Behavior
/// - Loads workspaces from the `workspaces.json` file.
/// - Starts a background thread for checking hotkey presses.
/// - Configures the GUI with a custom application icon and launches it.
///
/// # Side Effects
/// - Reads from the `workspaces.json` file to load saved configurations.
/// - Registers hotkeys and logs any failures during the process.
/// - Spawns a background thread that continuously monitors hotkeys.
///
/// # Error Conditions
/// - Logs and exits if the GUI fails to initialize or run.
/// - Logs errors if the `workspaces.json` file is missing or contains invalid data.
///
/// # Notes
/// - The background thread runs indefinitely, polling for hotkey presses every 100 milliseconds.
/// - Ensure that the `workspaces.json` file exists and is writable to preserve state.
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
    /// Renders the header section of the application's GUI.
    ///
    /// This function displays:
    /// - The application's title.
    /// - Buttons for saving workspaces and adding a new workspace.
    ///
    /// # Behavior
    /// - The "Save Workspaces" button triggers saving the current workspaces to a file.
    /// - The "Add New Workspace" button creates a new workspace with a default name and adds it to the list.
    ///
    /// # Example
    /// ```rust
    /// let mut save_flag = false;
    /// let mut new_workspace = None;
    /// let app = App {
    ///     app_title_name: "Multi Manager".to_string(),
    ///     workspaces: Arc::new(Mutex::new(Vec::new())),
    ///     ..Default::default()
    /// };
    /// egui::CentralPanel::default().show(&ctx, |ui| {
    ///     app.render_header(ui, &mut save_flag, &mut new_workspace);
    /// });
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the header.
    /// - `save_flag: &mut bool`: A flag that is set to `true` when the "Save Workspaces" button is clicked.
    /// - `new_workspace: &mut Option<Workspace>`: A mutable reference to store a newly created workspace.
    ///
    /// # Side Effects
    /// - Sets the `save_flag` to `true` when the "Save Workspaces" button is clicked.
    /// - Adds a new workspace to `new_workspace` when the "Add New Workspace" button is clicked.
    ///
    /// # Notes
    /// - The new workspace is initialized with a default name based on the current number of workspaces.
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
                    hotkey_manager: default_hotkey_manager(),
                });
            }
        });
    }
    /// Renders the list of workspaces in the application's GUI.
    ///
    /// This function displays each workspace as a collapsible header, allowing users to view and edit details.
    /// It also provides controls for reordering and deleting workspaces.
    ///
    /// # Behavior
    /// - Displays workspaces in a scrollable area.
    /// - Allows workspaces to be moved up or down in the list.
    /// - Allows individual workspaces to be deleted with confirmation.
    /// - Each workspace's details are rendered using the `Workspace` struct's `render_details` method.
    ///
    /// # Example
    /// ```rust
    /// let mut workspace_to_delete = None;
    /// app.render_workspace_list(ui, &mut workspace_to_delete);
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the workspace list.
    /// - `workspace_to_delete: &mut Option<usize>`: A mutable reference to the index of the workspace to be deleted.
    ///
    /// # Side Effects
    /// - Modifies the workspace list by deleting or reordering items.
    /// - Updates the indices of the workspaces when reordered.
    ///
    /// # Notes
    /// - The list is displayed within a scrollable area to handle large numbers of workspaces.
    /// - Moving a workspace up or down swaps it with the adjacent workspace.
    /// - Deleting a workspace removes it from the list and requires user confirmation.
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

    /// Renders the controls for managing individual workspaces.
    ///
    /// This function provides UI elements for:
    /// - Disabling/enabling a workspace.
    /// - Moving a workspace up or down in the list.
    /// - Deleting a workspace with confirmation.
    ///
    /// # Behavior
    /// - Displays a checkbox for toggling the workspace's "disabled" state.
    /// - Provides buttons to move the workspace up or down in the list.
    /// - Provides a "Delete Workspace" button with a confirmation dialog.
    ///
    /// # Example
    /// ```rust
    /// let mut context = WorkspaceControlContext {
    ///     workspace_to_delete: &mut None,
    ///     move_up_index: &mut None,
    ///     move_down_index: &mut None,
    ///     workspaces_len: 3,
    ///     index: 1,
    /// };
    /// app.render_workspace_controls(ui, &mut workspace, &mut context);
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the controls.
    /// - `workspace: &mut Workspace`: A mutable reference to the workspace being managed.
    /// - `context: &mut WorkspaceControlContext`: A struct containing metadata and state for managing the workspace.
    ///
    /// # Side Effects
    /// - Updates the workspace's `disabled` state.
    /// - Modifies the context's `workspace_to_delete`, `move_up_index`, or `move_down_index` based on user actions.
    ///
    /// # Notes
    /// - Disabling a workspace prevents it from being activated via hotkeys.
    /// - Moving a workspace up or down affects its order in the workspace list.
    /// - The "Delete Workspace" button requires user confirmation and updates the `workspace_to_delete` context.
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

    /// Saves the current list of workspaces to a JSON file.
    ///
    /// This function serializes the list of workspaces and writes it to the specified file.
    /// It is typically called when the "Save Workspaces" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Serializes the `workspaces` into a JSON string using `serde_json`.
    /// - Writes the serialized data to `workspaces.json`.
    /// - Logs a success message upon completion.
    ///
    /// # Example
    /// ```rust
    /// app.save_workspaces();
    /// ```
    ///
    /// # Side Effects
    /// - Creates or overwrites the `workspaces.json` file with the current state of the workspaces.
    ///
    /// # Notes
    /// - This function relies on the `serde_json` crate for serialization.
    /// - Errors during file creation or writing are logged but not returned.
    ///
    /// # Dependencies
    /// - Calls `save_workspaces` function in `workspace.rs` for actual file operations.
    ///
    /// # Logs
    /// - Logs a message when the workspaces are successfully saved.
    /// - Logs an error message if file creation or writing fails.
    fn save_workspaces(&self) {
        let workspaces = self.workspaces.lock().unwrap();
        save_workspaces(&workspaces, "workspaces.json");
        info!("Workspaces saved successfully.");
    }

    /// Adds a new workspace to the list of workspaces.
    ///
    /// This function appends a new `Workspace` instance to the list.
    /// Typically used when the "Add New Workspace" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Locks the `workspaces` mutex to modify the list.
    /// - Adds the provided `Workspace` to the end of the list.
    ///
    /// # Example
    /// ```rust
    /// let new_workspace = Workspace {
    ///     name: "New Workspace".to_string(),
    ///     hotkey: None,
    ///     windows: Vec::new(),
    ///     disabled: false,
    ///     valid: false,
    /// };
    /// app.add_workspace(new_workspace);
    /// ```
    ///
    /// # Parameters
    /// - `workspace: Workspace`: The workspace instance to be added.
    ///
    /// # Side Effects
    /// - Modifies the `workspaces` list by adding a new workspace.
    ///
    /// # Notes
    /// - The function does not perform any validation or registration of hotkeys for the new workspace.
    /// - Any changes made to the workspace list are not persisted to disk until `save_workspaces` is called.
    fn add_workspace(&self, workspace: Workspace) {
        let mut workspaces = self.workspaces.lock().unwrap();
        workspaces.push(workspace);
    }

    /// Deletes a workspace from the list by its index.
    ///
    /// This function removes a workspace from the `workspaces` list, typically called
    /// when the "Delete Workspace" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Locks the `workspaces` mutex to modify the list.
    /// - Removes the workspace at the specified index from the list.
    ///
    /// # Parameters
    /// - `index: usize`: The zero-based index of the workspace to delete.
    ///
    /// # Example
    /// ```rust
    /// app.delete_workspace(2);
    /// ```
    ///
    /// # Side Effects
    /// - Modifies the `workspaces` list by removing the specified workspace.
    /// - Any changes made to the workspace list are not persisted to disk until `save_workspaces` is called.
    ///
    /// # Notes
    /// - If the `index` is out of bounds, the function will panic as it directly calls `Vec::remove`.
    /// - This function does not unregister any associated hotkeys or clean up other resources.
    ///
    /// # Error Conditions
    /// - Panics if the `index` is greater than or equal to the length of the `workspaces` list.
    fn delete_workspace(&self, index: usize) {
        let mut workspaces = self.workspaces.lock().unwrap();
        workspaces.remove(index);
    }

    /// Validates and registers hotkeys for all workspaces during initialization.
    ///
    /// This function ensures that all valid hotkeys associated with workspaces are registered
    /// at the start of the application. It prevents re-validation by using a flag stored
    /// in `initial_validation_done`.
    ///
    /// # Behavior
    /// - Checks if initial validation has already been done using the `initial_validation_done` flag.
    /// - Iterates through all workspaces and attempts to register their hotkeys.
    /// - Logs a warning if a hotkey fails to register.
    /// - Marks the validation as complete after processing all workspaces.
    ///
    /// # Dependencies
    /// - Uses the `register_hotkey` function from `window_manager.rs`.
    ///
    /// # Parameters
    /// - None.
    ///
    /// # Example
    /// ```rust
    /// app.validate_initial_hotkeys();
    /// ```
    ///
    /// # Side Effects
    /// - Registers all valid hotkeys for the existing workspaces.
    /// - Updates the `initial_validation_done` flag to `true`.
    ///
    /// # Notes
    /// - This function is called during the initial setup of the GUI in `run_gui`.
    /// - If a hotkey is invalid or fails to register, it logs a warning but continues processing other workspaces.
    ///
    /// # Logs
    /// - Logs success or failure messages for each hotkey registration.
    ///
    /// # Error Conditions
    /// - None. Errors during hotkey registration are logged but not propagated.
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
