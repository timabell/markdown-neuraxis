use crate::platform::{
    StoragePermissionStatus, check_storage_permission, request_storage_permission,
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
pub fn SetupScreen(on_complete: EventHandler<PathBuf>) -> Element {
    let mode = use_signal(|| SetupMode::Choosing);
    let mut path_input = use_signal(|| DEFAULT_NEW_PATH.to_string());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_saving = use_signal(|| false);
    let permission_status = use_signal(|| StoragePermissionStatus::Granted);

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
        let mut permission_status = permission_status;
        move |_| {
            // Check storage permission before allowing existing folder selection
            let status = check_storage_permission();
            log::info!("Storage permission status: {status:?}");
            permission_status.set(status);

            path_input.set(String::new());
            error_message.set(None);
            mode.set(SetupMode::ExistingFolder);
        }
    };

    let handle_request_permission = {
        let mut permission_status = permission_status;
        move |_| {
            log::info!("Requesting storage permission...");
            request_storage_permission();
            // Note: User will return from settings, we'll re-check on next action
            // For now, set to Denied so UI shows they need to check again
            permission_status.set(StoragePermissionStatus::Denied);
        }
    };

    let handle_recheck_permission = {
        let mut permission_status = permission_status;
        move |_| {
            let status = check_storage_permission();
            log::info!("Re-checked permission status: {status:?}");
            permission_status.set(status);
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
        let current_mode = *mode.read();
        move |_| {
            let path_str = path_input.read().clone();
            let path = PathBuf::from(&path_str);

            if path_str.trim().is_empty() {
                error_message.set(Some("Please enter a path".to_string()));
                return;
            }

            is_saving.set(true);
            error_message.set(None);

            match current_mode {
                SetupMode::NewFolder => {
                    if path.exists() {
                        error_message.set(Some(
                            "This folder already exists. Use 'existing folder' option or choose a different path.".to_string(),
                        ));
                        is_saving.set(false);
                        return;
                    }

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
            };

            match config.save() {
                Ok(()) => {
                    log::info!("Config saved successfully");
                    on_complete.call(path);
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

                SetupMode::NewFolder => rsx! {
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
                },

                SetupMode::ExistingFolder => rsx! {
                    match *permission_status.read() {
                        StoragePermissionStatus::Granted => rsx! {
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
                        },
                        StoragePermissionStatus::Denied | StoragePermissionStatus::NeedsSettingsIntent => rsx! {
                            div {
                                class: "permission-notice",
                                p {
                                    class: "permission-title",
                                    "Storage Permission Required"
                                }
                                p {
                                    "To access existing folders, the app needs permission to read files on your device."
                                }
                                p {
                                    class: "permission-instructions",
                                    if *permission_status.read() == StoragePermissionStatus::NeedsSettingsIntent {
                                        "On Android 11+, you need to enable 'All files access' in Settings."
                                    } else {
                                        "Please grant storage permission in Settings."
                                    }
                                }
                            }

                            div {
                                class: "setup-buttons",
                                button {
                                    class: "setup-btn back",
                                    onclick: handle_back,
                                    "Back"
                                }
                                button {
                                    class: "setup-btn secondary",
                                    onclick: handle_request_permission,
                                    "Open Settings"
                                }
                                button {
                                    class: "setup-btn submit",
                                    onclick: handle_recheck_permission,
                                    "I've granted permission"
                                }
                            }
                        },
                    }
                },
            }
        }
    }
}
