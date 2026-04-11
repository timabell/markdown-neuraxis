use crate::platform::pick_folder;
use dioxus::prelude::*;
use markdown_neuraxis_config::Config;
use markdown_neuraxis_engine::{
    Document, FileTree, MarkdownFile, Snapshot, editing::commands::Cmd, io,
};
use relative_path::RelativePathBuf;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SOLARIZED_LIGHT_CSS: &str = include_str!("../assets/solarized-light.css");

/// Runtime error information for display in the App UI
#[derive(Clone, Debug)]
pub struct RuntimeError {
    pub message: String,
    pub details: Option<String>,
}

impl RuntimeError {
    /// Log the error and set it on the signal in one call
    pub fn log_and_set(
        error_state: &mut Signal<Option<RuntimeError>>,
        message: String,
        details: impl ToString,
    ) {
        let details = details.to_string();
        log::error!("{}: {}", message, details);
        error_state.set(Some(RuntimeError {
            message,
            details: Some(details),
        }));
    }
}

#[component]
pub fn App(notes_path: PathBuf) -> Element {
    log::info!(
        "App component initialized with path: {}",
        notes_path.display()
    );

    // Notes path as a signal so it can be changed at runtime
    let notes_path = use_signal(|| notes_path);

    // Error state for runtime errors
    let mut error_state = use_signal(|| None::<RuntimeError>);

    // Build file tree
    let mut file_tree = use_signal(|| {
        let path = notes_path.read();
        log::info!("Building file tree for: {}", path.display());
        match io::build_file_tree(&path) {
            Ok(tree) => {
                log::info!("File tree built successfully");
                tree
            }
            Err(e) => {
                log::error!("Error building file tree: {e}");
                FileTree::new(path.clone())
            }
        }
    });

    let selected_file = use_signal(|| None::<MarkdownFile>);
    let current_document = use_signal(|| None::<Arc<Document>>);
    let current_snapshot = use_signal(|| None::<Snapshot>);
    let focused_folder = use_signal(|| None::<RelativePathBuf>);

    // Mobile navigation state - tracks whether file tree is shown on mobile
    let mut mobile_nav_open = use_signal(|| false);

    // Create callbacks outside the rsx! block for cleaner code
    let on_sidebar_file_select = {
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        let mut mobile_nav_open = mobile_nav_open;
        let mut focused_folder = focused_folder;
        move |markdown_file: MarkdownFile| {
            let path = notes_path.read();
            load_existing_document(
                &markdown_file,
                &path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
            // Clear any folder focus when a file is selected
            focused_folder.set(None);
            // Close mobile nav when file is selected
            mobile_nav_open.set(false);
        }
    };

    let on_file_navigate = {
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |file_path: PathBuf| {
            let path = notes_path.read();
            navigate_to_path(
                file_path,
                &path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_wikilink_navigate = {
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        let mut file_tree = file_tree;
        let mut focused_folder = focused_folder;
        move |target: String| {
            let path = notes_path.read();
            // First check if target matches a folder
            let folder_path = file_tree.read().find_folder(&target);
            if let Some(folder_path) = folder_path {
                // Expand the folder and all its ancestors
                file_tree.write().expand_to_folder(&folder_path);
                // Clear the current file selection and focus the folder
                selected_file.set(None);
                current_document.set(None);
                current_snapshot.set(None);
                focused_folder.set(Some(folder_path));
                return;
            }

            // Not a folder, resolve as file - clear any folder focus
            focused_folder.set(None);
            let markdown_file = resolve_wikilink(&target, &path);
            // Expand parent folders so the file is visible in the tree
            if let Some(parent) = markdown_file.relative_path().parent()
                && !parent.as_str().is_empty()
            {
                file_tree
                    .write()
                    .expand_to_folder(&parent.to_relative_path_buf());
            }
            load_document(
                markdown_file,
                &path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_command = create_command_callback(
        notes_path,
        selected_file,
        current_document,
        current_snapshot,
        file_tree,
        error_state,
    );

    rsx! {
        style { {SOLARIZED_LIGHT_CSS} }
        div {
            class: "app-container",
            // Error banner for runtime errors
            if let Some(error) = error_state.read().as_ref() {
                div {
                    style: "background: #dc322f; color: white; padding: 8px 16px; display: flex; justify-content: space-between; align-items: center;",
                    span {
                        "{error.message}"
                        if let Some(ref details) = error.details {
                            " - {details}"
                        }
                    }
                    button {
                        onclick: move |_| error_state.set(None),
                        "Dismiss"
                    }
                }
            }
            div {
                class: if *mobile_nav_open.read() { "sidebar mobile-visible" } else { "sidebar" },
                div {
                    class: "sidebar-header",
                    h2 { "Files" }
                    button {
                        class: "change-folder-btn",
                        title: "Change notes folder",
                        onclick: move |_| {
                            let mut notes_path = notes_path;
                            let mut file_tree = file_tree;
                            let mut selected_file = selected_file;
                            let mut current_document = current_document;
                            let mut current_snapshot = current_snapshot;
                            let mut focused_folder = focused_folder;
                            let mut error_state = error_state;

                            let current_path = notes_path.read().clone();
                            spawn(async move {
                                if let Some(new_path) = pick_folder(Some(&current_path)).await {
                                    // Save the new path to config
                                    let config = Config { notes_path: new_path.clone() };
                                    match config.save() {
                                        Ok(()) => {
                                            log::info!("Config saved with new notes path: {}", new_path.display());

                                            // Update notes_path signal
                                            notes_path.set(new_path.clone());

                                            // Rebuild file tree
                                            match io::build_file_tree(&new_path) {
                                                Ok(tree) => {
                                                    log::info!("File tree rebuilt successfully");
                                                    file_tree.set(tree);
                                                }
                                                Err(e) => {
                                                    log::error!("Error building file tree: {e}");
                                                    file_tree.set(FileTree::new(new_path));
                                                }
                                            }

                                            // Clear current file state
                                            selected_file.set(None);
                                            current_document.set(None);
                                            current_snapshot.set(None);
                                            focused_folder.set(None);
                                            error_state.set(None);
                                        }
                                        Err(e) => {
                                            RuntimeError::log_and_set(
                                                &mut error_state,
                                                "Failed to save config".to_string(),
                                                e,
                                            );
                                        }
                                    }
                                }
                                // If None (cancelled), do nothing
                            });
                        },
                        "📂"
                    }
                }
                super::components::TreeView {
                    tree: ReadSignal::from(file_tree),
                    selected_file: selected_file.read().clone(),
                    focused_folder: focused_folder.read().clone(),
                    on_file_select: on_sidebar_file_select,
                    on_folder_toggle: move |relative_path: RelativePathBuf| {
                        file_tree.write().toggle_folder(&relative_path);
                    }
                }
            }
            div {
                class: if *mobile_nav_open.read() { "main-content mobile-hidden" } else { "main-content" },
                if let (Some(file), Some(snapshot), Some(document)) = (
                    selected_file.read().as_ref(),
                    current_snapshot.read().as_ref(),
                    current_document.read().as_ref()
                ) {
                    super::components::MainPanel {
                        file: file.clone(),
                        snapshot: snapshot.clone(),
                        notes_path: notes_path.read().clone(),
                        document: document.clone(),
                        on_file_select: Some(Callback::new(on_file_navigate)),
                        on_command,
                        on_wikilink_click: on_wikilink_navigate,
                    }
                } else {
                    div {
                        class: "welcome",
                        h1 { "markdown-neuraxis" }
                        p { "Select a file from the sidebar to view its content" }
                    }
                }
            }
            // Mobile bottom navigation bar
            div {
                class: "mobile-bottom-bar",
                button {
                    class: "hamburger-btn",
                    onclick: move |_| {
                        let current = *mobile_nav_open.read();
                        mobile_nav_open.set(!current);
                    },
                    "☰"
                }
            }
        }
    }
}

/// Helper function to load and parse a document from an existing file
fn load_existing_document(
    markdown_file: &MarkdownFile,
    notes_path: &Path,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    // Clear any previous error
    error_state.set(None);

    match io::read_file(markdown_file.relative_path(), notes_path) {
        Ok(content) => match Document::from_bytes(content.as_bytes()) {
            Ok(document) => {
                // Create snapshot for rendering
                let snapshot = document.snapshot();

                *current_document.write() = Some(Arc::new(document));
                *current_snapshot.write() = Some(snapshot);
                *selected_file.write() = Some(markdown_file.clone());
            }
            Err(e) => {
                RuntimeError::log_and_set(
                    error_state,
                    format!("Failed to parse '{}'", markdown_file.relative_path()),
                    e,
                );
            }
        },
        Err(e) => {
            RuntimeError::log_and_set(
                error_state,
                format!("Failed to read '{}'", markdown_file.relative_path()),
                e,
            );
        }
    }
}

/// Load a document or create a blank one if it doesn't exist
pub fn load_document(
    markdown_file: MarkdownFile,
    notes_path: &Path,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    // Clear any previous error
    error_state.set(None);

    match io::read_file(markdown_file.relative_path(), notes_path) {
        Ok(content) => match Document::from_bytes(content.as_bytes()) {
            Ok(document) => {
                let snapshot = document.snapshot();
                *current_document.write() = Some(Arc::new(document));
                *current_snapshot.write() = Some(snapshot);
                *selected_file.write() = Some(markdown_file);
            }
            Err(e) => {
                RuntimeError::log_and_set(
                    error_state,
                    format!("Failed to parse '{}'", markdown_file.relative_path()),
                    e,
                );
            }
        },
        Err(_) => {
            // File doesn't exist - create a blank document
            match Document::from_bytes(b"") {
                Ok(document) => {
                    let snapshot = document.snapshot();
                    *current_document.write() = Some(Arc::new(document));
                    *current_snapshot.write() = Some(snapshot);
                    *selected_file.write() = Some(markdown_file);
                }
                Err(e) => {
                    RuntimeError::log_and_set(
                        error_state,
                        "Failed to create new document".to_string(),
                        e,
                    );
                }
            }
        }
    }
}

/// Navigate to a file from an absolute path
fn navigate_to_path(
    file_path: PathBuf,
    notes_path: &Path,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    // Convert absolute path to relative
    let relative_path = if let Ok(rel) = file_path.strip_prefix(notes_path) {
        rel.to_path_buf()
    } else {
        // Fallback for paths outside notes root
        file_path.clone()
    };
    let relative_path_buf =
        RelativePathBuf::from_path(&relative_path).expect("Failed to create relative path");
    let markdown_file = MarkdownFile::new(relative_path_buf);

    load_document(
        markdown_file,
        notes_path,
        selected_file,
        current_document,
        current_snapshot,
        error_state,
    );
}

/// Resolve a wikilink target to a markdown file
pub fn resolve_wikilink(target: &str, _notes_path: &Path) -> MarkdownFile {
    // Ensure .md extension is present
    let filename = if target.ends_with(".md") {
        target.to_string()
    } else {
        format!("{}.md", target)
    };

    let relative_path =
        RelativePathBuf::from_path(&filename).expect("Failed to create relative path");
    MarkdownFile::new(relative_path)
}

/// Create a command callback for document editing
fn create_command_callback(
    notes_path: Signal<PathBuf>,
    selected_file: Signal<Option<MarkdownFile>>,
    mut current_document: Signal<Option<Arc<Document>>>,
    mut current_snapshot: Signal<Option<Snapshot>>,
    mut file_tree: Signal<FileTree>,
    mut error_state: Signal<Option<RuntimeError>>,
) -> impl FnMut(Cmd) + 'static {
    move |cmd: Cmd| {
        let path = notes_path.read();
        let document_arc = current_document.read().clone();
        if let Some(mut document_arc) = document_arc {
            // Use Arc::make_mut for efficient copy-on-write
            let document = Arc::make_mut(&mut document_arc);
            let _patch = document.apply(cmd);
            let new_snapshot = document.snapshot();

            // Auto-save the document to disk
            if let Some(file) = selected_file.read().as_ref() {
                let content = document.text();

                // Check if file exists before writing
                let file_existed = io::read_file(file.relative_path(), &path).is_ok();

                // Only create new files if there's actual content
                if !file_existed && content.trim().is_empty() {
                    // Skip creating empty files
                    *current_document.write() = Some(document_arc);
                    *current_snapshot.write() = Some(new_snapshot);
                    return;
                }

                match io::write_file(file.relative_path(), &path, &content) {
                    Ok(()) => {
                        if !file_existed {
                            let absolute_path = file.relative_path().to_path(&*path);
                            file_tree.write().add_file(&absolute_path, &path);
                            log::info!(
                                "New file created and auto-saved: {:?}",
                                file.relative_path()
                            );
                        }
                    }
                    Err(e) => {
                        RuntimeError::log_and_set(
                            &mut error_state,
                            format!("Failed to save '{}'", file.relative_path()),
                            e,
                        );
                    }
                }
            }

            *current_document.write() = Some(document_arc);
            *current_snapshot.write() = Some(new_snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_resolve_wikilink_adds_md_extension() {
        let notes_path = Path::new("/test");
        let result = resolve_wikilink("my-note", notes_path);

        assert_eq!(result.relative_path().as_str(), "my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_preserves_md_extension() {
        let notes_path = Path::new("/test");
        let result = resolve_wikilink("my-note.md", notes_path);

        assert_eq!(result.relative_path().as_str(), "my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_with_path_separators() {
        let notes_path = Path::new("/test");
        let result = resolve_wikilink("folder/my-note", notes_path);

        assert_eq!(result.relative_path().as_str(), "folder/my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_with_path_separators_and_extension() {
        let notes_path = Path::new("/test");
        let result = resolve_wikilink("folder/my-note.md", notes_path);

        assert_eq!(result.relative_path().as_str(), "folder/my-note.md");
    }
}
