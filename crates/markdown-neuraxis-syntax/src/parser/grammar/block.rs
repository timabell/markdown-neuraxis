//! Block-level grammar rules.

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

use super::inline;

/// Parse a block element.
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
        SyntaxKind::WHITESPACE => {
            // Indented content - could be continuation or indented code
            // For now, treat as paragraph
            paragraph(p);
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

/// Parse a blockquote.
fn blockquote(p: &mut Parser<'_, '_>) {
    let m = p.start();

    while p.at(SyntaxKind::GT) {
        // Consume `>` and optional space
        p.bump();
        p.eat(SyntaxKind::WHITESPACE);

        // Parse content until end of line
        inline::inline_until_newline(p);

        // Consume newline
        if p.eat(SyntaxKind::NEWLINE) {
            // Check for continuation line
            // Skip leading whitespace
            while p.at(SyntaxKind::WHITESPACE) {
                p.bump();
            }

            if !p.at(SyntaxKind::GT) {
                break;
            }
        } else {
            break;
        }
    }

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
}
