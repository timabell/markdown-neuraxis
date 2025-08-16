use crate::domain::models::FileEntry;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn FileItem(
    file: FileEntry,
    notes_path: PathBuf,
    is_selected: bool,
    on_select: EventHandler<PathBuf>,
) -> Element {
    let pages_path = notes_path.join("pages");
    let display_name = if let Ok(relative) = file.path.strip_prefix(&pages_path) {
        relative.to_string_lossy().to_string()
    } else {
        file.name.clone()
    };

    rsx! {
        div {
            class: if is_selected { "file-item selected" } else { "file-item" },
            onclick: move |_| on_select.call(file.path.clone()),
            "{display_name}"
        }
    }
}