//! # markdown-neuraxis-syntax
//!
//! A lossless Markdown syntax tree using [Rowan] + [Logos], following the
//! [rust-analyzer] architecture model.
//!
//! [Rowan]: https://docs.rs/rowan
//! [Logos]: https://docs.rs/logos
//! [rust-analyzer]: https://rust-analyzer.github.io/book/contributing/syntax.html
//!
//! ## What is a Lossless CST?
//!
//! Unlike an Abstract Syntax Tree (AST) which discards formatting details, a
//! Concrete Syntax Tree (CST) preserves **every byte** of the original source:
//! whitespace, comments, formatting choices - everything. This enables:
//!
//! - **Structural editing**: Modify the tree and serialize back to text without
//!   losing the user's formatting preferences
//! - **Accurate error reporting**: Span information maps exactly to source positions
//! - **Refactoring tools**: Move code around while preserving style
//!
//! ## Architecture Overview
//!
//! The parsing pipeline has three stages:
//!
//! ```text
//! Source Text → Lexer → Tokens → Parser → Events → Sink → Rowan Tree
//!               (Logos)          (Grammar)        (GreenNodeBuilder)
//! ```
//!
//! ### 1. Lexer ([`lexer`] module)
//!
//! The lexer uses [Logos] to tokenize input into a flat sequence of tokens.
//! Every character becomes part of some token - nothing is discarded.
//!
//! ```text
//! "# Hello\n" → [HASH, WHITESPACE, TEXT("Hello"), NEWLINE]
//! ```
//!
//! ### 2. Parser ([`parser`] module)
//!
//! The parser consumes tokens and emits **events** (Start, Token, Finish).
//! It uses a **marker system** to safely build nested structures without
//! recursion limits. Grammar rules live in [`parser::grammar`].
//!
//! ```text
//! Tokens → Events: [Start(HEADING), Token(HASH), Token(WHITESPACE),
//!                   Token(TEXT), Token(NEWLINE), Finish]
//! ```
//!
//! ### 3. Sink ([`parser::sink`] module)
//!
//! The sink consumes events and builds a Rowan green tree using
//! `GreenNodeBuilder`. The resulting tree is immutable and can be
//! cheaply cloned (it's reference-counted internally).
//!
//! ## Module Structure
//!
//! ```text
//! markdown-neuraxis-syntax/
//! ├── lib.rs           # This file - public API and integration tests
//! ├── syntax_kind.rs   # SyntaxKind enum (tokens + nodes) and Rowan integration
//! ├── lexer.rs         # Logos-based tokenizer
//! └── parser/
//!     ├── mod.rs       # Parser struct, Marker system, public parse() function
//!     ├── event.rs     # Event enum (Start, Token, Finish, Placeholder)
//!     ├── sink.rs      # Converts events to Rowan GreenNode
//!     └── grammar/
//!         ├── mod.rs   # Root document parsing
//!         ├── block.rs # Block-level elements (headings, lists, code blocks)
//!         └── inline.rs# Inline elements (links, emphasis, code spans)
//! ```
//!
//! ## Quick Start
//!
//! ```
//! use markdown_neuraxis_syntax::{parse, SyntaxKind};
//!
//! let tree = parse("# Hello\n");
//!
//! // The tree preserves all text
//! assert_eq!(tree.text().to_string(), "# Hello\n");
//!
//! // Navigate the tree structure
//! assert_eq!(tree.kind(), SyntaxKind::ROOT);
//! let heading = tree.children().next().unwrap();
//! assert_eq!(heading.kind(), SyntaxKind::HEADING);
//! ```
//!
//! ## Why This Architecture?
//!
//! This design is battle-tested in rust-analyzer, which parses millions of
//! lines of Rust code. Key benefits:
//!
//! - **Error tolerance**: Malformed input produces a valid (if imperfect) tree
//! - **Incremental potential**: Rowan supports incremental reparsing
//! - **Memory efficient**: Green nodes are interned and shared
//! - **Type safe**: The marker system prevents tree corruption at compile time
//!
//! ## Further Reading
//!
//! - [ADR-12: Rowan Parser Rewrite](../../doc/adr/0012-rowan-parser-rewrite.md) -
//!   The architectural decision record explaining why we chose this approach
//! - [rust-analyzer syntax docs](https://rust-analyzer.github.io/book/contributing/syntax.html) -
//!   The reference implementation we're following
//! - [Rowan crate docs](https://docs.rs/rowan) - The underlying tree library

pub mod lexer;
pub mod parser;
pub mod syntax_kind;

pub use parser::parse;
pub use syntax_kind::{MarkdownLang, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

#[cfg(test)]
mod tests {
    use super::*;

    /// Format a syntax tree for snapshot testing.
    fn format_tree(node: &SyntaxNode, indent: usize) -> String {
        let mut result = String::new();
        let prefix = "  ".repeat(indent);

        result.push_str(&format!(
            "{}{:?}@{:?}\n",
            prefix,
            node.kind(),
            node.text_range()
        ));

        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => {
                    result.push_str(&format_tree(&n, indent + 1));
                }
                rowan::NodeOrToken::Token(t) => {
                    let text = t.text().replace('\n', "\\n");
                    result.push_str(&format!(
                        "{}  {:?}@{:?} {:?}\n",
                        prefix,
                        t.kind(),
                        t.text_range(),
                        text
                    ));
                }
            }
        }

        result
    }

    /// Run a snapshot test for a given .md file.
    /// Called by generated test functions (see build.rs).
    fn snapshot_test(name: &str) {
        let snapshot_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/snapshots");
        let input_path = snapshot_dir.join(format!("{name}.md"));
        let input = std::fs::read_to_string(&input_path).unwrap();
        let tree = parse(&input);

        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path(&snapshot_dir);
        settings.bind(|| {
            insta::assert_snapshot!(name, format_tree(&tree, 0));
        });

        // All parses must preserve input bytes (lossless)
        assert_eq!(tree.text().to_string(), input, "{name}: roundtrip failed");
    }

    // Generated by build.rs - one test per .md file in snapshots/
    include!(concat!(env!("OUT_DIR"), "/snapshot_tests.rs"));

    #[test]
    fn roundtrip_preserves_text() {
        let inputs = [
            "Hello, world!\n",
            "# Heading\n",
            "> Quote\n",
            "- Item\n",
            "```\ncode\n```\n",
            "[[wikilink]]\n",
            "[link](url)\n",
            "*emphasis*\n",
            "**strong**\n",
            "`code span`\n",
        ];

        for input in inputs {
            let tree = parse(input);
            assert_eq!(
                tree.text().to_string(),
                input,
                "Roundtrip failed for: {:?}",
                input
            );
        }
    }
}
