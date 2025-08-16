use crate::models::Document;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn MainPanel(file: PathBuf, notes_path: PathBuf, document: Document) -> Element {
    let pages_path = notes_path.join("pages");
    let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
        relative.to_string_lossy().to_string()
    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        "Selected File".to_string()
    };

    rsx! {
        h1 { "📝 {display_name}" }
        hr {}
        if !document.outline.is_empty() {
            div {
                class: "outline-container",
                h3 { "Parsed outline:" }
                div {
                    class: "outline-content",
                    for item in &document.outline {
                        super::OutlineItemComponent { item: item.clone(), indent: 0 }
                    }
                }
            }
        }
    }
}
