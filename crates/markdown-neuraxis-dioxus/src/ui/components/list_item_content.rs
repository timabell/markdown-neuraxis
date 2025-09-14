use crate::ui::components::{editor_block::EditorBlock, text_segment::ContentWithWikiLinks};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{
    AnchorId, BlockKind, Cmd, Document, ListItem, RenderBlock,
};
use std::{path::PathBuf, sync::Arc};

#[component]
pub fn ListItemContent(
    item: ListItem,
    is_focused: bool,
    document: Arc<Document>,
    notes_path: PathBuf,
    on_command: Callback<Cmd>,
    on_focus: Callback<RenderBlock>,
    on_wikilink_click: Callback<String>,
    focused_anchor_id: Signal<Option<AnchorId>>,
) -> Element {
    if is_focused {
        // For list items, include the bullet marker in the editable text
        let content_text = match &item.block.kind {
            BlockKind::ListItem { marker, .. } => {
                // Use the actual marker text stored during parsing
                format!("{}{}", marker.to_string_with_space(), item.block.content)
            }
            _ => item.block.content.clone(),
        };

        rsx! {
            EditorBlock {
                block: item.block.clone(),
                content_text,
                on_command,
                on_cancel: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |_| focused_anchor_id.set(None)
                }
            }
        }
    } else {
        let block = item.block.clone();
        rsx! {
            span {
                class: "list-content clickable-block",
                onclick: move |evt: MouseEvent| {
                    evt.stop_propagation();
                    on_focus.call(block.clone());
                },
                ContentWithWikiLinks {
                    content: item.block.content.clone(),
                    segments: item.block.segments.clone(),
                    notes_path,
                    on_wikilink_click
                }
            }
        }
    }
}
