use crate::ui::components::{
    CollapseToggle, editor_block::EditorBlock, text_segment::InlineSegments,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, Cmd};
use std::collections::HashSet;

#[component]
pub fn Heading(
    block: Block,
    source: String,
    level: u8,
    focused_anchor_id: Signal<Option<AnchorId>>,
    collapsed_ids: Signal<HashSet<AnchorId>>,
    on_context_menu: Option<Callback<(AnchorId, f64, f64)>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);
    let is_collapsed = collapsed_ids.read().contains(&block.id);
    let block_id = block.id;
    let class_name = format!("heading level-{level} clickable-block");

    if is_focused {
        let content_text = source
            .get(block.node_range.clone())
            .unwrap_or("")
            .to_string();
        let block_clone = block.clone();
        rsx! {
            div {
                class: "{class_name}",
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
        let segments = block.segments.clone();

        match level {
            1 => rsx! {
                h1 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
            2 => rsx! {
                h2 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
            3 => rsx! {
                h3 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
            4 => rsx! {
                h4 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
            5 => rsx! {
                h5 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
            _ => rsx! {
                h6 {
                    class: "{class_name}",
                    onclick: {
                        let mut focused_anchor_id = focused_anchor_id;
                        move |evt| {
                            evt.stop_propagation();
                            focused_anchor_id.set(Some(block_id))
                        }
                    },
                    CollapseToggle { block_id, is_collapsed, collapsed_ids, on_context_menu }
                    InlineSegments { segments, on_wikilink_click }
                }
            },
        }
    }
}
