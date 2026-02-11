pub mod blocks;
pub mod inline;
pub mod rope;
pub mod snapshot;

use xi_rope::Rope;

use blocks::{BlockBuilder, BlockKind, BlockNode, MarkdownLineClassifier};
use rope::{lines_with_spans, slice::slice_to_string};

#[derive(Debug)]
pub struct ParsedDoc {
    pub blocks: Vec<BlockNode>,
}

pub fn parse_document(rope: &Rope) -> ParsedDoc {
    let classifier = MarkdownLineClassifier;
    let mut builder = BlockBuilder::new();

    for lr in lines_with_spans(rope) {
        let lc = classifier.classify(&lr);
        builder.push(&lc);
    }

    ParsedDoc {
        blocks: builder.finish(),
    }
}

/// Convenience: inline parse for a given block node (paragraphs only in this skeleton).
pub fn parse_inline_for_block(rope: &Rope, b: &BlockNode) -> Vec<inline::InlineNode> {
    if !matches!(b.kind, BlockKind::Paragraph) {
        return vec![];
    }
    let s = slice_to_string(rope, b.content_span);
    inline::parse_inline(b.content_span.start, &s)
}
