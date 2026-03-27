use crate::ui::components::{editor_block::EditorBlock, text_segment::InlineSegments};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, Cmd};

#[component]
pub fn BlockQuote(
    block: Block,
    source: String,
    focused_anchor_id: Signal<Option<AnchorId>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);

    if is_focused {
        let content_text = source
            .get(block.node_range.clone())
            .unwrap_or("")
            .to_string();
        let block_clone = block.clone();
        rsx! {
            div {
                class: "block-quote",
                EditorBlock {
                    block: block_clone,
                    content_text,
                    on_command,
                    on_cancel: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |_| focused_anchor_id.set(None)
                    }
                }
            }
        }
    } else {
        let block_id = block.id;
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
                InlineSegments {
                    segments: block.segments.clone(),
                    on_wikilink_click
                }
            }
        }
    }
}
