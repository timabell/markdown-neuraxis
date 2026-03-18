use crate::ui::components::block::BlockRenderer;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Cmd, Document, Snapshot};
use std::path::PathBuf;
use std::sync::Arc;

/// Component for document content rendering
#[component]
pub fn DocumentContent(
    snapshot: Snapshot,
    source: String,
    notes_path: PathBuf,
    document: Arc<Document>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    rsx! {
        div {
            class: "document-content",
            for (block_index, block) in snapshot.blocks.iter().enumerate() {
                BlockRenderer {
                    key: "{block_index}",
                    block: block.clone(),
                    source: source.clone(),
                    focused_anchor_id,
                    on_command,
                    on_wikilink_click
                }
            }
        }
    }
}
