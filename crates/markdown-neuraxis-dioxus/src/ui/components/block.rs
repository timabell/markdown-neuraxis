use crate::ui::components::{
    block_quote::BlockQuote, code_fence::CodeFence, editor_block::EditorBlock, heading::Heading,
    paragraph::Paragraph, text_segment::InlineSegments, thematic_break::ThematicBreak,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockContent, BlockKind, Cmd};

#[component]
pub fn BlockRenderer(
    block: Block,
    source: String,
    focused_anchor_id: Signal<Option<AnchorId>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    let is_focused = focused_anchor_id.read().as_ref() == Some(&block.id);

    match &block.kind {
        BlockKind::Root => {
            // Container: render children
            if let BlockContent::Children(children) = &block.content {
                rsx! {
                    for (i, child) in children.iter().enumerate() {
                        BlockRenderer {
                            key: "{i}",
                            block: child.clone(),
                            source: source.clone(),
                            focused_anchor_id,
                            on_command,
                            on_wikilink_click
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::List { ordered } => {
            // Container: render list items
            if let BlockContent::Children(children) = &block.content {
                if *ordered {
                    rsx! {
                        ol {
                            class: "list",
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                } else {
                    rsx! {
                        ul {
                            class: "list",
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        BlockKind::ListItem { .. } => {
            if is_focused {
                // Use first line only - node_range includes nested children
                let first_line = block.lines.first();
                let edit_range = first_line.map(|line| line.full.clone());
                let content_text = first_line
                    .and_then(|line| source.get(line.full.clone()))
                    .unwrap_or("")
                    .to_string();
                let block_clone = block.clone();
                rsx! {
                    li {
                        class: "list-item",
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
                        if let BlockContent::Children(children) = &block.content {
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                }
            } else {
                let segments = block.segments.clone();
                let block_id = block.id;
                rsx! {
                    li {
                        class: "list-item",
                        onclick: {
                            let mut focused_anchor_id = focused_anchor_id;
                            move |evt| {
                                evt.stop_propagation();
                                focused_anchor_id.set(Some(block_id))
                            }
                        },
                        InlineSegments {
                            segments,
                            on_wikilink_click
                        }
                        // Render nested children (nested lists)
                        if let BlockContent::Children(children) = &block.content {
                            for (i, child) in children.iter().enumerate() {
                                BlockRenderer {
                                    key: "{i}",
                                    block: child.clone(),
                                    source: source.clone(),
                                    focused_anchor_id,
                                    on_command,
                                    on_wikilink_click
                                }
                            }
                        }
                    }
                }
            }
        }
        BlockKind::Heading { level } => rsx! {
            Heading {
                block: block.clone(),
                source: source.clone(),
                level: *level,
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::Paragraph => rsx! {
            Paragraph {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::FencedCode { language } => rsx! {
            CodeFence {
                block: block.clone(),
                source: source.clone(),
                lang: language.clone(),
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::BlockQuote => rsx! {
            BlockQuote {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                on_command,
                on_wikilink_click
            }
        },
        BlockKind::ThematicBreak => rsx! {
            ThematicBreak {
                block: block.clone(),
                source: source.clone(),
                focused_anchor_id,
                on_command
            }
        },
    }
}
