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
