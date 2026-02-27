//! # SyntaxKind and Rowan Integration
//!
//! This module defines the fundamental building blocks for the syntax tree:
//!
//! - [`SyntaxKind`]: An enum of all possible token and node types
//! - [`MarkdownLang`]: The language definition that connects our kinds to Rowan
//! - Type aliases ([`SyntaxNode`], [`SyntaxToken`], [`SyntaxElement`]) for working with the tree
//!
//! ## Design: Single Enum for Tokens and Nodes
//!
//! Following the rust-analyzer model, we use a **single enum** for both tokens
//! (produced by the lexer) and composite nodes (produced by the parser). This
//! might seem unusual, but it has practical benefits:
//!
//! - Rowan stores kinds as `u16` internally, so they must fit in one type
//! - Pattern matching works uniformly across the tree
//! - Adding new syntax is just adding enum variants
//!
//! The enum is split into two sections:
//! 1. **Tokens** (up to and including `EOF`) - atomic units from the lexer
//! 2. **Nodes** (after `EOF`) - composite structures built by the parser
//!
//! ## SCREAMING_CASE Convention
//!
//! We use `SCREAMING_CASE` for variants (e.g., `BLOCK_QUOTE` not `BlockQuote`)
//! following rust-analyzer's convention. This is unusual for Rust enums but:
//! - Makes syntax kinds visually distinct from regular types
//! - Matches the style in rust-analyzer, our reference implementation
//! - The `#[allow(non_camel_case_types)]` attribute suppresses the warning
//!
//! ## Rowan Integration
//!
//! Rowan is a library for building lossless syntax trees. It needs to know:
//! 1. How to convert our `SyntaxKind` to/from its internal `rowan::SyntaxKind`
//! 2. What "language" we're parsing (via the [`MarkdownLang`] zero-sized type)
//!
//! The [`rowan::Language`] trait implementation handles this conversion.
//!
//! ## Example: Navigating a Tree
//!
//! ```
//! use markdown_neuraxis_syntax::{parse, SyntaxKind, SyntaxNode};
//!
//! let tree = parse("# Title\n");
//!
//! // Check the root
//! assert_eq!(tree.kind(), SyntaxKind::ROOT);
//!
//! // Find all headings
//! for child in tree.children() {
//!     if child.kind() == SyntaxKind::HEADING {
//!         println!("Found heading: {}", child.text());
//!     }
//! }
//! ```

/// All syntax kinds for the Markdown CST.
///
/// This enum represents both tokens (lexer output) and composite nodes (parser output).
/// The `repr(u16)` ensures efficient storage in rowan's green tree.
///
/// We use SCREAMING_CASE following the rust-analyzer convention for SyntaxKind.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // === Tokens (lexer output) ===
    /// Horizontal whitespace (spaces, tabs)
    WHITESPACE,
    /// Line ending
    NEWLINE,
    /// Plain text content
    TEXT,
    /// `>` character for blockquotes
    GT,
    /// `-` character for lists and thematic breaks
    DASH,
    /// `*` character for lists, emphasis, and thematic breaks
    STAR,
    /// `+` character for lists
    PLUS,
    /// Single backtick for code spans
    BACKTICK,
    /// `~` character for fenced code and strikethrough
    TILDE,
    /// `[` for links and wikilinks
    LBRACKET,
    /// `]` for links and wikilinks
    RBRACKET,
    /// `|` for wikilink aliases and tables
    PIPE,
    /// `(` for link URLs
    LPAREN,
    /// `)` for link URLs
    RPAREN,
    /// `#` for headings
    HASH,
    /// `<` for HTML blocks
    LT,
    /// `.` for numbered lists
    DOT,
    /// Raw HTML content
    HTML_TEXT,
    /// End of file marker
    EOF,

    // === Composite Nodes (parser output) ===
    /// Root document node
    ROOT,
    /// Blockquote container (`> ...`)
    BLOCK_QUOTE,
    /// List container (ordered or unordered)
    LIST,
    /// Individual list item
    LIST_ITEM,
    /// Paragraph block
    PARAGRAPH,
    /// ATX heading (`# ...`)
    HEADING,
    /// Thematic break (`---`, `***`, etc.)
    THEMATIC_BREAK,
    /// Fenced code block
    FENCED_CODE,
    /// Raw HTML block
    HTML_BLOCK,
    /// Inline content container
    INLINE,
    /// Wikilink (`[[target]]` or `[[target|alias]]`)
    WIKILINK,
    /// Inline code span
    CODE_SPAN,
    /// Standard link `[text](url)`
    LINK,
    /// Emphasis `*text*`
    EMPHASIS,
    /// Strong emphasis `**text**`
    STRONG,

    /// Error recovery node
    ERROR,
}

