use dioxus::prelude::*;
use markdown_neuraxis_engine::{
    Document, FileTree, MarkdownFile, Snapshot, editing::commands::Cmd, io,
};
use relative_path::RelativePathBuf;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SOLARIZED_LIGHT_CSS: &str = include_str!("../assets/solarized-light.css");

#[component]
pub fn App(notes_path: PathBuf) -> Element {
    // Build file tree
    let mut file_tree = use_signal(|| match io::build_file_tree(&notes_path) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("Error building file tree: {e}");
            FileTree::new(notes_path.clone())
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
        move |markdown_file: MarkdownFile| {
            load_document(
                &markdown_file,
                &notes_path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
            );
        }
    };

    let on_file_navigate = {
        let notes_path = notes_path.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        move |file_path: PathBuf| {
            navigate_to_path(
                file_path,
                &notes_path,
                &mut selected_file,
                &mut current_document,
                &mut current_snapshot,
            );
        }
    };

    let on_wikilink_navigate = {
        let notes_path = notes_path.clone();
        let mut selected_file = selected_file;
        let mut current_document = current_document;
        let mut current_snapshot = current_snapshot;
        move |target: String| {
            if let Some(markdown_file) = resolve_wikilink(&target, &notes_path) {
                load_document(
                    &markdown_file,
                    &notes_path,
                    &mut selected_file,
                    &mut current_document,
                    &mut current_snapshot,
                );
            } else {
                eprintln!("Wikilink target not found: {}", target);
            }
        }
    };

    let on_save = create_save_callback(notes_path.clone(), selected_file, current_document);
    let on_command = create_command_callback(
        notes_path.clone(),
        selected_file,
        current_document,
        current_snapshot,
    );

    rsx! {
        style { {SOLARIZED_LIGHT_CSS} }
        div {
            class: "app-container",
            div {
                class: "sidebar",
                h2 { "Files" }
                super::components::TreeView {
                    tree: ReadOnlySignal::from(file_tree),
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
                        on_save,
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

/// Helper function to load and parse a document from a file
fn load_document(
    markdown_file: &MarkdownFile,
    notes_path: &Path,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
) {
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
                eprintln!(
                    "Error parsing document {:?}: {e}",
                    markdown_file.relative_path()
                );
            }
        },
        Err(e) => {
            eprintln!(
                "Error reading file {:?}: {e}",
                markdown_file.relative_path()
            );
        }
    }
}

/// Load a document or create a blank one if it doesn't exist
fn load_or_create_document(
    markdown_file: MarkdownFile,
    notes_path: &Path,
    selected_file: &mut Signal<Option<MarkdownFile>>,
    current_document: &mut Signal<Option<Arc<Document>>>,
    current_snapshot: &mut Signal<Option<Snapshot>>,
) {
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
                eprintln!(
                    "Error parsing document {:?}: {e}",
                    markdown_file.relative_path()
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
                    eprintln!("Error creating blank document: {e}");
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

    load_or_create_document(
        markdown_file,
        notes_path,
        selected_file,
        current_document,
        current_snapshot,
    );
}

/// Resolve a wikilink target to a markdown file
fn resolve_wikilink(target: &str, notes_path: &Path) -> Option<MarkdownFile> {
    let potential_files = vec![
        format!("{}.md", target),
        target.to_string(),
    ];

    for potential_file in potential_files {
        let relative_path =
            RelativePathBuf::from_path(&potential_file).expect("Failed to create relative path");
        let markdown_file = MarkdownFile::new(relative_path.clone());

        // Check if file exists
        if io::read_file(markdown_file.relative_path(), notes_path).is_ok() {
            return Some(markdown_file);
        }
    }
    None
}

/// Create a save callback
fn create_save_callback(
    notes_path: PathBuf,
    selected_file: Signal<Option<MarkdownFile>>,
    current_document: Signal<Option<Arc<Document>>>,
) -> impl Fn(()) + 'static {
    move |_| {
        if let (Some(file), Some(document)) = (
            selected_file.read().as_ref(),
            current_document.read().as_ref(),
        ) {
            let content = document.text();
            match io::write_file(file.relative_path(), &notes_path, &content) {
                Ok(()) => {
                    println!("File saved successfully: {:?}", file.relative_path());
                }
                Err(e) => {
                    eprintln!("Error saving file {:?}: {e}", file.relative_path());
                }
            }
        }
    }
}

/// Create a command callback for document editing
fn create_command_callback(
    notes_path: PathBuf,
    selected_file: Signal<Option<MarkdownFile>>,
    mut current_document: Signal<Option<Arc<Document>>>,
    mut current_snapshot: Signal<Option<Snapshot>>,
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
                match io::write_file(file.relative_path(), &notes_path, &content) {
                    Ok(()) => {
                        // File saved successfully
                    }
                    Err(e) => {
                        eprintln!("Error auto-saving file {:?}: {e}", file.relative_path());
                    }
                }
            }

            *current_document.write() = Some(document_arc);
            *current_snapshot.write() = Some(new_snapshot);
        }
    }
}
