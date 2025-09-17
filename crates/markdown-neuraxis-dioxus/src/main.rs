use dioxus::prelude::*;
use markdown_neuraxis_engine::io;
use std::env;
use std::path::PathBuf;
use std::process;

mod ui;

use markdown_neuraxis_config::Config;
use ui::App;

#[cfg(target_os = "android")]
fn create_default_android_config() -> PathBuf {
    let config_path = Config::config_path();
    let default_notes_path = PathBuf::from("~/markdown-neuraxis");
    let default_config = Config {
        notes_path: default_notes_path.clone(),
    };

    if let Err(e) = default_config.save() {
        eprintln!("Warning: Failed to create default config file: {e}");
        eprintln!(
            "Using temporary notes path: {}",
            default_notes_path.display()
        );
    } else {
        eprintln!("Created default config file at {}", config_path.display());
    }

    default_notes_path
}

fn main() {
    // Determine notes path from CLI args or config file
    let config_path = Config::config_path();

    let notes_path;
    let from_config;

    // On Android, env::args() can cause capacity overflow, so handle args more carefully
    #[cfg(target_os = "android")]
    let args_count = 1; // Android apps typically don't receive CLI args

    #[cfg(not(target_os = "android"))]
    let args_count = env::args().count();

    if args_count == 2 {
        // CLI argument provided - use it (only on non-Android)
        #[cfg(not(target_os = "android"))]
        {
            let args: Vec<String> = env::args().collect();
            notes_path = PathBuf::from(&args[1]);
            from_config = false;
        }
        #[cfg(target_os = "android")]
        {
            // This branch should never be reached on Android
            unreachable!("Android should not have CLI args");
        }
    } else if args_count == 1 {
        // No CLI argument - try config file
        match Config::load() {
            Ok(Some(config)) => {
                notes_path = config.notes_path;
                from_config = true;
            }
            Ok(None) => {
                #[cfg(target_os = "android")]
                {
                    notes_path = create_default_android_config();
                    from_config = true;
                }
                #[cfg(not(target_os = "android"))]
                {
                    eprintln!("Error: No notes path provided and no config file found");
                    let program_name = env::args()
                        .next()
                        .unwrap_or_else(|| "markdown-neuraxis".to_string());
                    eprintln!("Usage: {} <notes-folder-path>", program_name);
                    eprintln!("Or create a config file at {}", config_path.display());
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Error: Failed to load config file: {e}");
                let program_name = env::args()
                    .next()
                    .unwrap_or_else(|| "markdown-neuraxis".to_string());
                eprintln!("Usage: {} <notes-folder-path>", program_name);
                process::exit(1);
            }
        }
    } else {
        let program_name = env::args()
            .next()
            .unwrap_or_else(|| "markdown-neuraxis".to_string());
        eprintln!("Usage: {} [notes-folder-path]", program_name);
        process::exit(1);
    };

    // Validate notes directory using engine
    if let Err(e) = io::validate_notes_dir(&notes_path) {
        let source = if from_config {
            format!(" from config file '{}'", config_path.display())
        } else {
            String::new()
        };
        eprintln!(
            "Error: Notes path '{}'{} is invalid: {e}",
            notes_path.display(),
            source
        );
        process::exit(1);
    }

    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_window_config())
        .launch(app_root);
}

fn app_root() -> Element {
    // Re-get notes path using same logic as main
    let notes_path;

    // On Android, env::args() can cause capacity overflow, so handle args more carefully
    #[cfg(target_os = "android")]
    {
        // Android apps always use config file
        notes_path = Config::load()
            .map_err(|_| "Config file error")
            .unwrap()
            .unwrap_or_else(|| panic!("Config file not found"))
            .notes_path;
    }

    #[cfg(not(target_os = "android"))]
    {
        let args_count = env::args().count();
        notes_path = if args_count == 2 {
            let args: Vec<String> = env::args().collect();
            PathBuf::from(&args[1])
        } else {
            // No CLI argument - use config file, error if not found
            Config::load()
                .map_err(|_| "Config file error")
                .unwrap()
                .unwrap_or_else(|| panic!("Config file not found"))
                .notes_path
        };
    }

    rsx! {
        App { notes_path: notes_path }
    }
}

fn make_window_config() -> dioxus::desktop::Config {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("markdown-neuraxis")
        .with_always_on_top(false);

    Config::default().with_window(window)
}
