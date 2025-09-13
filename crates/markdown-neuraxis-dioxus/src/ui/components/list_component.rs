use crate::ui::components::list_item_component::ListItemComponent;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, ListItem, RenderBlock};
use std::path::PathBuf;
use std::sync::Arc;

/// Component to render a nested list group as proper HTML ul/ol structure
#[component]
pub fn ListComponent(
    items: Vec<ListItem>,
    list_type: &'static str,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Arc<Document>,
) -> Element {
    match list_type {
        "ol" => rsx! {
            ol {
                class: "markdown-list",
                for item in items {
                    ListItemComponent {
                        item,
                        notes_path: notes_path.clone(),
                        on_file_select,
                        on_focus,
                        on_command,
                        on_wikilink_click,
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
                        notes_path: notes_path.clone(),
                        on_file_select,
                        on_focus,
                        on_command,
                        on_wikilink_click,
                        focused_anchor_id,
                        document: document.clone()
                    }
                }
            }
        },
    }
}
