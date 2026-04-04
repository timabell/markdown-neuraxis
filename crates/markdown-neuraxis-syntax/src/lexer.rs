//! # Lexer - Tokenizing Markdown Source
//!
//! This module provides the first stage of parsing: breaking source text into
//! tokens using the [Logos] lexer generator.
//!
//! [Logos]: https://docs.rs/logos
//!
//! ## The Lossless Guarantee
//!
//! The most important property of this lexer is that **every byte in the input
//! appears in exactly one token**. We never skip or discard characters. This
//! is what makes round-tripping possible:
//!
//! ```
//! use markdown_neuraxis_syntax::lexer::lex;
//!
//! let input = "# Hello, world!\n";
//! let tokens = lex(input);
//!
//! // Concatenating all token texts gives back the original
//! let reconstructed: String = tokens.iter().map(|t| t.text).collect();
//! assert_eq!(input, reconstructed);
//! ```
//!
//! ## Why Two Token Enums?
//!
//! You'll notice we have both [`TokenKind`] (in this module) and [`SyntaxKind`]
//! (in the syntax_kind module). This is because:
//!
//! 1. **Logos requires its own enum** for the `#[derive(Logos)]` macro
//! 2. **Rowan uses our SyntaxKind** for the final tree
//!
//! The [`TokenKind::to_syntax_kind`] method converts between them.
//!
//! ## Token Design Philosophy
//!
//! Tokens are kept **minimal and context-free**. The lexer doesn't know if `*`
//! starts a list, emphasis, or a thematic break - that's the parser's job.
//! This separation keeps the lexer simple and fast.
//!
//! Special characters that have syntactic meaning get their own token types:
//! - `#` → `HASH` (headings)
//! - `>` → `GT` (blockquotes)
//! - `-`, `*`, `+` → `DASH`, `STAR`, `PLUS` (lists, emphasis, thematic breaks)
//! - `[`, `]`, `(`, `)` → bracket tokens (links)
//! - `` ` ``, `~` → `BACKTICK`, `TILDE` (code, fenced blocks)
//!
//! Everything else becomes `TEXT` tokens, grouped into runs of consecutive
//! characters for efficiency (e.g., "Hello" is one TEXT token, not five).
//!
//! ## Public API
//!
//! - [`lex`] - Tokenize input, returning `Vec<Token>`
//! - [`lex_with_spans`] - Tokenize with byte offset spans
//! - [`Token`] - A token with its kind and text slice
//!
//! [`SyntaxKind`]: crate::syntax_kind::SyntaxKind

use logos::Logos;

use crate::syntax_kind::SyntaxKind;

/// Token kinds produced by the Logos lexer.
///
/// This enum exists separately from [`SyntaxKind`] because Logos needs to
/// derive on it. Each variant maps to a corresponding `SyntaxKind` token.
///
/// The `#[logos(skip r"")]` attribute means "skip nothing" - we explicitly
/// handle all input rather than letting Logos skip anything.
///
/// [`SyntaxKind`]: crate::syntax_kind::SyntaxKind
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"")]
pub enum TokenKind {
    /// Horizontal whitespace (spaces, tabs)
    #[regex(r"[ \t]+")]
    Whitespace,

    /// Line ending (LF or CRLF)
    #[regex(r"\r?\n")]
    Newline,

    /// `>` for blockquotes
    #[token(">")]
    Gt,

    /// `-` for lists and thematic breaks
    #[token("-")]
    Dash,

    /// `*` for lists, emphasis, thematic breaks
    #[token("*")]
    Star,

    /// `+` for lists
    #[token("+")]
    Plus,

    /// Single backtick
    #[token("`")]
    Backtick,

    /// Tilde for fenced code
    #[token("~")]
    Tilde,

    /// `[` for links
    #[token("[")]
    LBracket,

    /// `]` for links
    #[token("]")]
    RBracket,

    /// `|` for wikilink aliases
    #[token("|")]
    Pipe,

    /// `(` for link URLs
    #[token("(")]
    LParen,

    /// `)` for link URLs
    #[token(")")]
    RParen,

    /// `#` for headings
    #[token("#")]
    Hash,

    /// `<` for HTML blocks
    #[token("<")]
    Lt,

    /// `.` for numbered lists
    #[token(".")]
    Dot,

    /// `_` for underscore emphasis
    #[token("_")]
    Underscore,

    /// `!` for images
    #[token("!")]
    Exclaim,

    /// `:` for properties
    #[token(":")]
    Colon,

    /// `=` for setext headings
    #[token("=")]
    Equals,

    /// Plain text - anything not matched by other rules
    #[regex(r"[^\s\[\]()>`*+#|~.<_!:=-]+")]
    Text,
}

impl TokenKind {
    /// Convert to SyntaxKind.
    pub fn to_syntax_kind(self) -> SyntaxKind {
        match self {
            TokenKind::Whitespace => SyntaxKind::WHITESPACE,
            TokenKind::Newline => SyntaxKind::NEWLINE,
            TokenKind::Gt => SyntaxKind::GT,
            TokenKind::Dash => SyntaxKind::DASH,
            TokenKind::Star => SyntaxKind::STAR,
            TokenKind::Plus => SyntaxKind::PLUS,
            TokenKind::Backtick => SyntaxKind::BACKTICK,
            TokenKind::Tilde => SyntaxKind::TILDE,
            TokenKind::LBracket => SyntaxKind::LBRACKET,
            TokenKind::RBracket => SyntaxKind::RBRACKET,
            TokenKind::Pipe => SyntaxKind::PIPE,
            TokenKind::LParen => SyntaxKind::LPAREN,
            TokenKind::RParen => SyntaxKind::RPAREN,
            TokenKind::Hash => SyntaxKind::HASH,
            TokenKind::Lt => SyntaxKind::LT,
            TokenKind::Dot => SyntaxKind::DOT,
            TokenKind::Underscore => SyntaxKind::UNDERSCORE,
            TokenKind::Exclaim => SyntaxKind::EXCLAIM,
            TokenKind::Colon => SyntaxKind::COLON,
            TokenKind::Equals => SyntaxKind::EQUALS,
            TokenKind::Text => SyntaxKind::TEXT,
        }
    }
}

/// A lexed token with its kind and text slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: SyntaxKind,
    pub text: &'a str,
}

/// Lex the input into a sequence of tokens.
///
/// Guarantees that all bytes from the input appear in the output tokens.
pub fn lex(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut lexer = TokenKind::lexer(input);

    while let Some(result) = lexer.next() {
        let text = lexer.slice();
        let kind = match result {
            Ok(token_kind) => token_kind.to_syntax_kind(),
            Err(()) => {
                // Logos error means unrecognized character - treat as TEXT
                SyntaxKind::TEXT
            }
        };
        tokens.push(Token { kind, text });
    }

    tokens
}

/// Lex and return tokens along with their byte spans.
pub fn lex_with_spans(input: &str) -> Vec<(Token<'_>, std::ops::Range<usize>)> {
    let mut tokens = Vec::new();
    let mut lexer = TokenKind::lexer(input);

    while let Some(result) = lexer.next() {
        let span = lexer.span();
        let text = lexer.slice();
        let kind = match result {
            Ok(token_kind) => token_kind.to_syntax_kind(),
            Err(()) => SyntaxKind::TEXT,
        };
        tokens.push((Token { kind, text }, span));
    }

    tokens
}
