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
