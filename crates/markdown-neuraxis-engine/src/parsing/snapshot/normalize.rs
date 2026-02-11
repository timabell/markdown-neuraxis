use std::collections::BTreeMap;

use serde::Serialize;
use xi_rope::Rope;

use crate::parsing::{
    blocks::{BlockKind, BlockNode, ContainerFrame},
    parse_inline_for_block,
    rope::{slice::preview, span::Span},
};

#[derive(Serialize)]
pub struct Snap {
    pub blocks: Vec<BlockSnap>,
}

#[derive(Serialize)]
pub struct BlockSnap {
    pub kind: String,
    pub span: (usize, usize),
    pub containers: Vec<String>,
    pub text: String,
    pub inline: Vec<InlineSnap>,
}

#[derive(Serialize)]
pub struct InlineSnap {
    pub kind: String,
    pub span: (usize, usize),
    pub text: String,
    pub parts: BTreeMap<String, (usize, usize)>,
}

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

            let inline_nodes = parse_inline_for_block(rope, b);
            let inline = inline_nodes
                .into_iter()
                .map(|n| {
                    let mut parts = BTreeMap::new();
                    match n {
                        crate::parsing::inline::InlineNode::Text(sp) => InlineSnap {
                            kind: "Text".into(),
                            span: (sp.start, sp.end),
                            text: preview(rope, sp, 60),
                            parts,
                        },
                        crate::parsing::inline::InlineNode::CodeSpan { full, inner } => {
                            parts.insert("inner".into(), (inner.start, inner.end));
                            InlineSnap {
                                kind: "CodeSpan".into(),
                                span: (full.start, full.end),
                                text: preview(rope, full, 60),
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
                                text: preview(rope, full, 60),
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
