use dioxus::prelude::*;
use std::env;
use std::path::PathBuf;

mod platform;
mod ui;

use markdown_neuraxis_config::Config;
use ui::App;
use ui::components::{ErrorScreen, SetupScreen};

const SOLARIZED_LIGHT_CSS: &str = include_str!("assets/solarized-light.css");

/// Application error information for display to users
#[derive(Clone, Debug)]
pub struct AppError {
    pub title: String,
    pub message: String,
    pub details: Option<String>,
}

/// Application state for the root component
#[derive(Clone, Debug)]
enum AppState {
    /// No config found, show setup screen
    NeedsSetup,
    /// Config loaded successfully, show main app
    Ready(PathBuf),
    /// Error occurred during initialization
    Error(AppError),
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("markdown-neuraxis starting up!");

    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_window_config())
        .launch(app_root);
}

/// Determine initial app state from CLI args and config
fn get_initial_state() -> AppState {
    let args_count = env::args().count();

    // Check for CLI argument first
    if args_count == 2 {
        let args: Vec<String> = env::args().collect();
        let path = PathBuf::from(&args[1]);
        log::info!("Using notes path from CLI argument: {}", path.display());
        return AppState::Ready(path);
    }

    if args_count > 2 {
        return AppState::Error(AppError {
            title: "Invalid Arguments".to_string(),
            message: "Too many arguments provided".to_string(),
            details: Some("Usage: markdown-neuraxis [notes-folder-path]".to_string()),
        });
    }

    // No CLI argument - check config file
    log::info!("No CLI argument provided, checking config file");
    match Config::load() {
        Ok(Some(config)) => {
            log::info!(
                "Loaded notes path from config: {}",
                config.notes_path.display()
            );
            AppState::Ready(config.notes_path)
        }
        Ok(None) => {
            log::info!("No config file found, showing setup screen");
            AppState::NeedsSetup
        }
        Err(e) => {
            log::error!("Config::load() failed with error: {e}");
            AppState::Error(AppError {
                title: "Configuration Error".to_string(),
                message: "Failed to load configuration file".to_string(),
                details: Some(e.to_string()),
            })
        }
    }
}

fn app_root() -> Element {
    log::info!("app_root() called");

    let mut app_state = use_signal(get_initial_state);

    match app_state.read().clone() {
        AppState::NeedsSetup => {
            log::info!("Showing setup screen");
            rsx! {
                style { {SOLARIZED_LIGHT_CSS} }
                SetupScreen {
                    on_complete: move |path: PathBuf| {
                        log::info!("Setup complete, transitioning to app with path: {}", path.display());
                        app_state.set(AppState::Ready(path));
                    }
                }
            }
        }
        AppState::Ready(path) => {
            log::info!(
                "app_root() creating App component with path: {}",
                path.display()
            );
            rsx! {
                App { notes_path: path }
            }
        }
        AppState::Error(error) => {
            log::error!("app_root() error: {} - {}", error.title, error.message);
            rsx! {
                style { {SOLARIZED_LIGHT_CSS} }
                ErrorScreen {
                    title: error.title,
                    message: error.message,
                    details: error.details,
                }
            }
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
