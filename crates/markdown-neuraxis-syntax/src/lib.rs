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
    use insta::assert_snapshot;

    /// Helper to format a syntax tree for snapshot testing.
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

    #[test]
    fn snapshot_simple_paragraph() {
        let tree = parse("Hello, world!\n");
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_heading() {
        let tree = parse("# Hello\n");
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_nested_content() {
        let input = "# Heading\n\n> A quote with [[wikilink]]\n\n- List item\n";
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_fenced_code() {
        let input = "```rust\nfn main() {}\n```\n";
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_inline_elements() {
        let input = "Text with `code` and *emphasis* and **strong**.\n";
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_wikilink_with_alias() {
        let input = "See [[target|display text]] for more.\n";
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_standard_link() {
        let input = "Click [here](https://example.com) to visit.\n";
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    #[test]
    fn snapshot_complex_document() {
        let input = r#"# Main Title

This is a paragraph with [[wikilinks]] and [regular links](url).

## Code Example

```rust
fn main() {
    println!("Hello");
}
```

> A blockquote with *emphasis*.

- First item
- Second item
- Third with `code`

---

Final paragraph.
"#;
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
    }

    // === Error tolerance / messy input tests ===
    // Real-world notes are messy. These test that we produce a valid tree
    // even for garbage input, preserving all bytes.

    #[test]
    fn snapshot_messy_unclosed_constructs() {
        // Simulates half-finished edits: unclosed links, wikilinks, emphasis
        let input = r#"# Draft notes

Check out [[this page for more info

Also see [broken link without url

Some *half done emphasis

And `unclosed code span

More text here.
"#;
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
        // Critical: all bytes preserved even for garbage
        assert_eq!(tree.text().to_string(), input);
    }

    #[test]
    fn snapshot_messy_real_world_notes() {
        // Simulates a messy daily journal imported from various tools:
        // - Inconsistent heading styles
        // - Mixed list markers
        // - Broken wikilinks from copy/paste
        // - Random HTML fragments from web clipper
        // - Unclosed fenced code block
        // - Trailing whitespace (common in editors)
        let input = r#"#Meeting Notes 2024-01-15
(no space after #, technically not a heading per CommonMark)

##Action Items
- [ ] Call [[John] about project
- [x] Review PR #123
* mixed bullet style
+ another style
  - nested but inconsistent indent

> half finished blockquote
that continues without >

Some <b>html that's not closed

```python
def broken():
    # oops forgot to close the fence

Random [[wikilink|with pipe]] and [[broken one

---

TODO: fix [[
"#;
        let tree = parse(input);
        assert_snapshot!(format_tree(&tree, 0));
        assert_eq!(tree.text().to_string(), input);
    }

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
