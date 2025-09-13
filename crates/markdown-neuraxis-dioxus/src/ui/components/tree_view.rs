use crate::ui::components::tree_view_item::TreeViewItem;
use dioxus::events::{Key, KeyboardEvent};
use dioxus::prelude::*;
use markdown_neuraxis_engine::models::{FileTree, FileTreeItem, MarkdownFile};
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
        let items_list = items.read();
        handle_tree_navigation(
            evt,
            *has_focus.read(),
            &items_list,
            &mut focused_index,
            &on_file_select,
            &on_folder_toggle,
        );
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

/// Handle keyboard navigation for the tree view
fn handle_tree_navigation(
    evt: KeyboardEvent,
    has_focus: bool,
    items: &[FileTreeItem],
    focused_index: &mut Signal<usize>,
    on_file_select: &EventHandler<MarkdownFile>,
    on_folder_toggle: &EventHandler<RelativePathBuf>,
) {
    if !has_focus || items.is_empty() {
        return;
    }

    let current_index = *focused_index.read();

    match evt.key() {
        Key::ArrowDown => {
            evt.prevent_default(); // Prevent scrolling
            let new_index = (current_index + 1).min(items.len() - 1);
            focused_index.set(new_index);

            let item = &items[new_index];
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

            let item = &items[new_index];
            if !item.node.is_folder
                && let Some(ref markdown_file) = item.node.markdown_file
            {
                on_file_select.call(markdown_file.clone());
            }
        }
        Key::ArrowRight => {
            evt.prevent_default(); // Prevent default behavior
            if current_index < items.len() {
                let item = &items[current_index];
                if item.node.is_folder && !item.node.is_expanded {
                    on_folder_toggle.call(item.node.relative_path.clone());
                }
            }
        }
        Key::ArrowLeft => {
            evt.prevent_default(); // Prevent default behavior
            if current_index < items.len() {
                let item = &items[current_index];
                if item.node.is_folder && item.node.is_expanded {
                    on_folder_toggle.call(item.node.relative_path.clone());
                }
            }
        }
        _ => {}
    }
}
