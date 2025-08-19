use crate::models::{Document, FileTree};
use crate::{io, parsing};
use dioxus::prelude::*;
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

    let mut selected_file = use_signal(|| None::<PathBuf>);
    let mut current_document = use_signal(|| None::<Document>);

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
                    on_file_select: move |file_path: PathBuf| {
                        match io::read_file(&file_path) {
                            Ok(content) => {
                                let document = parsing::parse_markdown(&content, file_path.clone());
                                *current_document.write() = Some(document);
                                *selected_file.write() = Some(file_path);
                            }
                            Err(e) => {
                                eprintln!("Error reading file {file_path:?}: {e}");
                            }
                        }
                    },
                    on_folder_toggle: move |folder_path: PathBuf| {
                        file_tree.write().toggle_folder(&folder_path);
                    }
                }
            }
            div {
                class: "main-content",
                if let (Some(file), Some(doc)) = (selected_file.read().as_ref(), current_document.read().as_ref()) {
                    super::components::MainPanel {
                        file: file.to_path_buf(),
                        notes_path: notes_path.clone(),
                        document: doc.clone(),
                        on_file_select: Some(Callback::new(move |file_path: PathBuf| {
                            match io::read_file(&file_path) {
                                Ok(content) => {
                                    let document = parsing::parse_markdown(&content, file_path.clone());
                                    *current_document.write() = Some(document);
                                    *selected_file.write() = Some(file_path);
                                }
                                Err(e) => {
                                    eprintln!("Error reading file {file_path:?}: {e}");
                                }
                            }
                        }))
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
