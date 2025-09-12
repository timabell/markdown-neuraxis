use crate::editing::RenderBlock;
use crate::ui::components::block::Block;
use crate::ui::components::editor_block::EditorBlock;
use dioxus::prelude::*;
use std::path::PathBuf;

/// Component to render a single block
#[component]
pub fn SingleBlock(
    block: RenderBlock,
    group_index: usize,
    document: crate::editing::Document,
    focused_anchor_id: Signal<Option<crate::editing::AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<crate::editing::Cmd>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);

    if is_focused {
        rsx! {
            EditorBlock {
                key: "{group_index}-editor",
                block: block.clone(),
                content_text: document.slice_to_cow(block.byte_range.clone()).to_string(),
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
                on_file_select,
                on_focus: {
                    let mut focused_anchor_id = focused_anchor_id;
                    let block_id = block.id;
                    move |_| {
                        focused_anchor_id.set(Some(block_id));
                    }
                }
            }
        }
    }
}
