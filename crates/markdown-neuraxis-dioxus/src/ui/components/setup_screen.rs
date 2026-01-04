use crate::StorageLocation;
#[cfg(target_os = "android")]
use crate::platform::{
    get_folder_picker_result, is_folder_picker_complete, launch_folder_picker, reset_folder_picker,
};
use dioxus::prelude::*;
use markdown_neuraxis_config::Config;
use std::path::PathBuf;

const WELCOME_CONTENT: &str = r#"# Welcome to markdown-neuraxis

Welcome to your new markdown knowledge base!

markdown-neuraxis is an experimental local-first tool for structured thought, life organization, and personal knowledge management built on plain Markdown files.

## Getting Started

- Create new `.md` files in this folder
- Use `[[wiki-links]]` to connect your notes
- Organize with `#tags`
- Track tasks with `TODO`, `DOING`, `DONE` states

## Learn More

Visit the project on GitHub for documentation and updates:
https://github.com/tim-abell/markdown-neuraxis

Happy note-taking!
"#;

#[cfg(target_os = "android")]
const DEFAULT_NEW_PATH: &str = "/storage/emulated/0/Documents/markdown-neuraxis";
#[cfg(not(target_os = "android"))]
const DEFAULT_NEW_PATH: &str = "";

/// Setup mode - new folder or existing
#[derive(Clone, Copy, PartialEq)]
enum SetupMode {
    Choosing,
    NewFolder,
    ExistingFolder,
}

