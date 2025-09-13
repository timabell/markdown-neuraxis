use crate::ui::components::{list_children::ListChildren, list_item_content::ListItemContent};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, ListItem, RenderBlock};
use std::path::PathBuf;
use std::sync::Arc;

/// Component to render a single list item with potential nested children
#[component]
pub fn ListItemComponent(
    item: ListItem,
    notes_path: PathBuf,
    on_file_select: Option<Callback<PathBuf>>,
    on_focus: Callback<RenderBlock>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    document: Arc<Document>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&item.block.id);

    rsx! {
        li {
            class: "markdown-list-item",

            // Render either editor or clickable content based on focus state
            ListItemContent {
                item: item.clone(),
                is_focused,
                document: document.clone(),
                notes_path: notes_path.clone(),
                on_command,
                on_focus,
                on_wikilink_click,
                focused_anchor_id
            }

            // Render nested children if present
            ListChildren {
                item: item.clone(),
                notes_path,
                on_file_select,
                on_focus,
                on_command,
                on_wikilink_click,
                focused_anchor_id,
                document
            }
        }
    }
}
