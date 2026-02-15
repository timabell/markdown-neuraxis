use xi_rope::Rope;

use crate::parsing::blocks::BlockNode;

/// Validates parser output invariants.
///
/// Asserts that:
/// - All block spans are within rope bounds
/// - All content spans are within rope bounds
/// - Content spans are contained within their block spans
///
/// # Panics
/// Panics with a descriptive message if any invariant is violated.
pub fn check(rope: &Rope, blocks: &[BlockNode]) {
    let n = rope.len();
    for b in blocks {
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
    }
}
