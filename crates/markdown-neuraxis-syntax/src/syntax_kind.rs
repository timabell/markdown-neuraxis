//! SyntaxKind enum for all tokens and nodes in the Markdown CST.
//!
//! Following the rust-analyzer model, all tokens and nodes share a single enum.
//! Every byte in the source must appear as a token in the tree.

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

/// Language definition for rowan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkdownLang {}

impl rowan::Language for MarkdownLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= SyntaxKind::ERROR as u16);
        // SAFETY: We check bounds above and SyntaxKind is repr(u16)
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// Type alias for our syntax nodes.
pub type SyntaxNode = rowan::SyntaxNode<MarkdownLang>;
/// Type alias for our syntax tokens.
pub type SyntaxToken = rowan::SyntaxToken<MarkdownLang>;
/// Type alias for syntax elements (node or token).
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
