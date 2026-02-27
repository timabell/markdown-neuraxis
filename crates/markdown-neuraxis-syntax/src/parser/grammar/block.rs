//! # Block-Level Grammar
//!
//! Block elements are the structural building blocks of a Markdown document.
//! They're identified by patterns at the **start of a line**:
//!
//! | Pattern | Block Type |
//! |---------|-----------|
//! | `# ` | Heading |
//! | `> ` | Blockquote |
//! | `- `, `* `, `+ ` | List item |
//! | `---`, `***` | Thematic break |
//! | ``` ` ` ` ``` | Fenced code block |
//! | (anything else) | Paragraph |
//!
//! ## Dispatch Logic
//!
//! The main [`block`] function looks at the first token of a line and
//! dispatches to the appropriate handler. Some patterns are ambiguous:
//!
//! - `*` could start a list item OR a thematic break OR emphasis in a paragraph
//! - We use lookahead (`is_thematic_break`, `is_code_fence`) to disambiguate
//!
//! ## Current Limitations
//!
//! This is a TDD exploration, so some features aren't implemented yet:
//!
//! - **Nested containers**: Lists inside blockquotes, blockquotes inside lists
//! - **LIST grouping**: Consecutive list items aren't wrapped in a LIST node
//! - **Indented code blocks**: Currently treated as paragraphs
//! - **Setext headings**: Only ATX (`#`) headings are supported

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

use super::inline;

/// Parse a single block element.
///
/// This is the main dispatch function for block parsing. It skips blank lines,
/// then examines the first token to determine the block type.
pub fn block(p: &mut Parser<'_, '_>) {
    // Skip leading blank lines
    while p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }

    if p.at_end() {
        return;
    }

    // Detect block type at line start
    match p.current() {
        SyntaxKind::HASH => heading(p),
        SyntaxKind::GT => blockquote(p),
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            // Could be list item or thematic break
            if is_thematic_break(p) {
                thematic_break(p);
            } else {
                list_item(p);
            }
        }
        SyntaxKind::BACKTICK | SyntaxKind::TILDE => {
            if is_code_fence(p) {
                fenced_code(p);
            } else {
                paragraph(p);
            }
        }
        SyntaxKind::TEXT => {
            // Could be a numbered list item (e.g., "1. item")
            if is_numbered_list_item(p) {
                list_item_numbered(p);
            } else {
                paragraph(p);
            }
        }
        SyntaxKind::WHITESPACE => {
            // Indented content - could be nested list item or continuation
            if is_indented_list_item(p) {
                list_item_indented(p);
            } else {
                paragraph(p);
            }
        }
        _ => paragraph(p),
    }
}

/// Check if current position is a thematic break (---, ***, etc.)
fn is_thematic_break(p: &Parser<'_, '_>) -> bool {
    let marker = p.current();
    if !matches!(marker, SyntaxKind::DASH | SyntaxKind::STAR) {
        return false;
    }

    // Need at least 3 markers
    let mut count = 0;
    let mut i = 0;

    while p.nth(i) != SyntaxKind::EOF && p.nth(i) != SyntaxKind::NEWLINE {
        match p.nth(i) {
            k if k == marker => count += 1,
            SyntaxKind::WHITESPACE => {}
            _ => return false,
        }
        i += 1;
    }

    count >= 3
}

/// Check if current position starts a code fence.
fn is_code_fence(p: &Parser<'_, '_>) -> bool {
    let marker = p.current();
    if !matches!(marker, SyntaxKind::BACKTICK | SyntaxKind::TILDE) {
        return false;
    }

    // Count consecutive markers
    let mut count = 0;
    let mut i = 0;

    while p.nth(i) == marker {
        count += 1;
        i += 1;
    }

    count >= 3
}

/// Parse an ATX heading.
fn heading(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume hash marks
    while p.at(SyntaxKind::HASH) {
        p.bump();
    }

    // Consume optional space after hashes
    p.eat(SyntaxKind::WHITESPACE);

    // Parse inline content until end of line
    inline::inline_until_newline(p);

    // Consume the newline if present
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::HEADING);
}

