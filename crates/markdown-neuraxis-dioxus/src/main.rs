use dioxus::prelude::*;
use markdown_neuraxis_engine::io;
use std::env;
use std::path::PathBuf;

mod ui;

use markdown_neuraxis_config::Config;
use ui::App;
use ui::components::ErrorScreen;

/// Application error information for display to users
#[derive(Clone, Debug)]
pub struct AppError {
    pub title: String,
    pub message: String,
    pub details: Option<String>,
}

/// Global error state for startup errors (used to pass error info to error screen)
/// Only used on Android where stderr isn't visible to users
#[cfg(target_os = "android")]
static STARTUP_ERROR: std::sync::OnceLock<AppError> = std::sync::OnceLock::new();

/// Launch an error screen instead of the main app (Android only)
/// On desktop, errors are shown via stderr before exit
#[cfg(target_os = "android")]
fn launch_error_app(error: AppError) {
    log::error!(
        "Launching error screen: {} - {}",
        error.title,
        error.message
    );

    // Store error for the error_app_root component to access
    let _ = STARTUP_ERROR.set(error);

    dioxus::launch(error_app_root);
}

/// Root component for displaying startup errors (Android only)
#[cfg(target_os = "android")]
fn error_app_root() -> Element {
    let error = STARTUP_ERROR
        .get()
        .expect("STARTUP_ERROR must be set before launching error app");

    rsx! {
        style { {include_str!("assets/solarized-light.css")} }
        ErrorScreen {
            title: error.title.clone(),
            message: error.message.clone(),
            details: error.details.clone(),
        }
    }
}

