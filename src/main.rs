use dioxus::prelude::*;
use markdown_neuraxis::{io, ui::App};
use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    // Validate CLI arguments before starting Dioxus app
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <notes-folder-path>", args[0]);
        process::exit(1);
    }

    let notes_path = PathBuf::from(&args[1]);
    if !notes_path.exists() {
        eprintln!("Error: '{}' does not exist", args[1]);
        process::exit(1);
    }

    if !notes_path.is_dir() {
        eprintln!("Error: '{}' is not a directory", args[1]);
        process::exit(1);
    }

    // Validate notes structure
    if let Err(e) = io::validate_notes_dir(&notes_path) {
        eprintln!("Error: Invalid notes structure: {e}");
        process::exit(1);
    }

    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_window_config())
        .launch(app_root);
}

fn app_root() -> Element {
    // Re-get CLI args since validation already passed
    let args: Vec<String> = env::args().collect();
    let notes_path = PathBuf::from(&args[1]);

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