/// Parse a blockquote line.
///
/// Each line starting with `>` creates a BLOCK_QUOTE node.
/// Multiple `>` markers (e.g., `>>`) create nested BLOCK_QUOTE nodes.
/// The snapshot layer consolidates consecutive same-depth blockquotes.
fn blockquote(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume first `>`
    p.bump();
    p.eat(SyntaxKind::WHITESPACE);

    // Check for additional `>` markers (nested blockquote)
    if p.at(SyntaxKind::GT) {
        // Recurse for nested blockquote
        blockquote(p);
    } else {
        // Parse content until end of line
        inline::inline_until_newline(p);
    }

    // Consume newline
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::BLOCK_QUOTE);
}

/// Parse a list item.
fn list_item(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume the marker (-, *, +)
    p.bump();

    // Consume required space after marker
    if !p.eat(SyntaxKind::WHITESPACE) {
        // Not a valid list item, treat as paragraph
        m.abandon(p);
        return paragraph(p);
    }

    // Parse inline content
    inline::inline_until_newline(p);

    // Consume newline
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::LIST_ITEM);
}

/// Parse a thematic break.
fn thematic_break(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume all tokens until newline
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }

    // Consume the newline
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::THEMATIC_BREAK);
}

/// Check if current position is a numbered list item (e.g., "1. ")
fn is_numbered_list_item(p: &Parser<'_, '_>) -> bool {
    // Must start with TEXT containing only digits
    if p.current() != SyntaxKind::TEXT {
        return false;
    }

    let text = p.current_text();
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Next must be DOT, then WHITESPACE
    p.nth(1) == SyntaxKind::DOT && p.nth(2) == SyntaxKind::WHITESPACE
}

/// Parse a numbered list item (e.g., "1. item")
fn list_item_numbered(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume the number
    p.bump();

    // Consume the dot
    p.bump();

    // Consume the required space
    if !p.eat(SyntaxKind::WHITESPACE) {
        m.abandon(p);
        return paragraph(p);
    }

    // Parse inline content
    inline::inline_until_newline(p);

    // Consume newline
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::LIST_ITEM);
}

/// Check if current whitespace precedes an indented list item
fn is_indented_list_item(p: &Parser<'_, '_>) -> bool {
    if p.current() != SyntaxKind::WHITESPACE {
        return false;
    }

    // Look for list marker after whitespace
    let after_ws = p.nth(1);
    match after_ws {
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            // Bullet list: whitespace + marker + whitespace
            p.nth(2) == SyntaxKind::WHITESPACE
        }
        SyntaxKind::TEXT => {
            // Numbered list: whitespace + digits + dot + whitespace
            p.nth(2) == SyntaxKind::DOT && p.nth(3) == SyntaxKind::WHITESPACE
        }
        _ => false,
    }
}

/// Parse an indented list item (nested list)
fn list_item_indented(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume leading whitespace (indentation)
    while p.at(SyntaxKind::WHITESPACE) {
        p.bump();
    }

    // Parse based on marker type
    match p.current() {
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            // Consume the marker
            p.bump();

            // Consume required space
            if !p.eat(SyntaxKind::WHITESPACE) {
                m.abandon(p);
                return paragraph(p);
            }
        }
        SyntaxKind::TEXT => {
            // Numbered list - consume number, dot, space
            p.bump(); // number
            p.bump(); // dot
            if !p.eat(SyntaxKind::WHITESPACE) {
                m.abandon(p);
                return paragraph(p);
            }
        }
        _ => {
            m.abandon(p);
            return paragraph(p);
        }
    }

    // Parse inline content
    inline::inline_until_newline(p);

    // Consume newline
    p.eat(SyntaxKind::NEWLINE);

    m.complete(p, SyntaxKind::LIST_ITEM);
}

