use crate::app::ApplicationServices;
use crate::domain::models::Document;
use dioxus::prelude::*;
use std::path::PathBuf;

#[derive(Props, Clone, PartialEq)]
pub struct AppProps {
    pub services: ApplicationServices,
    pub notes_path: PathBuf,
}

#[component]
pub fn App(props: AppProps) -> Element {
    use_context_provider(|| props.services.clone());
    
    let markdown_files = use_signal(|| {
        props.services
            .document_service
            .scan_markdown_files(&props.notes_path)
            .unwrap_or_default()
    });
    
    let mut selected_file = use_signal(|| None::<PathBuf>);
    let mut current_document = use_signal(|| None::<Document>);

    rsx! {
        style { {super::styles::SOLARIZED_LIGHT_CSS} }
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
                            notes_path: props.notes_path.clone(),
                            is_selected: selected_file.read().as_ref() == Some(&file.path),
                            on_select: move |file_path: PathBuf| {
                                let services = use_context::<ApplicationServices>();
                                match services.document_service.load_document(&file_path) {
                                    Ok(document) => {
                                        *current_document.write() = Some(document);
                                        *selected_file.write() = Some(file_path);
                                    }
                                    Err(e) => {
                                        eprintln!("Error loading document {file_path:?}: {e}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                class: "main-content",
                if let (Some(ref file), Some(ref doc)) = (selected_file.read().as_ref(), current_document.read().as_ref()) {
                    super::components::MainPanel {
                        file: file.to_path_buf(),
                        notes_path: props.notes_path.clone(),
                        document: (*doc).clone()
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