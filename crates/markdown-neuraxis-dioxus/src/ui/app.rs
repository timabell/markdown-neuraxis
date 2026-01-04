use dioxus::prelude::*;
use markdown_neuraxis_engine::{
    Document, FileTree, MarkdownFile, Snapshot,
    editing::commands::Cmd,
    io::{self, IoProvider, StdFsProvider},
};
use relative_path::RelativePathBuf;
use std::path::PathBuf;
use std::sync::Arc;

/// Wrapper for IoProvider that implements PartialEq for Dioxus component props
#[derive(Clone)]
pub struct IoProviderRef(pub Arc<dyn IoProvider>);

impl PartialEq for IoProviderRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for IoProviderRef {
    type Target = dyn IoProvider;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

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

    // Create IO provider for file operations
    let io_provider = IoProviderRef(Arc::new(StdFsProvider::new(notes_path.clone())));

    // Error state for runtime errors
    let mut error_state = use_signal(|| None::<RuntimeError>);

    // Build file tree
    let mut file_tree = {
        let provider = io_provider.clone();
        let notes_path_for_fallback = notes_path.clone();
        use_signal(move || {
            log::info!("Building file tree for: {}", provider.root_display_name());
            match io::build_file_tree(provider.0.as_ref()) {
                Ok(tree) => {
                    log::info!("File tree built successfully");
                    tree
                }
                Err(e) => {
                    log::error!("Error building file tree: {e}");
                    FileTree::new(notes_path_for_fallback.clone())
                }
            }
        })
    };

    let selected_file = use_signal(|| None::<MarkdownFile>);
    let current_document = use_signal(|| None::<Arc<Document>>);
    let current_snapshot = use_signal(|| None::<Snapshot>);

    // Mobile navigation state - tracks whether file tree is shown on mobile
    let mut mobile_nav_open = use_signal(|| false);

    // Create callbacks outside the rsx! block for cleaner code
    let on_sidebar_file_select = {
        let provider = io_provider.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        let mut mobile_nav_open = mobile_nav_open;
        move |markdown_file: MarkdownFile| {
            load_existing_document(
                &markdown_file,
                provider.0.as_ref(),
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
            // Close mobile nav when file is selected
            mobile_nav_open.set(false);
        }
    };

    let on_file_navigate = {
        let notes_path = notes_path.clone();
        let provider = io_provider.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |file_path: PathBuf| {
            // Convert absolute path to relative for navigation
            let relative_path = if let Ok(rel) = file_path.strip_prefix(&notes_path) {
                RelativePathBuf::from_path(rel).unwrap_or_default()
            } else {
                RelativePathBuf::from_path(&file_path).unwrap_or_default()
            };
            navigate_to_path(
                relative_path,
                provider.0.as_ref(),
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_wikilink_navigate = {
        let provider = io_provider.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |target: String| {
            let markdown_file = resolve_wikilink(&target);
            load_document(
                markdown_file,
                provider.0.as_ref(),
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_command = create_command_callback(
        io_provider,
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
                h2 { "Files" }
                super::components::TreeView {
                    tree: ReadSignal::from(file_tree),
                    selected_file: selected_file.read().clone(),
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
                        notes_path: notes_path.clone(),
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
    provider: &dyn IoProvider,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    // Clear any previous error
    error_state.set(None);

    match provider.read_file(markdown_file.relative_path()) {
        Ok(content) => match Document::from_bytes(content.as_bytes()) {
            Ok(mut document) => {
                // Create anchors for the document blocks
                document.create_anchors_from_tree();

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
    provider: &dyn IoProvider,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    // Clear any previous error
    error_state.set(None);

    match provider.read_file(markdown_file.relative_path()) {
        Ok(content) => match Document::from_bytes(content.as_bytes()) {
            Ok(mut document) => {
                document.create_anchors_from_tree();
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
                Ok(mut document) => {
                    document.create_anchors_from_tree();
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

/// Navigate to a file from a relative path
fn navigate_to_path(
    relative_path: RelativePathBuf,
    provider: &dyn IoProvider,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
    error_state: &mut Signal<Option<RuntimeError>>,
) {
    let markdown_file = MarkdownFile::new(relative_path);

    load_document(
        markdown_file,
        provider,
        selected_file,
        current_document,
        current_snapshot,
        error_state,
    );
}

/// Resolve a wikilink target to a markdown file
pub fn resolve_wikilink(target: &str) -> MarkdownFile {
    // Ensure .md extension is present
    let filename = if target.ends_with(".md") {
        target.to_string()
    } else {
        format!("{}.md", target)
    };

    let relative_path = RelativePathBuf::from(filename);
    MarkdownFile::new(relative_path)
}

/// Create a command callback for document editing
fn create_command_callback(
    provider: IoProviderRef,
    selected_file: Signal<Option<MarkdownFile>>,
    mut current_document: Signal<Option<Arc<Document>>>,
    mut current_snapshot: Signal<Option<Snapshot>>,
    mut file_tree: Signal<FileTree>,
    mut error_state: Signal<Option<RuntimeError>>,
) -> impl FnMut(Cmd) + 'static {
    move |cmd: Cmd| {
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
                let file_existed = provider.0.exists(file.relative_path());

                match provider.0.write_file(file.relative_path(), &content) {
                    Ok(()) => {
                        if !file_existed {
                            file_tree
                                .write()
                                .add_file_relative(file.relative_path().to_owned());
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

    #[test]
    fn test_resolve_wikilink_adds_md_extension() {
        let result = resolve_wikilink("my-note");

        assert_eq!(result.relative_path().as_str(), "my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_preserves_md_extension() {
        let result = resolve_wikilink("my-note.md");

        assert_eq!(result.relative_path().as_str(), "my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_with_path_separators() {
        let result = resolve_wikilink("folder/my-note");

        assert_eq!(result.relative_path().as_str(), "folder/my-note.md");
    }

    #[test]
    fn test_resolve_wikilink_with_path_separators_and_extension() {
        let result = resolve_wikilink("folder/my-note.md");

        assert_eq!(result.relative_path().as_str(), "folder/my-note.md");
    }
}