/// Parse a fenced code block.
fn fenced_code(p: &mut Parser<'_, '_>) {
    let m = p.start();

    let fence_marker = p.current();

    // Count opening fence length
    let mut fence_len = 0;
    while p.at(fence_marker) {
        p.bump();
        fence_len += 1;
    }

    // Parse info string (language)
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }

    // Consume newline after opening fence
    p.eat(SyntaxKind::NEWLINE);

    // Parse content until closing fence
    loop {
        if p.at_end() {
            break;
        }

        // Check for closing fence at start of line
        if p.at(fence_marker) {
            let mut close_len = 0;

            // Peek ahead to count fence markers
            while p.nth(close_len) == fence_marker {
                close_len += 1;
            }

            if close_len >= fence_len {
                // This is the closing fence - consume it
                for _ in 0..close_len {
                    p.bump();
                }
                // Consume rest of line
                while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
                    p.bump();
                }
                p.eat(SyntaxKind::NEWLINE);
                break;
            }
        }

        // Not a closing fence, consume the line
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        p.eat(SyntaxKind::NEWLINE);
    }

    m.complete(p, SyntaxKind::FENCED_CODE);
}

/// Parse a paragraph (default block).
fn paragraph(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume until blank line or block-level construct
    loop {
        inline::inline_until_newline(p);

        if !p.eat(SyntaxKind::NEWLINE) {
            break;
        }

        // Check for paragraph break (blank line or new block)
        if p.at_end() || p.at(SyntaxKind::NEWLINE) {
            break;
        }

        // Check for block-level constructs that interrupt paragraphs
        match p.current() {
            SyntaxKind::HASH | SyntaxKind::GT => break,
            SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
                // Only break if it looks like a list item (marker + space)
                if p.nth(1) == SyntaxKind::WHITESPACE {
                    break;
                }
            }
            SyntaxKind::BACKTICK | SyntaxKind::TILDE => {
                if is_code_fence(p) {
                    break;
                }
            }
            _ => {}
        }

        // Continue paragraph - but we already consumed the newline, so just loop
    }

    m.complete(p, SyntaxKind::PARAGRAPH);
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use crate::syntax_kind::SyntaxKind;

    #[test]
    fn parse_heading() {
        let tree = parse("# Heading\n");
        assert_eq!(tree.kind(), SyntaxKind::ROOT);

        let heading = tree.children().next().unwrap();
        assert_eq!(heading.kind(), SyntaxKind::HEADING);
        assert!(heading.text().to_string().contains("Heading"));
    }

    #[test]
    fn parse_h2_heading() {
        let tree = parse("## Second\n");
        let heading = tree.children().next().unwrap();
        assert_eq!(heading.kind(), SyntaxKind::HEADING);
    }

    #[test]
    fn parse_blockquote() {
        let tree = parse("> quoted text\n");
        let bq = tree.children().next().unwrap();
        assert_eq!(bq.kind(), SyntaxKind::BLOCK_QUOTE);
    }

    #[test]
    fn parse_list_item() {
        let tree = parse("- item\n");
        let item = tree.children().next().unwrap();
        assert_eq!(item.kind(), SyntaxKind::LIST_ITEM);
    }

    #[test]
    fn parse_thematic_break() {
        let tree = parse("---\n");
        let hr = tree.children().next().unwrap();
        assert_eq!(hr.kind(), SyntaxKind::THEMATIC_BREAK);
    }

    #[test]
    fn parse_fenced_code() {
        let tree = parse("```rust\ncode\n```\n");
        let code = tree.children().next().unwrap();
        assert_eq!(code.kind(), SyntaxKind::FENCED_CODE);
    }

    #[test]
    fn parse_paragraph() {
        let tree = parse("Just some text.\n");
        let para = tree.children().next().unwrap();
        assert_eq!(para.kind(), SyntaxKind::PARAGRAPH);
    }

    #[test]
    fn parse_multiple_blocks() {
        let input = "# Heading\n\nParagraph text.\n\n- list item\n";
        let tree = parse(input);

        let blocks: Vec<_> = tree.children().collect();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].kind(), SyntaxKind::HEADING);
        assert_eq!(blocks[1].kind(), SyntaxKind::PARAGRAPH);
        assert_eq!(blocks[2].kind(), SyntaxKind::LIST_ITEM);
    }

    #[test]
    fn text_preservation() {
        let input = "# Heading\n\n> Quote\n\n- Item\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    // === Numbered list tests ===

    #[test]
    fn parse_numbered_list_item() {
        let tree = parse("1. First item\n");
        let item = tree.children().next().unwrap();
        assert_eq!(item.kind(), SyntaxKind::LIST_ITEM);
        assert!(item.text().to_string().contains("First item"));
    }

    #[test]
    fn parse_numbered_list_multi_digit() {
        let tree = parse("10. Tenth item\n");
        let item = tree.children().next().unwrap();
        assert_eq!(item.kind(), SyntaxKind::LIST_ITEM);
    }

    #[test]
    fn parse_numbered_list_preserves_text() {
        let input = "1. First\n2. Second\n3. Third\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);

        let items: Vec<_> = tree.children().collect();
        assert_eq!(items.len(), 3);
        for item in items {
            assert_eq!(item.kind(), SyntaxKind::LIST_ITEM);
        }
    }

    // === Nested list tests ===

    #[test]
    fn parse_nested_bullet_list() {
        let input = "- Parent\n  - Child\n";
        let tree = parse(input);

        let items: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::LIST_ITEM)
            .collect();
        assert_eq!(items.len(), 2, "Should have parent and child list items");
    }

    #[test]
    fn parse_nested_list_multiple_levels() {
        let input = "- Level 1\n  - Level 2\n    - Level 3\n";
        let tree = parse(input);

        let items: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::LIST_ITEM)
            .collect();
        assert_eq!(items.len(), 3, "Should have 3 nested list items");
    }

    #[test]
    fn parse_nested_list_with_tabs() {
        let input = "- Parent\n\t- Child\n";
        let tree = parse(input);

        let items: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::LIST_ITEM)
            .collect();
        assert_eq!(items.len(), 2, "Should recognize tab-indented child");
    }

    #[test]
    fn parse_nested_numbered_list() {
        let input = "1. Parent\n   1. Child\n";
        let tree = parse(input);

        let items: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::LIST_ITEM)
            .collect();
        assert_eq!(
            items.len(),
            2,
            "Should have parent and child numbered items"
        );
    }

    #[test]
    fn parse_nested_list_preserves_text() {
        let input = "- Parent\n  - Child\n    - Grandchild\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    // === Blockquote nesting tests ===

    #[test]
    fn parse_single_blockquote_line() {
        let tree = parse("> Quote\n");
        let bq = tree.children().next().unwrap();
        assert_eq!(bq.kind(), SyntaxKind::BLOCK_QUOTE);
    }

    #[test]
    fn parse_nested_blockquote() {
        let input = ">> Nested\n";
        let tree = parse(input);

        // Should have nested BLOCK_QUOTE structure
        let bqs: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::BLOCK_QUOTE)
            .collect();
        assert_eq!(bqs.len(), 2, "Should have 2 blockquotes (outer and nested)");
    }

    #[test]
    fn parse_deeply_nested_blockquote() {
        let input = ">>> Deep\n";
        let tree = parse(input);

        let bqs: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::BLOCK_QUOTE)
            .collect();
        assert_eq!(bqs.len(), 3, "Should have 3 blockquote levels");
    }

    #[test]
    fn parse_multiple_blockquote_lines() {
        let input = "> Line 1\n> Line 2\n";
        let tree = parse(input);

        // Each line should be a separate BLOCK_QUOTE at root level
        let bqs: Vec<_> = tree
            .children()
            .filter(|n| n.kind() == SyntaxKind::BLOCK_QUOTE)
            .collect();
        assert_eq!(bqs.len(), 2, "Should have 2 separate blockquote nodes");
    }

    #[test]
    fn parse_blockquote_preserves_text() {
        let input = "> Line 1\n>> Nested\n>>> Deep\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }
}
