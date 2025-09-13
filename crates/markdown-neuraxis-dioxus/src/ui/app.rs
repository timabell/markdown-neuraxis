use dioxus::prelude::*;
use markdown_neuraxis_engine::{Document, FileTree, MarkdownFile, Snapshot, io};
use relative_path::RelativePathBuf;
use std::path::PathBuf;
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

    let mut selected_file = use_signal(|| None::<MarkdownFile>);
    let mut current_document = use_signal(|| None::<Arc<Document>>);
    let mut current_snapshot = use_signal(|| None::<Snapshot>);

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
                    on_file_select: {
                        let notes_path = notes_path.clone();
                        move |markdown_file: MarkdownFile| {
                            match io::read_file(markdown_file.relative_path(), &notes_path) {
                                Ok(content) => {
                                    match Document::from_bytes(content.as_bytes()) {
                                        Ok(mut document) => {
                                            // Create anchors for the document blocks
                                            document.create_anchors_from_tree();

                                            // Create snapshot for rendering
                                            let snapshot = document.snapshot();

                                            *current_document.write() = Some(Arc::new(document));
                                            *current_snapshot.write() = Some(snapshot);
                                            *selected_file.write() = Some(markdown_file);
                                        }
                                        Err(e) => {
                                            eprintln!("Error parsing document {:?}: {e}", markdown_file.relative_path());
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading file {:?}: {e}", markdown_file.relative_path());
                                }
                            }
                        }
                    },
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
                        document: document.clone(),
                        on_file_select: Some(Callback::new({
                            let notes_path = notes_path.clone();
                            move |file_path: PathBuf| {
                                // Convert absolute path to relative
                                let relative_path = if let Ok(rel) = file_path.strip_prefix(&notes_path) {
                                    rel.to_path_buf()
                                } else {
                                    // Fallback for paths outside notes root
                                    file_path.clone()
                                };
                                let relative_path_buf = RelativePathBuf::from_path(&relative_path)
                                    .expect("Failed to create relative path");
                                let markdown_file = MarkdownFile::new(relative_path_buf);

                                match io::read_file(markdown_file.relative_path(), &notes_path) {
                                    Ok(content) => {
                                        match Document::from_bytes(content.as_bytes()) {
                                            Ok(mut document) => {
                                                document.create_anchors_from_tree();

                                                let snapshot = document.snapshot();

                                                *current_document.write() = Some(Arc::new(document));
                                                *current_snapshot.write() = Some(snapshot);
                                                *selected_file.write() = Some(markdown_file);
                                            }
                                            Err(e) => {
                                                eprintln!("Error parsing document {:?}: {e}", markdown_file.relative_path());
                                            }
                                        }
                                    }
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
                        })),
                        on_save: {
                            let notes_path = notes_path.clone();
                            let selected_file = selected_file.read().clone();
                            let current_document = current_document.read().clone();
                            move |_| {
                                // Save the current document to disk
                                if let (Some(file), Some(document)) = (&selected_file, &current_document) {
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
                        },
                        on_command: {
                            let mut current_document = current_document;
                            let mut current_snapshot = current_snapshot;
                            let notes_path = notes_path.clone();
                            let selected_file = selected_file.read().clone();
                            move |cmd: markdown_neuraxis_engine::editing::commands::Cmd| {
                                // Apply the command to the current document using Arc for copy-on-write
                                let document_arc = current_document.read().clone();
                                if let Some(mut document_arc) = document_arc {
                                    // Use Arc::make_mut for efficient copy-on-write
                                    let document = Arc::make_mut(&mut document_arc);
                                    let _patch = document.apply(cmd);
                                    let new_snapshot = document.snapshot();

                                    // Auto-save the document to disk
                                    if let Some(file) = &selected_file {
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
