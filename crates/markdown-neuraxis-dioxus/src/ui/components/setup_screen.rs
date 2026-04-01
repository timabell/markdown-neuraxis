use crate::platform::pick_folder;
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

/// Setup screen shown on first run when no config exists
/// Allows the user to create a new notes folder or use an existing one
#[component]
pub fn SetupScreen(on_complete: EventHandler<PathBuf>) -> Element {
    let mut error_message = use_signal(|| None::<String>);
    let mut is_picking = use_signal(|| false);

    let handle_new_folder = {
        move |_| {
            is_picking.set(true);
            error_message.set(None);

            // Get documents directory as starting location
            let start_dir = dirs::document_dir();

            spawn(async move {
                let selected = pick_folder(start_dir.as_deref()).await;

                match selected {
                    Some(notes_path) => {
                        // Create welcome.md in the selected folder
                        let welcome_path = notes_path.join("welcome.md");
                        if let Err(e) = std::fs::write(&welcome_path, WELCOME_CONTENT) {
                            log::warn!("Failed to create welcome.md: {e}");
                        } else {
                            log::info!("Created welcome.md file");
                        }

                        // Save config
                        let config = Config {
                            notes_path: notes_path.clone(),
                        };

                        match config.save() {
                            Ok(()) => {
                                log::info!("Config saved successfully");
                                on_complete.call(notes_path);
                            }
                            Err(e) => {
                                error_message.set(Some(format!("Failed to save config: {e}")));
                                is_picking.set(false);
                            }
                        }
                    }
                    None => {
                        // User cancelled
                        is_picking.set(false);
                    }
                }
            });
        }
    };

    let handle_existing_folder = move |_| {
        is_picking.set(true);
        error_message.set(None);

        // Get home directory as starting location
        let start_dir = dirs::home_dir();

        spawn(async move {
            let selected = pick_folder(start_dir.as_deref()).await;

            match selected {
                Some(notes_path) => {
                    // Validate it exists and is a directory
                    if !notes_path.exists() {
                        error_message.set(Some("Selected folder doesn't exist.".to_string()));
                        is_picking.set(false);
                        return;
                    }

                    if !notes_path.is_dir() {
                        error_message.set(Some("Selected path is not a directory.".to_string()));
                        is_picking.set(false);
                        return;
                    }

                    // Save config
                    let config = Config {
                        notes_path: notes_path.clone(),
                    };

                    match config.save() {
                        Ok(()) => {
                            log::info!("Config saved successfully");
                            on_complete.call(notes_path);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to save config: {e}")));
                            is_picking.set(false);
                        }
                    }
                }
                None => {
                    // User cancelled
                    is_picking.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "setup-screen",

            h1 { "Welcome to markdown-neuraxis" }

            p { class: "intro", "Choose how you'd like to get started:" }

            div {
                class: "setup-options",

                button {
                    class: "setup-option-btn primary",
                    onclick: handle_new_folder,
                    disabled: *is_picking.read(),
                    div { class: "title", "Create new notes folder" }
                    div { class: "description", "Start fresh with a new folder and a welcome guide" }
                }

                button {
                    class: "setup-option-btn secondary",
                    onclick: handle_existing_folder,
                    disabled: *is_picking.read(),
                    div { class: "title", "Use existing folder" }
                    div { class: "description", "Point to a folder that already contains markdown files" }
                }
            }

            if let Some(error) = error_message.read().as_ref() {
                p { class: "setup-error", "{error}" }
            }
        }
    }
}
