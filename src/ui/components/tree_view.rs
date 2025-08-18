use crate::models::{FileTree, FileTreeItem};
use dioxus::events::Key;
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
    let mut focused_index = use_signal(|| 0usize);
    let mut has_focus = use_signal(|| false);

    // Handle focus events
    let handle_focus = move |_| {
        *has_focus.write() = true;
    };

    let handle_blur = move |_| {
        *has_focus.write() = false;
    };

    // Handle keyboard navigation (only when focused)
    let handle_keydown = move |evt: KeyboardEvent| {
        if !*has_focus.read() {
            return;
        }
        let items_list = items.read();
        if items_list.is_empty() {
            return;
        }

        let current_index = *focused_index.read();

        match evt.key() {
            Key::ArrowDown => {
                evt.prevent_default(); // Prevent scrolling
                let new_index = (current_index + 1).min(items_list.len() - 1);
                *focused_index.write() = new_index;

                let item = &items_list[new_index];
                if !item.node.is_folder {
                    on_file_select.call(item.node.path.clone());
                }
            }
            Key::ArrowUp => {
                evt.prevent_default(); // Prevent scrolling
                let new_index = current_index.saturating_sub(1);
                *focused_index.write() = new_index;

                let item = &items_list[new_index];
                if !item.node.is_folder {
                    on_file_select.call(item.node.path.clone());
                }
            }
            Key::ArrowRight => {
                evt.prevent_default(); // Prevent default behavior
                if current_index < items_list.len() {
                    let item = &items_list[current_index];
                    if item.node.is_folder && !item.node.is_expanded {
                        on_folder_toggle.call(item.node.path.clone());
                    }
                }
            }
            Key::ArrowLeft => {
                evt.prevent_default(); // Prevent default behavior
                if current_index < items_list.len() {
                    let item = &items_list[current_index];
                    if item.node.is_folder && item.node.is_expanded {
                        on_folder_toggle.call(item.node.path.clone());
                    }
                }
            }
            _ => {}
        }
    };

    rsx! {
        div {
            class: "tree-view",
            tabindex: "0",
            onkeydown: handle_keydown,
            onfocus: handle_focus,
            onblur: handle_blur,
            for (index, item) in items.read().iter().enumerate() {
                TreeViewItem {
                    key: "{item.node.path.display()}",
                    item: item.clone(),
                    is_selected: selected_file.as_ref() == Some(&item.node.path),
                    is_focused: index == *focused_index.read() && *has_focus.read(),
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
    is_focused: bool,
    on_file_select: EventHandler<PathBuf>,
    on_folder_toggle: EventHandler<PathBuf>,
) -> Element {
    let node = item.node.clone();
    let depth = item.depth;
    let classes = if is_selected && !node.is_folder {
        if is_focused {
            "tree-item file selected focused"
        } else {
            "tree-item file selected"
        }
    } else if is_focused {
        if node.is_folder {
            "tree-item folder focused"
        } else {
            "tree-item file focused"
        }
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