impl SyntaxKind {
    /// Returns true if this kind represents a token (lexer output).
    pub fn is_token(self) -> bool {
        (self as u16) <= (Self::EOF as u16)
    }

    /// Returns true if this kind represents a composite node.
    pub fn is_node(self) -> bool {
        !self.is_token()
    }

    /// Returns true if this kind is trivia (whitespace/newlines).
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::WHITESPACE | Self::NEWLINE)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

/// Language marker type for Rowan.
///
/// This is a zero-sized type (no variants = no size) that serves as a marker
/// to tell Rowan which [`SyntaxKind`] enum to use. Rowan is generic over
/// languages, so this type "brands" our trees as Markdown trees.
///
/// You'll rarely use this directly - it's mainly used in type aliases below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkdownLang {}

impl rowan::Language for MarkdownLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= SyntaxKind::ERROR as u16);
        // SAFETY: We check bounds above and SyntaxKind is repr(u16).
        // This assumes enum variants are contiguous starting from 0.
        // Adding variants in the middle would break serialized trees.
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// A syntax node in the Markdown CST.
///
/// Nodes are composite elements that contain other nodes and tokens.
/// For example, a `HEADING` node contains `HASH` tokens and `TEXT` tokens.
///
/// Nodes are cheap to clone (they're reference-counted internally) and
/// provide methods like:
/// - `kind()` - Get the [`SyntaxKind`]
/// - `text()` - Get all text under this node
/// - `children()` - Iterate over child nodes
/// - `children_with_tokens()` - Iterate over children including tokens
/// - `parent()` - Get the parent node
pub type SyntaxNode = rowan::SyntaxNode<MarkdownLang>;

/// A syntax token in the Markdown CST.
///
/// Tokens are the leaves of the tree - they contain actual text and have no
/// children. Every character in the source text belongs to exactly one token.
///
/// Tokens provide methods like:
/// - `kind()` - Get the [`SyntaxKind`]
/// - `text()` - Get the token's text
/// - `text_range()` - Get the byte range in the source
pub type SyntaxToken = rowan::SyntaxToken<MarkdownLang>;

/// Either a node or a token.
///
/// Useful when iterating with `children_with_tokens()` which yields both
/// nodes and tokens in tree order.
pub type SyntaxElement = rowan::SyntaxElement<MarkdownLang>;

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::Language;

    #[test]
    fn token_kinds_are_tokens() {
        assert!(SyntaxKind::WHITESPACE.is_token());
        assert!(SyntaxKind::TEXT.is_token());
        assert!(SyntaxKind::EOF.is_token());
    }

    #[test]
    fn node_kinds_are_nodes() {
        assert!(SyntaxKind::ROOT.is_node());
        assert!(SyntaxKind::PARAGRAPH.is_node());
        assert!(SyntaxKind::WIKILINK.is_node());
    }

    #[test]
    fn trivia_detection() {
        assert!(SyntaxKind::WHITESPACE.is_trivia());
        assert!(SyntaxKind::NEWLINE.is_trivia());
        assert!(!SyntaxKind::TEXT.is_trivia());
    }

    #[test]
    fn rowan_conversion_roundtrip() {
        let kind = SyntaxKind::PARAGRAPH;
        let raw: rowan::SyntaxKind = kind.into();
        let back = MarkdownLang::kind_from_raw(raw);
        assert_eq!(kind, back);
    }
}
