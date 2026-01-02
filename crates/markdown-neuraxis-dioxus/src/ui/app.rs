use dioxus::prelude::*;
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

    // Error state for runtime errors
    let mut error_state = use_signal(|| None::<RuntimeError>);

    // Build file tree
    let mut file_tree = use_signal(|| {
        log::info!("Building file tree for: {}", notes_path.display());
        match io::build_file_tree(&notes_path) {
            Ok(tree) => {
                log::info!("File tree built successfully");
                tree
            }
            Err(e) => {
                log::error!("Error building file tree: {e}");
                FileTree::new(notes_path.clone())
            }
        }
    });

    let selected_file = use_signal(|| None::<MarkdownFile>);
    let current_document = use_signal(|| None::<Arc<Document>>);
    let current_snapshot = use_signal(|| None::<Snapshot>);

    // Create callbacks outside the rsx! block for cleaner code
    let on_sidebar_file_select = {
        let notes_path = notes_path.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |markdown_file: MarkdownFile| {
            load_existing_document(
                &markdown_file,
                &notes_path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_file_navigate = {
        let notes_path = notes_path.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |file_path: PathBuf| {
            navigate_to_path(
                file_path,
                &notes_path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_wikilink_navigate = {
        let notes_path = notes_path.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        let mut error_state = error_state;
        move |target: String| {
            let markdown_file = resolve_wikilink(&target, &notes_path);
            load_document(
                markdown_file,
                &notes_path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
                &mut error_state,
            );
        }
    };

    let on_command = create_command_callback(
        notes_path.clone(),
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
                class: "sidebar",
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
                class: "main-content",
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
    notes_path: PathBuf,
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
                let file_existed = io::read_file(file.relative_path(), &notes_path).is_ok();

                match io::write_file(file.relative_path(), &notes_path, &content) {
                    Ok(()) => {
                        if !file_existed {
                            let absolute_path = file.relative_path().to_path(&notes_path);
                            file_tree.write().add_file(&absolute_path, &notes_path);
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
