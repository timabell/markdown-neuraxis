use dioxus::prelude::*;
use markdown_neuraxis_engine::models::{FileTreeItem, MarkdownFile};
use relative_path::RelativePathBuf;

#[component]
pub fn TreeViewItem(
    item: FileTreeItem,
    is_selected: bool,
    is_focused: bool,
    on_file_select: EventHandler<MarkdownFile>,
    on_folder_toggle: EventHandler<RelativePathBuf>,
    on_new_file: EventHandler<RelativePathBuf>,
) -> Element {
    let node = item.node.clone();
    let node_for_add = item.node.clone();
    let depth = item.depth;
    let type_class = if node.is_folder { "folder" } else { "file" };
    let selected_class = if is_selected { " selected" } else { "" };
    let focused_class = if is_focused { " focused" } else { "" };
    let classes = format!("tree-item {type_class}{selected_class}{focused_class}");

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
                    if node.is_expanded { "📂 " } else { "📁 " }
                }
            } else {
                span {
                    class: "tree-file-marker",
                    "   "
                }
            }

            span {
                class: "tree-label",
                "{node.name}"
            }

            if node.is_folder {
                span {
                    class: "tree-add-file",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        on_new_file.call(node_for_add.relative_path.clone());
                    },
                    "+"
                }
            }
        }
    }
}
