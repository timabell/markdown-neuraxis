use crate::models::Document;
use crate::{io, parsing};
use dioxus::prelude::*;
use std::path::PathBuf;

const SOLARIZED_LIGHT_CSS: &str = include_str!("../assets/solarized-light.css");

#[component]
pub fn App(notes_path: PathBuf) -> Element {
    // Scan for markdown files
    let markdown_files = use_signal(|| match io::scan_markdown_files(&notes_path) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Error scanning files: {e}");
            Vec::new()
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
                p { "Found {markdown_files.read().len()} markdown files:" }
                div {
                    class: "file-list",
                    for file in markdown_files.read().iter() {
                        super::components::FileItem {
                            file: file.clone(),
                            notes_path: notes_path.clone(),
                            is_selected: selected_file.read().as_ref() == Some(file),
                            on_select: move |file_path: PathBuf| {
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
                            }
                        }
                    }
                }
            }
            div {
                class: "main-content",
                if let (Some(file), Some(doc)) = (selected_file.read().as_ref(), current_document.read().as_ref()) {
                    super::components::MainPanel {
                        file: file.to_path_buf(),
                        notes_path: notes_path.clone(),
                        document: doc.clone()
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
