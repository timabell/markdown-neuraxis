use crate::parsing::rope::{lines::LineRef, span::Span};

use super::kinds::{BlockQuote, CodeFence, FenceSig};

/// Classification of a single line containing only local facts.
///
/// This is phase 1 of block parsing: each line is classified independently
/// without reference to surrounding context.
#[derive(Debug, Clone)]
pub struct LineClass {
    /// Full byte span of this line in the rope.
    pub line: Span,
    /// Whether the line is blank (whitespace only after stripping prefixes).
    pub is_blank: bool,
    /// Number of blockquote `>` prefixes found.
    pub quote_depth: u8,
    /// Byte span of the line content after stripping quote prefixes.
    pub remainder_span: Span,
    /// Text content after stripping prefixes. Scaffold: will be zero-copy later.
    pub remainder_text: String,
    /// If the remainder looks like a fence opener/closer.
    pub fence_sig: Option<FenceSig>,
}

/// Classifies individual lines for the block parsing phase.
pub struct MarkdownLineClassifier;

impl MarkdownLineClassifier {
    /// Classifies a line into a [`LineClass`] containing local facts.
    ///
    /// Extracts blockquote depth, remainder span, blank status, and fence signature.
    pub fn classify(&self, lr: &LineRef) -> LineClass {
        let trimmed = lr.text.trim_end_matches(['\r', '\n']);
        let is_blank = trimmed.trim().is_empty();

        let (qd, idx) = BlockQuote::strip_prefixes(trimmed);
        let remainder = &trimmed[idx..];
        let remainder_span = Span {
            start: lr.span.start + idx,
            end: lr.span.start + trimmed.len(),
        };

        LineClass {
            line: lr.span,
            is_blank,
            quote_depth: qd,
            remainder_span,
            remainder_text: remainder.to_string(),
            fence_sig: CodeFence::sig(remainder),
        }
    }
}
