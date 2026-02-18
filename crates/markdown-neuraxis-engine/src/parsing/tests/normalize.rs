use std::collections::BTreeMap;

use serde::Serialize;
use xi_rope::Rope;

use crate::parsing::{
    blocks::{BlockKind, BlockNode, ContainerFrame, ContentView},
    parse_inline_for_block,
    rope::{slice::preview, span::Span},
};

/// Snapshot of a parsed document for testing with `insta`.
///
/// Contains a serializable representation of all blocks and their inline content.
#[derive(Serialize)]
pub struct Snap {
    /// All blocks in the document.
    pub blocks: Vec<BlockSnap>,
}

/// Snapshot of a single block for testing.
#[derive(Serialize)]
pub struct BlockSnap {
    /// Block kind as a string (e.g., "Paragraph", "FencedCode(Backticks)").
    pub kind: String,
    /// Byte span as (start, end) tuple.
    pub span: (usize, usize),
    /// Container stack as string labels (e.g., ["Quote(1)"]).
    pub containers: Vec<String>,
    /// Content view type and details.
    pub content: ContentSnap,
    /// Preview of block text (truncated for readability).
    pub text: String,
    /// Inline nodes within this block.
    pub inline: Vec<InlineSnap>,
}

/// Snapshot of content view for testing.
#[derive(Serialize)]
pub struct ContentSnap {
    /// Content view type: "Contiguous" or "Lines".
    pub view_type: String,
    /// For Contiguous: the content span. For Lines: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_span: Option<(usize, usize)>,
    /// For Lines: the per-line details. For Contiguous: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<ContentLineSnap>>,
}

/// Snapshot of a single content line for testing.
#[derive(Serialize)]
pub struct ContentLineSnap {
    /// Full physical line span.
    pub raw_line: (usize, usize),
    /// Container prefix span.
    pub prefix: (usize, usize),
    /// Content span after prefix.
    pub content: (usize, usize),
    /// Preview of prefix text.
    pub prefix_text: String,
    /// Preview of content text.
    pub content_text: String,
}

/// Snapshot of a single inline node for testing.
#[derive(Serialize)]
pub struct InlineSnap {
    /// Node kind as a string (e.g., "Text", "WikiLink", "CodeSpan").
    pub kind: String,
    /// Byte span as (start, end) tuple.
    pub span: (usize, usize),
    /// Preview of node text (truncated for readability).
    pub text: String,
    /// Sub-spans by name (e.g., "target", "alias", "inner").
    pub parts: BTreeMap<String, (usize, usize)>,
}

/// Converts parsed blocks into a serializable snapshot for testing.
///
/// Includes block kinds, spans, containers, content view, and inline content with sub-spans.
pub fn normalize(rope: &Rope, blocks: &[BlockNode]) -> Snap {
    let blocks = blocks
        .iter()
        .map(|b| {
            let kind = match &b.kind {
                BlockKind::Paragraph => "Paragraph".to_string(),
                BlockKind::FencedCode { kind } => format!("FencedCode({kind:?})"),
            };

            let containers = b
                .containers
                .iter()
                .map(|c| match c {
                    ContainerFrame::BlockQuote { depth } => format!("Quote({depth})"),
                })
                .collect::<Vec<_>>();

            let content = match &b.content {
                ContentView::Contiguous(span) => ContentSnap {
                    view_type: "Contiguous".to_string(),
                    content_span: Some((span.start, span.end)),
                    lines: None,
                },
                ContentView::Lines(lines) => ContentSnap {
                    view_type: "Lines".to_string(),
                    content_span: None,
                    lines: Some(
                        lines
                            .iter()
                            .map(|line| ContentLineSnap {
                                raw_line: (line.raw_line.start, line.raw_line.end),
                                prefix: (line.prefix.start, line.prefix.end),
                                content: (line.content.start, line.content.end),
                                prefix_text: preview(rope, line.prefix, 30),
                                content_text: preview(rope, line.content, 60),
                            })
                            .collect(),
                    ),
                },
            };

            // For Lines blocks, inline spans are virtual positions in joined content
            let joined_content = b.content.join_content(rope);
            let is_lines = b.content.is_lines();

            let inline_nodes = parse_inline_for_block(rope, b);
            let inline = inline_nodes
                .into_iter()
                .map(|n| {
                    let mut parts = BTreeMap::new();

                    // Helper to preview text: use joined content for Lines, rope for Contiguous
                    let preview_text = |start: usize, end: usize, max_len: usize| -> String {
                        if is_lines {
                            let text = &joined_content[start..end.min(joined_content.len())];
                            if text.len() > max_len {
                                format!("{}...", &text[..max_len])
                            } else {
                                text.to_string()
                            }
                        } else {
                            preview(rope, Span { start, end }, max_len)
                        }
                    };

                    match n {
                        crate::parsing::inline::InlineNode::Text(sp) => InlineSnap {
                            kind: "Text".into(),
                            span: (sp.start, sp.end),
                            text: preview_text(sp.start, sp.end, 60),
                            parts,
                        },
                        crate::parsing::inline::InlineNode::CodeSpan { full, inner } => {
                            parts.insert("inner".into(), (inner.start, inner.end));
                            InlineSnap {
                                kind: "CodeSpan".into(),
                                span: (full.start, full.end),
                                text: preview_text(full.start, full.end, 60),
                                parts,
                            }
                        }
                        crate::parsing::inline::InlineNode::WikiLink {
                            full,
                            target,
                            alias,
                        } => {
                            parts.insert("target".into(), (target.start, target.end));
                            if let Some(a) = alias {
                                parts.insert("alias".into(), (a.start, a.end));
                            }
                            InlineSnap {
                                kind: "WikiLink".into(),
                                span: (full.start, full.end),
                                text: preview_text(full.start, full.end, 60),
                                parts,
                            }
                        }
                    }
                })
                .collect();

            BlockSnap {
                kind,
                span: (b.span.start, b.span.end),
                containers,
                content,
                text: preview(
                    rope,
                    Span {
                        start: b.span.start,
                        end: b.span.end,
                    },
                    80,
                ),
                inline,
            }
        })
        .collect();

    Snap { blocks }
}
