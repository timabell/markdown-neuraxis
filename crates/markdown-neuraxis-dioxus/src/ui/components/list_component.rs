use crate::ui::components::list_item_component::ListItemComponent;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, ListItem, RenderBlock};
use std::path::PathBuf;

/// Component to render a nested list group as proper HTML ul/ol structure
#[component]
pub fn ListComponent(
    items: Vec<ListItem>,
    list_type: &'static str,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Document,
) -> Element {
    match list_type {
        "ol" => rsx! {
            ol {
                class: "markdown-list",
                for item in items {
                    ListItemComponent {
                        item,
                        on_file_select,
                        on_focus,
                        on_command,
                        focused_anchor_id,
                        document: document.clone()
                    }
                }
            }
        },
        _ => rsx! {
            ul {
                class: "markdown-list",
                for item in items {
                    ListItemComponent {
                        item,
                        on_file_select,
                        on_focus,
                        on_command,
                        focused_anchor_id,
                        document: document.clone()
                    }
                }
            }
        },
    }
}
