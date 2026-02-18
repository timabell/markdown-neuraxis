use crate::parsing::rope::span::Span;

use super::content::ContentView;
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
///
/// The `content` field provides a view of the block's meaningful content
/// that separates container prefixes from actual content. This enables
/// GUI editing modes that can show or hide prefixes.
#[derive(Debug, Clone)]
pub struct BlockNode {
    /// The container stack this block is nested within.
    pub containers: Vec<ContainerFrame>,
    /// The kind of leaf block (Paragraph, FencedCode, etc.)
    pub kind: BlockKind,
    /// Full byte span of the block including delimiters.
    pub span: Span,
    /// Content projection for inline parsing and editing.
    ///
    /// For blocks inside containers (blockquotes, lists), this is
    /// `ContentView::Lines` with per-line prefix/content separation.
    /// For blocks not in containers, this is `ContentView::Contiguous`.
    pub content: ContentView,
}
