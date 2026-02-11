use crate::parsing::rope::{lines::LineRef, span::Span};

use super::kinds::{BlockQuote, CodeFence, FenceSig};

#[derive(Debug, Clone)]
pub struct LineClass {
    pub line: Span,
    pub is_blank: bool,

    pub quote_depth: u8,
    pub remainder_span: Span, // bytes in rope after stripping quote prefixes
    pub remainder_text: String, // scaffold: remainder string

    pub fence_sig: Option<FenceSig>, // "looks like a fence" on remainder
}

pub struct MarkdownLineClassifier;

impl MarkdownLineClassifier {
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
