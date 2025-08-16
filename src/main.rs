use dioxus::prelude::*;
use markdown_neuraxis::app::ApplicationServices;
use markdown_neuraxis::presentation::App;
use std::env;
use std::path::PathBuf;

// Simple main app that creates services directly in the component
fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_window_config())
        .launch(AppRoot);
}

fn AppRoot() -> Element {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return rsx! {
            div {
                style: "padding: 20px; font-family: monospace;",
                h1 { "Error" }
                p { "Usage: {args[0]} <notes-folder-path>" }
            }
        };
    }

    let notes_path = PathBuf::from(&args[1]);
    if !notes_path.exists() || !notes_path.is_dir() {
        return rsx! {
            div {
                style: "padding: 20px; font-family: monospace;",
                h1 { "Error" }
                p { "'{args[1]}' is not a valid directory" }
            }
        };
    }

    let services = ApplicationServices::new();

    // Validate notes structure
    if let Err(e) = services.document_service.validate_notes_structure(&notes_path) {
        return rsx! {
            div {
                style: "padding: 20px; font-family: monospace;",
                h1 { "Error" }
                p { "Invalid notes structure: {e}" }
            }
        };
    }
    
    rsx! {
        App {
            services: services,
            notes_path: notes_path,
        }
    }
}

fn make_window_config() -> dioxus::desktop::Config {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("markdown-neuraxis")
        .with_always_on_top(false);

    Config::default().with_window(window)
}
