use crate::parsing::rope::span::Span;

use super::kinds::FenceKind;

/// A frame in the container stack representing a nesting level.
///
/// Containers wrap leaf blocks (paragraphs, code blocks) and can nest arbitrarily.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerFrame {
    /// A blockquote container with its nesting depth.
    BlockQuote {
        /// How many `>` prefixes (1 = single quote, 2 = nested, etc.)
        depth: u8,
    },
    // Later: List, ListItem, etc.
}

/// The kind of a leaf block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// A paragraph block (default when no other block opener matches).
    Paragraph,
    /// A fenced code block (``` or ~~~).
    FencedCode {
        /// Whether backticks or tildes were used.
        kind: FenceKind,
    },
}

/// A parsed block node with its containers, kind, and spans.
#[derive(Debug, Clone)]
pub struct BlockNode {
    /// The container stack this block is nested within.
    pub containers: Vec<ContainerFrame>,
    /// The kind of leaf block (Paragraph, FencedCode, etc.)
    pub kind: BlockKind,
    /// Full byte span of the block including delimiters.
    pub span: Span,
    /// Content span for inline parsing (excludes prefixes like `>`).
    pub content_span: Span,
}
