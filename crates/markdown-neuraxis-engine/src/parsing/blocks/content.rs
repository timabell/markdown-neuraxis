//! Content projection types for handling nested prefix containers.
//!
//! These types enable GUI editing modes that can show or hide container prefixes
//! (like `>` for blockquotes) while maintaining lossless round-trip editing.

use crate::parsing::rope::span::Span;

/// A single line's content projection within a block.
///
/// Separates the container prefix (e.g., `> ` for blockquotes) from
/// the meaningful content, enabling GUI editing modes that can show
/// or hide prefixes.
///
/// # Invariants
///
/// - `prefix` and `content` are within `raw_line`
/// - `prefix.end <= content.start`
/// - `raw_line.start <= prefix.start`
/// - `content.end <= raw_line.end`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLine {
    /// Full physical line span in the rope.
    pub raw_line: Span,
    /// Container prefix region on this line (e.g., `> ` or list marker).
    pub prefix: Span,
    /// Remainder after stripping container prefixes.
    pub content: Span,
}

/// How a block's meaningful content is represented.
///
/// - `Contiguous`: Single span, no per-line prefix handling needed
/// - `Lines`: Content is non-contiguous due to per-line prefixes
///
/// # Usage
///
/// For blocks not inside any line-prefix container, use `ContentView::Contiguous`.
/// For blocks inside line-prefix containers (blockquotes, lists), use `ContentView::Lines`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentView {
    /// No per-line prefix semantics (block not inside prefix containers).
    Contiguous(Span),
    /// Content is non-contiguous; prefixes differ per line.
    Lines(Vec<ContentLine>),
}

impl ContentView {
    /// Returns true if this is a contiguous content view.
    #[must_use]
    pub fn is_contiguous(&self) -> bool {
        matches!(self, ContentView::Contiguous(_))
    }

    /// Returns true if this is a lines-based content view.
    #[must_use]
    pub fn is_lines(&self) -> bool {
        matches!(self, ContentView::Lines(_))
    }

    /// Joins content spans into a single string, separated by newlines.
    ///
    /// For `Contiguous`, slices the span directly.
    /// For `Lines`, joins each line's content span with `\n`.
    ///
    /// This is the canonical way to get the "without-prefix" view of a block's content.
    #[must_use]
    pub fn join_content(&self, rope: &xi_rope::Rope) -> String {
        use crate::parsing::rope::slice::slice_to_string;

        match self {
            ContentView::Contiguous(span) => slice_to_string(rope, *span),
            ContentView::Lines(lines) => {
                let mut result = String::new();
                for (i, line) in lines.iter().enumerate() {
                    result.push_str(&slice_to_string(rope, line.content));
                    if i < lines.len() - 1 {
                        result.push('\n');
                    }
                }
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_line_can_be_constructed() {
        let line = ContentLine {
            raw_line: Span { start: 0, end: 10 },
            prefix: Span { start: 0, end: 2 },
            content: Span { start: 2, end: 10 },
        };
        assert_eq!(line.raw_line.len(), 10);
        assert_eq!(line.prefix.len(), 2);
        assert_eq!(line.content.len(), 8);
    }

    #[test]
    fn content_view_contiguous_is_contiguous() {
        let view = ContentView::Contiguous(Span { start: 0, end: 10 });
        assert!(view.is_contiguous());
        assert!(!view.is_lines());
    }

    #[test]
    fn content_view_lines_is_lines() {
        let view = ContentView::Lines(vec![ContentLine {
            raw_line: Span { start: 0, end: 10 },
            prefix: Span { start: 0, end: 2 },
            content: Span { start: 2, end: 10 },
        }]);
        assert!(view.is_lines());
        assert!(!view.is_contiguous());
    }

    #[test]
    fn content_view_empty_lines() {
        let view = ContentView::Lines(vec![]);
        assert!(view.is_lines());
    }
}
