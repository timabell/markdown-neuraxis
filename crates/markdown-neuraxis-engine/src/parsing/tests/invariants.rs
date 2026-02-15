use xi_rope::Rope;

use crate::parsing::blocks::BlockNode;

/// Validates parser output invariants.
///
/// Asserts that:
/// - All block spans are within rope bounds
/// - All content spans are within rope bounds
/// - Content spans are contained within their block spans
/// - Blocks are ordered (each block starts at or after previous ends)
/// - No overlapping spans between sibling blocks
///
/// # Panics
/// Panics with a descriptive message if any invariant is violated.
pub fn check(rope: &Rope, blocks: &[BlockNode]) {
    let n = rope.len();
    let mut prev_end: Option<usize> = None;

    for (i, b) in blocks.iter().enumerate() {
        assert!(
            b.span.start <= b.span.end && b.span.end <= n,
            "block span out of bounds: {:?} (rope len: {})",
            b.span,
            n
        );
        assert!(
            b.content_span.start <= b.content_span.end && b.content_span.end <= n,
            "content span out of bounds: {:?} (rope len: {})",
            b.content_span,
            n
        );
        assert!(
            b.content_span.start >= b.span.start && b.content_span.end <= b.span.end,
            "content span not contained in block span: content {:?}, block {:?}",
            b.content_span,
            b.span
        );

        // Check blocks are ordered: each starts at or after the previous ends
        if let Some(pe) = prev_end {
            assert!(
                b.span.start >= pe,
                "block {} starts at {} before previous block ends at {} (blocks not ordered)",
                i,
                b.span.start,
                pe
            );
        }

        // Check no overlap with previous block
        if let Some(pe) = prev_end {
            assert!(
                b.span.start >= pe,
                "block {} overlaps with previous block: block starts at {}, previous ends at {}",
                i,
                b.span.start,
                pe
            );
        }

        prev_end = Some(b.span.end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::parse_document;

    #[test]
    fn spans_are_ordered_simple_paragraphs() {
        let md = "First paragraph.\n\nSecond paragraph.\n";
        let rope = Rope::from(md);
        let doc = parse_document(&rope);

        // Should not panic
        check(&rope, &doc.blocks);

        // Verify ordering manually
        assert!(doc.blocks.len() >= 2);
        assert!(doc.blocks[1].span.start >= doc.blocks[0].span.end);
    }

    #[test]
    fn spans_are_ordered_mixed_blocks() {
        let md = "Paragraph.\n\n```\ncode\n```\n\nAnother paragraph.\n";
        let rope = Rope::from(md);
        let doc = parse_document(&rope);

        // Should not panic
        check(&rope, &doc.blocks);

        // Verify no overlaps
        for i in 1..doc.blocks.len() {
            assert!(
                doc.blocks[i].span.start >= doc.blocks[i - 1].span.end,
                "Block {} overlaps with block {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn no_overlapping_spans_with_quotes() {
        let md = "> Quote 1\n\n> Quote 2\n";
        let rope = Rope::from(md);
        let doc = parse_document(&rope);

        // Should not panic
        check(&rope, &doc.blocks);
    }

    #[test]
    fn empty_document_passes_invariants() {
        let rope = Rope::from("");
        let doc = parse_document(&rope);

        // Should not panic with empty blocks
        check(&rope, &doc.blocks);
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn single_block_passes_invariants() {
        let rope = Rope::from("Single paragraph");
        let doc = parse_document(&rope);

        check(&rope, &doc.blocks);
        assert_eq!(doc.blocks.len(), 1);
    }
}
