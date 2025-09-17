use dioxus::prelude::*;
use markdown_neuraxis_engine::io;
use std::env;
use std::path::PathBuf;
use std::process;

mod ui;

use markdown_neuraxis_config::Config;
use ui::App;

fn main() {
    // Determine notes path from CLI args or config file
    let args: Vec<String> = env::args().collect();
    let config_path = Config::config_path();

    let notes_path;
    let from_config;

    if args.len() == 2 {
        // CLI argument provided - use it
        notes_path = PathBuf::from(&args[1]);
        from_config = false;
    } else if args.len() == 1 {
        // No CLI argument - try config file
        match Config::load() {
            Ok(Some(config)) => {
                notes_path = config.notes_path;
                from_config = true;
            }
            Ok(None) => {
                eprintln!("Error: No notes path provided and no config file found");
                eprintln!("Usage: {} <notes-folder-path>", args[0]);
                eprintln!("Or create a config file at {}", config_path.display());
                process::exit(1);
            }
            Err(e) => {
                eprintln!("Error: Failed to load config file: {e}");
                eprintln!("Usage: {} <notes-folder-path>", args[0]);
                process::exit(1);
            }
        }
    } else {
        eprintln!("Usage: {} [notes-folder-path]", args[0]);
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
    let args: Vec<String> = env::args().collect();
    let notes_path = if args.len() == 2 {
        PathBuf::from(&args[1])
    } else {
        // No CLI argument - use config file, error if not found
        Config::load()
            .map_err(|_| "Config file error")
            .unwrap()
            .unwrap_or_else(|| panic!("Config file not found"))
            .notes_path
    };

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
