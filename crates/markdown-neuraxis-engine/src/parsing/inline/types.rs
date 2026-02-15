use crate::parsing::rope::span::Span;

/// A parsed inline node with byte spans into the rope.
///
/// All variants store spans rather than text, enabling lossless round-trip.
#[derive(Debug, Clone)]
pub enum InlineNode {
    /// Plain text that isn't part of any special construct.
    Text(Span),
    /// A code span (backtick-delimited). This is a "raw zone" - no parsing inside.
    CodeSpan {
        /// Full span including backticks.
        full: Span,
        /// Inner span (content between backticks).
        inner: Span,
    },
    /// A wiki-style link `[[target]]` or `[[target|alias]]`.
    WikiLink {
        /// Full span including `[[` and `]]`.
        full: Span,
        /// Span of the target (page name).
        target: Span,
        /// Span of the alias if present (after `|`).
        alias: Option<Span>,
    },
}
