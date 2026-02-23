//! Lossless Markdown syntax tree using Rowan + Logos.
//!
//! This crate provides a concrete syntax tree (CST) for Markdown that:
//! - Preserves all whitespace and formatting
//! - Supports structural editing
//! - Handles MDNX-specific extensions (wikilinks)
//! - Follows the rust-analyzer architecture model

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
