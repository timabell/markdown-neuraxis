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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_short_text_unchanged() {
        let rope = Rope::from("hello");
        let sp = Span { start: 0, end: 5 };
        assert_eq!(preview(&rope, sp, 10), "hello");
    }

    #[test]
    fn preview_exact_length_unchanged() {
        let rope = Rope::from("hello");
        let sp = Span { start: 0, end: 5 };
        assert_eq!(preview(&rope, sp, 5), "hello");
    }

    #[test]
    fn preview_truncates_long_text() {
        let rope = Rope::from("hello world");
        let sp = Span { start: 0, end: 11 };
        assert_eq!(preview(&rope, sp, 5), "hello...");
    }

    #[test]
    fn preview_truncates_to_zero() {
        let rope = Rope::from("hello");
        let sp = Span { start: 0, end: 5 };
        assert_eq!(preview(&rope, sp, 0), "...");
    }

    #[test]
    fn slice_to_string_full_span() {
        let rope = Rope::from("hello world");
        let sp = Span { start: 0, end: 11 };
        assert_eq!(slice_to_string(&rope, sp), "hello world");
    }

    #[test]
    fn slice_to_string_partial_span() {
        let rope = Rope::from("hello world");
        let sp = Span { start: 6, end: 11 };
        assert_eq!(slice_to_string(&rope, sp), "world");
    }
}
