use dioxus::prelude::*;
use std::env;
use std::path::PathBuf;

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
    // Initialize logging
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("MarkdownNeuraxis"),
        );
    }

    #[cfg(not(target_os = "android"))]
    {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Set up panic hook to log panics before abort (especially useful on Android)
    #[cfg(target_os = "android")]
    {
        use markdown_neuraxis_config::ANDROID_PACKAGE_NAME;

        std::panic::set_hook(Box::new(|panic_info| {
            let msg = panic_info.to_string();
            log::error!("PANIC: {}", msg);

            // Also write to a crash log file for post-crash inspection
            let crash_path = format!("/data/data/{}/files/crash.log", ANDROID_PACKAGE_NAME);
            let crash_path = std::path::Path::new(&crash_path);
            if let Some(parent) = crash_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(crash_path, &msg);
        }));
    }

    log::info!("markdown-neuraxis starting up!");

    #[cfg(not(target_os = "android"))]
    {
        log::info!("About to launch Dioxus app for desktop");
        dioxus::LaunchBuilder::desktop()
            .with_cfg(make_window_config())
            .launch(app_root);
    }

    #[cfg(target_os = "android")]
    {
        log::info!("Launching Dioxus app for Android");
        dioxus::launch(app_root);
        log::info!("Dioxus launch completed");
    }
}

/// Determine initial app state from CLI args and config
fn get_initial_state() -> AppState {
    // On Android, env::args() can cause capacity overflow, so handle args more carefully
    #[cfg(target_os = "android")]
    let args_count = 1; // Android apps typically don't receive CLI args

    #[cfg(not(target_os = "android"))]
    let args_count = env::args().count();

    // Check for CLI argument first (desktop only)
    if args_count == 2 {
        #[cfg(not(target_os = "android"))]
        {
            let args: Vec<String> = env::args().collect();
            let path = PathBuf::from(&args[1]);
            log::info!("Using notes path from CLI argument: {}", path.display());
            return AppState::Ready(path);
        }
        #[cfg(target_os = "android")]
        {
            unreachable!("Android should not have CLI args");
        }
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
