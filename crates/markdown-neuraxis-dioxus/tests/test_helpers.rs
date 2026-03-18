//! Shared test helpers for working with the new snapshot structure

use markdown_neuraxis_engine::editing::AnchorId;
use markdown_neuraxis_engine::editing::snapshot::{Block, BlockContent};

/// A flattened block with extracted content string
#[derive(Debug, Clone)]
pub struct FlatBlock {
    pub content: String,
    pub id: AnchorId,
}

/// Flatten hierarchical blocks into a list of FlatBlocks
pub fn flatten_blocks(blocks: &[Block], source: &str) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    flatten_blocks_recursive(blocks, source, &mut out);
    out
}

fn flatten_blocks_recursive(blocks: &[Block], source: &str, out: &mut Vec<FlatBlock>) {
    for block in blocks {
        let content: String = block
            .lines
            .iter()
            .map(|line| &source[line.content.clone()])
            .collect::<Vec<_>>()
            .join("\n");

        out.push(FlatBlock {
            content,
            id: block.id,
        });

        if let BlockContent::Children(children) = &block.content {
            flatten_blocks_recursive(children, source, out);
        }
    }
}
