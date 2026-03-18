use crate::ui::components::editor_block::EditorBlock;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, Cmd};

#[component]
pub fn ThematicBreak(
    block: Block,
    source: String,
    focused_anchor_id: Signal<Option<AnchorId>>,
    on_command: Callback<Cmd>,
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
                class: "thematic-break-editor",
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
            hr {
                class: "thematic-break",
                tabindex: "0",
                onclick: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |_| focused_anchor_id.set(Some(block_id))
                },
                onkeydown: {
                    let mut focused_anchor_id = focused_anchor_id;
                    move |evt| {
                        if evt.key() == Key::Enter {
                            focused_anchor_id.set(Some(block_id));
                        }
                    }
                }
            }
        }
    }
}
