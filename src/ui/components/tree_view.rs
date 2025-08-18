use crate::models::{FileTree, FileTreeItem};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn TreeView(
    tree: ReadOnlySignal<FileTree>,
    selected_file: Option<PathBuf>,
    on_file_select: EventHandler<PathBuf>,
    on_folder_toggle: EventHandler<PathBuf>,
) -> Element {
    let items = use_memo(move || tree.read().get_items());

    rsx! {
        div {
            class: "tree-view",
            for item in items.read().iter() {
                TreeViewItem {
                    key: "{item.node.path.display()}",
                    item: item.clone(),
                    is_selected: selected_file.as_ref() == Some(&item.node.path),
                    on_file_select: on_file_select,
                    on_folder_toggle: on_folder_toggle,
                }
            }
        }
    }
}

#[component]
pub fn TreeViewItem(
    item: FileTreeItem,
    is_selected: bool,
    on_file_select: EventHandler<PathBuf>,
    on_folder_toggle: EventHandler<PathBuf>,
) -> Element {
    let node = item.node.clone();
    let depth = item.depth;
    let classes = if is_selected && !node.is_folder {
        "tree-item file selected"
    } else if node.is_folder {
        "tree-item folder"
    } else {
        "tree-item file"
    };

    rsx! {
        div {
            class: "{classes}",
            style: "padding-left: {depth * 20}px;",
            onclick: move |_| {
                if node.is_folder {
                    on_folder_toggle.call(node.path.clone());
                } else {
                    on_file_select.call(node.path.clone());
                }
            },

            if node.is_folder {
                span {
                    class: "tree-toggle",
                    if node.is_expanded { "- " } else { "+ " }
                }
            } else {
                span {
                    class: "tree-file-marker",
                    "  "
                }
            }

            span {
                class: "tree-label",
                "{node.name}"
            }
        }
    }
}
