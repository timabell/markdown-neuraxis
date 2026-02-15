/// A byte range `[start, end)` into the rope.
///
/// All parsed nodes store spans rather than copied text, enabling lossless
/// round-trip: slicing the rope with any span reproduces the exact source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl Span {
    /// Returns the length in bytes. Uses saturating subtraction for safety.
    #[must_use]
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the span is empty (start >= end).
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_normal_span() {
        let span = Span { start: 5, end: 10 };
        assert_eq!(span.len(), 5);
    }

    #[test]
    fn len_zero_length_span() {
        let span = Span { start: 5, end: 5 };
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn len_inverted_span_saturates() {
        // When start > end, saturating_sub returns 0
        let span = Span { start: 10, end: 5 };
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn is_empty_zero_length() {
        let span = Span { start: 5, end: 5 };
        assert!(span.is_empty());
    }

    #[test]
    fn is_empty_inverted_span() {
        let span = Span { start: 10, end: 5 };
        assert!(span.is_empty());
    }

    #[test]
    fn is_empty_non_empty_span() {
        let span = Span { start: 0, end: 1 };
        assert!(!span.is_empty());
    }
}
