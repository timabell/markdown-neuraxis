use crate::parsing::rope::span::Span;

#[derive(Debug, Clone)]
pub enum InlineNode {
    Text(Span),
    CodeSpan {
        full: Span,
        inner: Span,
    },
    WikiLink {
        full: Span,
        target: Span,
        alias: Option<Span>,
    },
}
