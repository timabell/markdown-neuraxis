use crate::models::{FileTree, FileTreeItem, MarkdownFile};
use dioxus::events::Key;
use dioxus::prelude::*;
use relative_path::RelativePathBuf;

#[component]
pub fn TreeView(
    tree: ReadOnlySignal<FileTree>,
    selected_file: Option<MarkdownFile>,
    on_file_select: EventHandler<MarkdownFile>,
    on_folder_toggle: EventHandler<RelativePathBuf>,
) -> Element {
    let items = use_memo(move || tree.read().get_items());
    let mut focused_index = use_signal(|| 0usize);
    let mut has_focus = use_signal(|| false);

    // Use effect to sync focused index when selected file changes
    {
        let selected_file_clone = selected_file.clone();
        use_effect(move || {
            if let Some(ref selected_file) = selected_file_clone {
                let items_list = items.read();
                if let Some(index) = items_list.iter().position(|item| {
                    if let Some(ref item_markdown_file) = item.node.markdown_file {
                        item_markdown_file.relative_path() == selected_file.relative_path()
                    } else {
                        false
                    }
                }) {
                    focused_index.set(index);
                }
            }
        });
    }

    // Handle focus events
    let handle_focus = move |_| {
        has_focus.set(true);
    };

    let handle_blur = move |_| {
        has_focus.set(false);
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
                focused_index.set(new_index);

                let item = &items_list[new_index];
                if !item.node.is_folder
                    && let Some(ref markdown_file) = item.node.markdown_file
                {
                    on_file_select.call(markdown_file.clone());
                }
            }
            Key::ArrowUp => {
                evt.prevent_default(); // Prevent scrolling
                let new_index = current_index.saturating_sub(1);
                focused_index.set(new_index);

                let item = &items_list[new_index];
                if !item.node.is_folder
                    && let Some(ref markdown_file) = item.node.markdown_file
                {
                    on_file_select.call(markdown_file.clone());
                }
            }
            Key::ArrowRight => {
                evt.prevent_default(); // Prevent default behavior
                if current_index < items_list.len() {
                    let item = &items_list[current_index];
                    if item.node.is_folder && !item.node.is_expanded {
                        on_folder_toggle.call(item.node.relative_path.clone());
                    }
                }
            }
            Key::ArrowLeft => {
                evt.prevent_default(); // Prevent default behavior
                if current_index < items_list.len() {
                    let item = &items_list[current_index];
                    if item.node.is_folder && item.node.is_expanded {
                        on_folder_toggle.call(item.node.relative_path.clone());
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
                    key: "{index}",
                    item: item.clone(),
                    is_selected: {
                        if let (Some(selected_file), Some(item_markdown_file)) =
                            (selected_file.as_ref(), &item.node.markdown_file) {
                            selected_file.relative_path() == item_markdown_file.relative_path()
                        } else {
                            false
                        }
                    },
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
    on_file_select: EventHandler<MarkdownFile>,
    on_folder_toggle: EventHandler<RelativePathBuf>,
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
                    on_folder_toggle.call(node.relative_path.clone());
                } else if let Some(ref markdown_file) = node.markdown_file {
                    on_file_select.call(markdown_file.clone());
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
