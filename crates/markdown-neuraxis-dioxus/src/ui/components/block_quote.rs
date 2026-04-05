use crate::ui::components::{
    block::BlockRenderer, editor_block::EditorBlock, text_segment::InlineSegments,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockContent, Cmd};
use std::collections::HashSet;

#[component]
pub fn BlockQuote(
    block: Block,
    source: String,
    focused_anchor_id: Signal<Option<AnchorId>>,
    collapsed_ids: Signal<HashSet<AnchorId>>,
    on_context_menu: Option<Callback<(AnchorId, f64, f64)>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);

    if is_focused {
        // Use content_range() - excludes nested children
        let edit_range = block.content_range();
        let content_text = source.get(edit_range.clone()).unwrap_or("").to_string();
        let edit_range = Some(edit_range);
        let block_clone = block.clone();
        let children = if let BlockContent::Children(c) = &block.content {
            Some(c.clone())
        } else {
            None
        };

        rsx! {
            blockquote {
                class: "block-quote",
                EditorBlock {
                    block: block_clone,
                    content_text,
                    edit_range,
                    on_command,
                    on_cancel: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |_| focused_anchor_id.set(None)
                    }
                }
                // Still render nested children below the editor
                if let Some(children) = children {
                    for (i, child) in children.iter().enumerate() {
                        BlockRenderer {
                            key: "{i}",
                            block: child.clone(),
                            source: source.clone(),
                            focused_anchor_id,
                            collapsed_ids,
                            on_context_menu,
                            on_command,
                            on_wikilink_click
                        }
                    }
                }
            }
        }
    } else {
        let block_id = block.id;
        let children = if let BlockContent::Children(c) = &block.content {
            Some(c.clone())
        } else {
            None
        };

        rsx! {
            blockquote {
                class: "block-quote",
                tabindex: "0",
                onclick: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |evt| {
                        evt.stop_propagation();
                        focused_anchor_id.set(Some(block_id))
                    }
                },
                onkeydown: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |evt| {
                        if evt.key() == Key::Enter {
                            focused_anchor_id.set(Some(block_id));
                        }
                    }
                },
                // Render inline segments (this level's content)
                InlineSegments {
                    segments: block.segments.clone(),
                    on_wikilink_click
                }
                // Render nested children (deeper blockquote levels)
                if let Some(children) = children {
                    for (i, child) in children.iter().enumerate() {
                        BlockRenderer {
                            key: "{i}",
                            block: child.clone(),
                            source: source.clone(),
                            focused_anchor_id,
                            collapsed_ids,
                            on_context_menu,
                            on_command,
                            on_wikilink_click
                        }
                    }
                }
            }
        }
    }
}