/// Setup screen shown on first run when no config exists
/// Allows the user to create a new notes folder or use an existing one
#[component]
pub fn SetupScreen(on_complete: EventHandler<StorageLocation>) -> Element {
    let mode = use_signal(|| SetupMode::Choosing);
    let mut path_input = use_signal(|| DEFAULT_NEW_PATH.to_string());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_saving = use_signal(|| false);

    // Folder picker state (Android only)
    #[cfg(target_os = "android")]
    let mut picker_active = use_signal(|| false);
    #[cfg(not(target_os = "android"))]
    let picker_active = use_signal(|| false);

    // Suppress unused warning on non-Android
    #[cfg(not(target_os = "android"))]
    let _ = &picker_active;

    let handle_new_folder = {
        let mut mode = mode;
        let mut path_input = path_input;
        let mut error_message = error_message;
        move |_| {
            path_input.set(DEFAULT_NEW_PATH.to_string());
            error_message.set(None);
            mode.set(SetupMode::NewFolder);
        }
    };

    let handle_existing_folder = {
        let mut mode = mode;
        let mut path_input = path_input;
        let mut error_message = error_message;
        move |_| {
            // SAF folder picker handles permissions automatically via URI grants
            path_input.set(String::new());
            error_message.set(None);
            mode.set(SetupMode::ExistingFolder);
        }
    };

    let handle_back = {
        let mut mode = mode;
        let mut error_message = error_message;
        move |_| {
            error_message.set(None);
            mode.set(SetupMode::Choosing);
        }
    };

    let handle_submit = {
        move |_| {
            let current_mode = *mode.read();
            let input_str = path_input.read().clone();

            if input_str.trim().is_empty() {
                error_message.set(Some("Please select a folder".to_string()));
                return;
            }

            is_saving.set(true);
            error_message.set(None);

            // On Android, check if we have a content URI (from SAF folder picker)
            #[cfg(target_os = "android")]
            let is_content_uri = input_str.starts_with("content://");
            #[cfg(not(target_os = "android"))]
            let is_content_uri = false;

            if is_content_uri {
                // Android SAF path - save URI directly, no filesystem validation needed
                // The folder picker already ensured valid selection with permissions
                let config = Config {
                    notes_path: PathBuf::new(),
                    notes_uri: Some(input_str.clone()),
                };

                match config.save() {
                    Ok(()) => {
                        log::info!("Config saved with SAF URI");
                        on_complete.call(StorageLocation::Uri(input_str));
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to save config: {e}")));
                        is_saving.set(false);
                    }
                }
                return;
            }

            // Desktop/filesystem path handling
            let path = PathBuf::from(&input_str);

            match current_mode {
                SetupMode::NewFolder => {
                    // On desktop, check if folder already exists
                    if path.exists() {
                        error_message.set(Some(
                            "This folder already exists. Use 'existing folder' option or choose a different path.".to_string(),
                        ));
                        is_saving.set(false);
                        return;
                    }

                    // Create folder if it doesn't exist
                    if let Err(e) = std::fs::create_dir_all(&path) {
                        error_message.set(Some(format!("Failed to create directory: {e}")));
                        is_saving.set(false);
                        return;
                    }

                    let welcome_path = path.join("welcome.md");
                    if let Err(e) = std::fs::write(&welcome_path, WELCOME_CONTENT) {
                        log::warn!("Failed to create welcome.md: {e}");
                    } else {
                        log::info!("Created welcome.md file");
                    }
                }
                SetupMode::ExistingFolder => {
                    if !path.exists() {
                        error_message.set(Some("This folder doesn't exist.".to_string()));
                        is_saving.set(false);
                        return;
                    }

                    if !path.is_dir() {
                        error_message.set(Some("Path exists but is not a directory".to_string()));
                        is_saving.set(false);
                        return;
                    }
                }
                SetupMode::Choosing => {
                    return;
                }
            }

            let config = Config {
                notes_path: path.clone(),
                notes_uri: None,
            };

            match config.save() {
                Ok(()) => {
                    log::info!("Config saved successfully");
                    on_complete.call(StorageLocation::Path(path));
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to save config: {e}")));
                    is_saving.set(false);
                }
            }
        }
    };

    rsx! {
        div {
            class: "setup-screen",

            h1 { "Welcome to markdown-neuraxis" }

            match *mode.read() {
                SetupMode::Choosing => rsx! {
                    p { class: "intro", "Choose how you'd like to get started:" }

                    div {
                        class: "setup-options",

                        button {
                            class: "setup-option-btn primary",
                            onclick: handle_new_folder,
                            div { class: "title", "Create new notes folder" }
                            div { class: "description", "Start fresh with a new folder and a welcome guide" }
                        }

                        button {
                            class: "setup-option-btn secondary",
                            onclick: handle_existing_folder,
                            div { class: "title", "Use existing folder" }
                            div { class: "description", "Point to a folder that already contains markdown files" }
                        }
                    }
                },

                SetupMode::NewFolder => {
                    // On Android, use native folder picker; on desktop, use text input
                    #[cfg(target_os = "android")]
                    {
                        let handle_browse = {
                            let mut picker_active = picker_active;
                            let mut error_message = error_message;
                            move |_| {
                                log::info!("Launching folder picker for new folder...");
                                if launch_folder_picker() {
                                    picker_active.set(true);
                                    error_message.set(None);
                                } else {
                                    error_message.set(Some("Failed to open folder picker".to_string()));
                                }
                            }
                        };

                        let handle_check_selection = {
                            let mut picker_active = picker_active;
                            let mut path_input = path_input;
                            let mut error_message = error_message;
                            move |_| {
                                log::info!("Checking folder picker result...");
                                if is_folder_picker_complete() {
                                    if let Some(path) = get_folder_picker_result() {
                                        log::info!("Folder picker returned: {path}");
                                        path_input.set(path);
                                        error_message.set(None);
                                    } else {
                                        log::info!("Folder picker was cancelled");
                                        error_message.set(Some("No folder was selected".to_string()));
                                    }
                                    reset_folder_picker();
                                    picker_active.set(false);
                                } else {
                                    error_message.set(Some("Please select a folder first".to_string()));
                                }
                            }
                        };

                        let path_str = path_input.read().clone();
                        let has_selection = !path_str.is_empty();
                        let is_picking = *picker_active.read();

                        rsx! {
                            p { "Select or create a folder for your notes. A welcome guide will be added." }

                            div {
                                class: "setup-form",

                                if has_selection {
                                    div {
                                        class: "selected-path",
                                        label { "Selected folder:" }
                                        p { class: "path-display", "{path_str}" }
                                    }
                                }

                                button {
                                    class: "setup-btn browse",
                                    onclick: handle_browse,
                                    disabled: *is_saving.read(),
                                    if has_selection { "Choose different location" } else { "Browse for location" }
                                }

                                if is_picking {
                                    button {
                                        class: "setup-btn secondary",
                                        onclick: handle_check_selection,
                                        "I've selected a folder"
                                    }
                                }
                            }

                            if let Some(error) = error_message.read().as_ref() {
                                p { class: "setup-error", "{error}" }
                            }

                            div {
                                class: "setup-buttons",
                                button {
                                    class: "setup-btn back",
                                    onclick: handle_back,
                                    disabled: *is_saving.read(),
                                    "Back"
                                }
                                button {
                                    class: "setup-btn submit",
                                    onclick: handle_submit,
                                    disabled: *is_saving.read() || !has_selection,
                                    if *is_saving.read() { "Creating..." } else { "Create folder" }
                                }
                            }
                        }
                    }

                    #[cfg(not(target_os = "android"))]
                    rsx! {
                        p { "Enter a path for your new notes folder. A welcome guide will be created to help you get started." }

                        div {
                            class: "setup-form",
                            label { "New folder path:" }
                            input {
                                r#type: "text",
                                value: "{path_input}",
                                oninput: move |evt| path_input.set(evt.value().clone()),
                                disabled: *is_saving.read(),
                            }
                        }

                        if let Some(error) = error_message.read().as_ref() {
                            p { class: "setup-error", "{error}" }
                        }

                        div {
                            class: "setup-buttons",
                            button {
                                class: "setup-btn back",
                                onclick: handle_back,
                                disabled: *is_saving.read(),
                                "Back"
                            }
                            button {
                                class: "setup-btn submit",
                                onclick: handle_submit,
                                disabled: *is_saving.read(),
                                if *is_saving.read() { "Creating..." } else { "Create folder" }
                            }
                        }
                    }
                },

                SetupMode::ExistingFolder => {
                    // On Android, use native folder picker; on desktop, use text input
                    // SAF folder picker handles permissions automatically via URI grants
                    #[cfg(target_os = "android")]
                    {
                        let handle_browse = {
                            let mut picker_active = picker_active;
                            let mut error_message = error_message;
                            move |_| {
                                log::info!("Launching folder picker...");
                                if launch_folder_picker() {
                                    picker_active.set(true);
                                    error_message.set(None);
                                } else {
                                    error_message.set(Some("Failed to open folder picker".to_string()));
                                }
                            }
                        };

                        let handle_check_selection = {
                            let mut picker_active = picker_active;
                            let mut path_input = path_input;
                            let mut error_message = error_message;
                            move |_| {
                                log::info!("Checking folder picker result...");
                                if is_folder_picker_complete() {
                                    if let Some(path) = get_folder_picker_result() {
                                        log::info!("Folder picker returned: {path}");
                                        path_input.set(path);
                                        error_message.set(None);
                                    } else {
                                        log::info!("Folder picker was cancelled");
                                        error_message.set(Some("No folder was selected".to_string()));
                                    }
                                    reset_folder_picker();
                                    picker_active.set(false);
                                } else {
                                    error_message.set(Some("Please select a folder first".to_string()));
                                }
                            }
                        };

                        let path_str = path_input.read().clone();
                        let has_selection = !path_str.is_empty();
                        let is_picking = *picker_active.read();

                        rsx! {
                            p { "Select the folder containing your markdown notes." }

                            div {
                                class: "setup-form",

                                if has_selection {
                                    div {
                                        class: "selected-path",
                                        label { "Selected folder:" }
                                        p { class: "path-display", "{path_str}" }
                                    }
                                }

                                button {
                                    class: "setup-btn browse",
                                    onclick: handle_browse,
                                    disabled: *is_saving.read(),
                                    if has_selection { "Choose different folder" } else { "Browse for folder" }
                                }

                                if is_picking {
                                    button {
                                        class: "setup-btn secondary",
                                        onclick: handle_check_selection,
                                        "I've selected a folder"
                                    }
                                }
                            }

                            if let Some(error) = error_message.read().as_ref() {
                                p { class: "setup-error", "{error}" }
                            }

                            div {
                                class: "setup-buttons",
                                button {
                                    class: "setup-btn back",
                                    onclick: handle_back,
                                    disabled: *is_saving.read(),
                                    "Back"
                                }
                                button {
                                    class: "setup-btn submit",
                                    onclick: handle_submit,
                                    disabled: *is_saving.read() || !has_selection,
                                    if *is_saving.read() { "Saving..." } else { "Use this folder" }
                                }
                            }
                        }
                    }

                    #[cfg(not(target_os = "android"))]
                    rsx! {
                        p { "Enter the path to your existing notes folder." }

                        div {
                            class: "setup-form",
                            label { "Existing folder path:" }
                            input {
                                r#type: "text",
                                value: "{path_input}",
                                oninput: move |evt| path_input.set(evt.value().clone()),
                                disabled: *is_saving.read(),
                            }
                        }

                        if let Some(error) = error_message.read().as_ref() {
                            p { class: "setup-error", "{error}" }
                        }

                        div {
                            class: "setup-buttons",
                            button {
                                class: "setup-btn back",
                                onclick: handle_back,
                                disabled: *is_saving.read(),
                                "Back"
                            }
                            button {
                                class: "setup-btn submit",
                                onclick: handle_submit,
                                disabled: *is_saving.read(),
                                if *is_saving.read() { "Saving..." } else { "Use this folder" }
                            }
                        }
                    }
                },
            }
        }
    }
}
