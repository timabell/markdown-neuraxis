use crate::ui::components::block::Block;
use crate::ui::components::editor_block::EditorBlock;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::RenderBlock;
use std::path::PathBuf;
use std::sync::Arc;

/// Component to render a single block
#[component]
pub fn SingleBlock(
    block: RenderBlock,
    group_index: usize,
    notes_path: PathBuf,
    document: Arc<markdown_neuraxis_engine::editing::Document>,
    focused_anchor_id: Signal<Option<markdown_neuraxis_engine::editing::AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<markdown_neuraxis_engine::editing::Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);

    if is_focused {
        rsx! {
            EditorBlock {
                key: "{group_index}-editor",
                block: block.clone(),
                content_text: block.content.clone(),
                on_command,
                on_cancel: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |_| {
                        focused_anchor_id.set(None);
                    }
                }
            }
        }
    } else {
        rsx! {
            Block {
                key: "{group_index}-render",
                block: block.clone(),
                notes_path,
                on_file_select,
                on_focus: {
                    let mut focused_anchor_id = focused_anchor_id;
                    let block_id = block.id;
                    move |_| {
                        focused_anchor_id.set(Some(block_id));
                    }
                },
                on_wikilink_click
            }
        }
    }
}
