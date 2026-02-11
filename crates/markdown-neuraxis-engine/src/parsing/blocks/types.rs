use crate::parsing::rope::span::Span;

use super::kinds::FenceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerFrame {
    BlockQuote { depth: u8 },
    // Later: List, ListItem, etc.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    FencedCode { kind: FenceKind },
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub containers: Vec<ContainerFrame>,
    pub kind: BlockKind,
    pub span: Span,
    pub content_span: Span, // what inline parser should see
}
