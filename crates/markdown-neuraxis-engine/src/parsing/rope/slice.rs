use xi_rope::Rope;

use super::span::Span;

/// Extracts the text for a span from the rope as an owned String.
///
/// This allocates; prefer working with spans where possible.
pub fn slice_to_string(rope: &Rope, sp: Span) -> String {
    rope.slice_to_cow(sp.start..sp.end).into_owned()
}

/// Extracts text for a span, truncating to `max` bytes with "..." suffix if needed.
///
/// Used for human-readable snapshot output.
pub fn preview(rope: &Rope, sp: Span, max: usize) -> String {
    let mut s = slice_to_string(rope, sp);
    if s.len() > max {
        s.truncate(max);
        s.push_str("...");
    }
    s
}
