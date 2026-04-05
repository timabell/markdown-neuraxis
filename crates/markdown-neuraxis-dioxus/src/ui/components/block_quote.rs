use crate::ui::components::{
    block::BlockRenderer, editor_block::EditorBlock, text_segment::InlineSegments,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockContent, BlockKind, Cmd};
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

    // Separate paragraph children (this level's content) from nested blockquotes
    let (paragraphs, nested_quotes): (Vec<_>, Vec<_>) =
        if let BlockContent::Children(c) = &block.content {
            c.iter()
                .cloned()
                .partition(|child| matches!(child.kind, BlockKind::Paragraph))
        } else {
            (vec![], vec![])
        };

    if is_focused {
        // Calculate edit range covering only paragraphs (not nested blockquotes)
        let edit_range = if let (Some(first), Some(last)) = (paragraphs.first(), paragraphs.last())
        {
            first.node_range.start..last.node_range.end
        } else {
            block.node_range.clone()
        };
        let content_text = source.get(edit_range.clone()).unwrap_or("").to_string();
        let edit_range = Some(edit_range);
        let block_clone = block.clone();

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
                // Render nested blockquotes below editor
                for (i, child) in nested_quotes.iter().enumerate() {
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
    } else {
        let block_id = block.id;

        rsx! {
            blockquote {
                class: "block-quote",
                // All paragraphs in one clickable container (hover applies to all)
                if !paragraphs.is_empty() {
                    span {
                        class: "block-quote-content clickable-block",
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
                        for (i, para) in paragraphs.iter().enumerate() {
                            p {
                                key: "{i}",
                                class: "block-quote-paragraph",
                                InlineSegments {
                                    segments: para.segments.clone(),
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                }
                // Nested blockquotes outside clickable area (handle their own clicks)
                for (i, child) in nested_quotes.iter().enumerate() {
                    BlockRenderer {
                        key: "nested-{i}",
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
