use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn FileItem(
    file: PathBuf,
    notes_path: PathBuf,
    is_selected: bool,
    on_select: EventHandler<PathBuf>,
) -> Element {
    let pages_path = notes_path.join("pages");
    let display_name = if let Ok(relative) = file.strip_prefix(&pages_path) {
        relative.to_string_lossy().to_string()
    } else if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        "Unknown".to_string()
    };

    rsx! {
        div {
            class: if is_selected { "file-item selected" } else { "file-item" },
            onclick: move |_| on_select.call(file.clone()),
            "{display_name}"
        }
    }
}
