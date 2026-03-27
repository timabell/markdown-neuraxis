//! Shared test helpers for working with the new snapshot structure

use markdown_neuraxis_engine::editing::AnchorId;
use markdown_neuraxis_engine::editing::snapshot::{Block, BlockContent, InlineNode, InlineSegment};

/// A flattened block with extracted content string
#[derive(Debug, Clone)]
pub struct FlatBlock {
    pub content: String,
    pub id: AnchorId,
}

/// Flatten hierarchical blocks into a list of FlatBlocks
pub fn flatten_blocks(blocks: &[Block]) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    flatten_blocks_recursive(blocks, &mut out);
    out
}

fn flatten_blocks_recursive(blocks: &[Block], out: &mut Vec<FlatBlock>) {
    for block in blocks {
        // Extract plain text from segments
        let content = segments_to_plain_text(&block.segments);

        out.push(FlatBlock {
            content,
            id: block.id,
        });

        if let BlockContent::Children(children) = &block.content {
            flatten_blocks_recursive(children, out);
        }
    }
}

/// Extract plain text from segments (test helper)
fn segments_to_plain_text(segments: &[InlineSegment]) -> String {
    segments
        .iter()
        .map(|seg| inline_node_to_text(&seg.kind))
        .collect()
}

/// Recursively extract plain text from an inline node
fn inline_node_to_text(node: &InlineNode) -> String {
    match node {
        InlineNode::Text(s) => s.clone(),
        InlineNode::Strong(children) | InlineNode::Emphasis(children) => {
            children.iter().map(inline_node_to_text).collect()
        }
        InlineNode::Code(s) => s.clone(),
        InlineNode::Strikethrough(s) => s.clone(),
        InlineNode::WikiLink { target, alias } => alias.as_ref().unwrap_or(target).clone(),
        InlineNode::Link { text, .. } => text.clone(),
        InlineNode::Image { alt, .. } => alt.clone(),
        InlineNode::HardBreak => "\n".to_string(),
    }
}
