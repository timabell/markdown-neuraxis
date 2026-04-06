use crate::ui::components::{
    block::BlockRenderer, editor_block::EditorBlock, text_segment::InlineSegments,
};
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockContent, BlockKind, Cmd};
use std::collections::HashSet;
use std::ops::Range;

/// Groups children into runs of consecutive paragraphs vs other block types.
/// Each ParagraphRun contains the blocks and combined byte range.
enum ChildGroup {
    ParagraphRun {
        blocks: Vec<Block>,
        range: Range<usize>,
    },
    OtherBlock(Block),
}

fn group_children(children: &[Block]) -> Vec<ChildGroup> {
    let mut groups = Vec::new();
    let mut current_paras: Vec<Block> = Vec::new();

    for child in children {
        if matches!(child.kind, BlockKind::Paragraph) {
            current_paras.push(child.clone());
        } else {
            // Flush any accumulated paragraph run
            if !current_paras.is_empty() {
                let range = current_paras.first().unwrap().node_range.start
                    ..current_paras.last().unwrap().node_range.end;
                groups.push(ChildGroup::ParagraphRun {
                    blocks: std::mem::take(&mut current_paras),
                    range,
                });
            }
            // Add this non-paragraph block
            groups.push(ChildGroup::OtherBlock(child.clone()));
        }
    }

    // Flush final paragraph run if any
    if !current_paras.is_empty() {
        let range = current_paras.first().unwrap().node_range.start
            ..current_paras.last().unwrap().node_range.end;
        groups.push(ChildGroup::ParagraphRun {
            blocks: current_paras,
            range,
        });
    }

    groups
}

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
    let children = if let BlockContent::Children(c) = &block.content {
        c.clone()
    } else {
        vec![]
    };

    // Group consecutive paragraphs into runs
    let groups = group_children(&children);

    // Check which paragraph run (if any) contains the focused paragraph
    let current_focus = focused_anchor_id.read();
    let focused_run_idx = current_focus.as_ref().and_then(|focused_id| {
        groups.iter().position(|group| {
            if let ChildGroup::ParagraphRun { blocks, .. } = group {
                blocks.iter().any(|b| &b.id == focused_id)
            } else {
                false
            }
        })
    });

    rsx! {
        blockquote {
            class: "block-quote",
            for (i, group) in groups.iter().enumerate() {
                if let ChildGroup::ParagraphRun { blocks, range } = group {
                    if focused_run_idx == Some(i) {
                        // This run is focused - show editor
                        {
                            let edit_range = range.clone();
                            let content_text = source.get(edit_range.clone()).unwrap_or("").to_string();
                            rsx! {
                                EditorBlock {
                                    key: "editor-{i}",
                                    block: block.clone(),
                                    content_text,
                                    edit_range: Some(edit_range),
                                    on_command,
                                    on_cancel: {
                                        let mut focused_anchor_id = focused_anchor_id;
                                        move |_| focused_anchor_id.set(None)
                                    }
                                }
                            }
                        }
                    } else {
                        // This run is not focused - show display mode
                        {
                            let first_block_id = blocks.first().map(|b| b.id).unwrap_or(block.id);
                            rsx! {
                                span {
                                    key: "run-{i}",
                                    class: "block-quote-content clickable-block",
                                    tabindex: "0",
                                    onclick: {
                                        let mut focused_anchor_id = focused_anchor_id;
                                        move |evt| {
                                            evt.stop_propagation();
                                            focused_anchor_id.set(Some(first_block_id))
                                        }
                                    },
                                    onkeydown: {
                                        let mut focused_anchor_id = focused_anchor_id;
                                        move |evt| {
                                            if evt.key() == Key::Enter {
                                                focused_anchor_id.set(Some(first_block_id));
                                            }
                                        }
                                    },
                                    for (j, para) in blocks.iter().enumerate() {
                                        p {
                                            key: "para-{j}",
                                            class: "block-quote-paragraph",
                                            InlineSegments {
                                                segments: para.segments.clone(),
                                                on_wikilink_click
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let ChildGroup::OtherBlock(child) = group {
                    // Non-paragraph children (nested quotes, lists, code, etc.)
                    BlockRenderer {
                        key: "child-{i}",
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
