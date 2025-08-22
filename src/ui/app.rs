use crate::models::{ContentBlock, Document, DocumentState, FileTree, MarkdownFile};
use crate::{io, parsing};
use dioxus::prelude::*;
use relative_path::RelativePathBuf;
use std::path::PathBuf;

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
    let mut current_document_state = use_signal(|| None::<DocumentState>);

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
                                        let document = parsing::parse_markdown(&content, RelativePathBuf::from(markdown_file.relative_path()));
                                    let document_state = DocumentState::from_document(document);
                                    *current_document_state.write() = Some(document_state);
                                    *selected_file.write() = Some(markdown_file);
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
                if let (Some(file), Some(doc_state)) = (selected_file.read().as_ref(), current_document_state.read().as_ref()) {
                    super::components::EditableMainPanel {
                        file: file.clone(),
                        document_state: doc_state.clone(),
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
                                    let document = parsing::parse_markdown(&content, RelativePathBuf::from(markdown_file.relative_path()));
                                    let document_state = DocumentState::from_document(document);
                                    *current_document_state.write() = Some(document_state);
                                    *selected_file.write() = Some(markdown_file);
                                }
                                Err(_) => {
                                    // File doesn't exist - create a blank document with an empty paragraph
                                    let mut document = Document::new(RelativePathBuf::from(markdown_file.relative_path()));
                                    document.content.push(ContentBlock::Paragraph {
                                        segments: vec![crate::models::TextSegment::Text("".to_string())]
                                    });
                                    let mut document_state = DocumentState::from_document(document);
                                    // Automatically start editing the first block
                                    if let Some((block_id, _)) = document_state.blocks.first() {
                                        document_state.start_editing(*block_id);
                                    }
                                    *current_document_state.write() = Some(document_state);
                                    *selected_file.write() = Some(markdown_file);
                                }
                            }
                        }})),
                        on_save: move |new_doc_state: DocumentState| {
                            // Save to file
                            let document = new_doc_state.to_document();
                            let content = document.content.iter()
                                .map(|block| block.to_markdown())
                                .collect::<Vec<_>>()
                                .join("\n\n");

                            // Convert relative path to absolute for filesystem operations
                            let absolute_path = new_doc_state.path.to_path(&notes_path);

                            // Check if this is a new file
                            let is_new_file = !absolute_path.exists();

                            // Create parent directory if it doesn't exist
                            if let Some(parent) = absolute_path.parent() {
                                if !parent.exists() {
                                    if let Err(e) = std::fs::create_dir_all(parent) {
                                        eprintln!("Error creating directory {parent:?}: {e}");
                                        return;
                                    }
                                }
                            }

                            if let Err(e) = std::fs::write(&absolute_path, &content) {
                                eprintln!("Error writing file {absolute_path:?}: {e}");
                            } else if is_new_file {
                                // Add the new file to the file tree
                                file_tree.write().add_file(&absolute_path, &notes_path);
                            }

                            // Update the state
                            *current_document_state.write() = Some(new_doc_state);
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