#[cfg(target_os = "android")]
fn create_default_android_config() -> PathBuf {
    log::info!("create_default_android_config() called");

    // Use external Documents folder for notes (requires WRITE_EXTERNAL_STORAGE permission)
    let default_notes_path = PathBuf::from("/storage/emulated/0/Documents/markdown-neuraxis");
    log::info!("Default notes path: {}", default_notes_path.display());

    let default_config = Config {
        notes_path: default_notes_path.clone(),
    };

    // Config goes in app's internal storage (no permissions needed)
    let config_path = Config::config_path();
    log::info!("Config path: {}", config_path.display());

    log::info!("About to save default config");
    match default_config.save() {
        Ok(()) => {
            log::info!(
                "Successfully created default config file at {}",
                config_path.display()
            );
        }
        Err(e) => {
            log::warn!("Failed to create default config file: {e}");
            log::warn!("Will use default notes path without persisting config");
        }
    }

    log::info!(
        "create_default_android_config() returning: {}",
        default_notes_path.display()
    );
    default_notes_path
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

    // Determine notes path from CLI args or config file
    let config_path = Config::config_path();
    log::info!("Config path: {}", config_path.display());

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
            log::info!(
                "Using notes path from CLI argument: {}",
                notes_path.display()
            );
        }
        #[cfg(target_os = "android")]
        {
            // This branch should never be reached on Android
            unreachable!("Android should not have CLI args");
        }
    } else if args_count == 1 {
        // No CLI argument - try config file
        log::info!("No CLI argument provided, checking config file");
        log::info!("About to call Config::load()");
        match Config::load() {
            Ok(Some(config)) => {
                log::info!("Config::load() returned Some(config)");
                notes_path = config.notes_path;
                from_config = true;
                log::info!("Loaded notes path from config: {}", notes_path.display());
            }
            Ok(None) => {
                log::info!("Config::load() returned None - no config file found");
                #[cfg(target_os = "android")]
                {
                    log::info!("Android: calling create_default_android_config()");
                    notes_path = create_default_android_config();
                    from_config = true;
                    log::info!(
                        "Created default Android config with path: {}",
                        notes_path.display()
                    );
                }
                #[cfg(not(target_os = "android"))]
                {
                    eprintln!("Error: No notes path provided and no config file found");
                    let program_name = env::args()
                        .next()
                        .unwrap_or_else(|| "markdown-neuraxis".to_string());
                    eprintln!("Usage: {} <notes-folder-path>", program_name);
                    eprintln!("Or create a config file at {}", config_path.display());
                    std::process::exit(1);
                }
            }
            Err(e) => {
                log::error!("Config::load() failed with error: {e}");
                #[cfg(target_os = "android")]
                {
                    return launch_error_app(AppError {
                        title: "Configuration Error".to_string(),
                        message: "Failed to load configuration file".to_string(),
                        details: Some(e.to_string()),
                    });
                }
                #[cfg(not(target_os = "android"))]
                {
                    eprintln!("Error: Failed to load config file: {e}");
                    let program_name = env::args()
                        .next()
                        .unwrap_or_else(|| "markdown-neuraxis".to_string());
                    eprintln!("Usage: {} <notes-folder-path>", program_name);
                    std::process::exit(1);
                }
            }
        }
    } else {
        let program_name = env::args()
            .next()
            .unwrap_or_else(|| "markdown-neuraxis".to_string());
        eprintln!("Usage: {} [notes-folder-path]", program_name);
        std::process::exit(1);
    };

    // On Android, create the notes directory if it doesn't exist
    #[cfg(target_os = "android")]
    {
        if !notes_path.exists() {
            log::info!(
                "Notes directory doesn't exist, attempting to create: {}",
                notes_path.display()
            );
            match std::fs::create_dir_all(&notes_path) {
                Ok(()) => {
                    log::info!("Successfully created notes directory");

                    // Create a welcome file for new users
                    let welcome_path = notes_path.join("welcome.md");
                    let welcome_content = r#"# Welcome to markdown-neuraxis

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

Happy note-taking! 📝
"#;

                    match std::fs::write(&welcome_path, welcome_content) {
                        Ok(()) => log::info!("Created welcome.md file"),
                        Err(e) => log::warn!("Failed to create welcome.md: {e}"),
                    }
                }
                Err(e) => {
                    log::error!("Failed to create notes directory: {e}");
                    log::error!(
                        "This usually means the app lacks WRITE_EXTERNAL_STORAGE permission"
                    );
                    return launch_error_app(AppError {
                        title: "Storage Permission Error".to_string(),
                        message: format!(
                            "Failed to create notes directory at '{}'",
                            notes_path.display()
                        ),
                        details: Some(format!(
                            "{}\n\nThis usually means the app lacks storage permission. \
                             Please grant storage access in Settings > Apps > markdown-neuraxis > Permissions.",
                            e
                        )),
                    });
                }
            }
        }
    }

    // Validate notes directory using engine
    if let Err(e) = io::validate_notes_dir(&notes_path) {
        let source = if from_config {
            format!(" (from config file '{}')", config_path.display())
        } else {
            String::new()
        };

        log::error!("Notes path validation failed: {e}");
        log::error!("Notes path: {}", notes_path.display());

        #[cfg(target_os = "android")]
        {
            return launch_error_app(AppError {
                title: "Invalid Notes Directory".to_string(),
                message: format!("Notes path '{}'{} is invalid", notes_path.display(), source),
                details: Some(e.to_string()),
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            eprintln!(
                "Error: Notes path '{}'{} is invalid: {e}",
                notes_path.display(),
                source
            );
            std::process::exit(1);
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        log::info!("About to launch Dioxus app for desktop");
        dioxus::LaunchBuilder::desktop()
            .with_cfg(make_window_config())
            .launch(app_root);
    }

    #[cfg(target_os = "android")]
    {
        // On Android, we need to actually launch the app
        log::info!("Launching Dioxus app for Android");
        dioxus::launch(app_root);
        log::info!("Dioxus launch completed");
    }
}

fn app_root() -> Element {
    log::info!("app_root() called");

    // Re-get notes path using same logic as main
    let notes_path: Result<PathBuf, AppError>;

    // On Android, env::args() can cause capacity overflow, so handle args more carefully
    #[cfg(target_os = "android")]
    {
        // Android apps always use config file
        log::info!("Android: loading config in app_root");
        notes_path = match Config::load() {
            Ok(Some(config)) => Ok(config.notes_path),
            Ok(None) => Err(AppError {
                title: "Configuration Missing".to_string(),
                message: "No configuration file found".to_string(),
                details: Some("The app could not find its configuration file.".to_string()),
            }),
            Err(e) => Err(AppError {
                title: "Configuration Error".to_string(),
                message: "Failed to load configuration".to_string(),
                details: Some(e.to_string()),
            }),
        };
    }

    #[cfg(not(target_os = "android"))]
    {
        let args_count = env::args().count();
        notes_path = if args_count == 2 {
            let args: Vec<String> = env::args().collect();
            Ok(PathBuf::from(&args[1]))
        } else {
            // No CLI argument - use config file, error if not found
            match Config::load() {
                Ok(Some(config)) => Ok(config.notes_path),
                Ok(None) => Err(AppError {
                    title: "Configuration Missing".to_string(),
                    message: "No configuration file found".to_string(),
                    details: None,
                }),
                Err(e) => Err(AppError {
                    title: "Configuration Error".to_string(),
                    message: "Failed to load configuration".to_string(),
                    details: Some(e.to_string()),
                }),
            }
        };
    }

    match notes_path {
        Ok(path) => {
            log::info!(
                "app_root() creating App component with path: {}",
                path.display()
            );
            rsx! {
                App { notes_path: path }
            }
        }
        Err(error) => {
            log::error!("app_root() error: {} - {}", error.title, error.message);
            rsx! {
                style { {include_str!("assets/solarized-light.css")} }
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
