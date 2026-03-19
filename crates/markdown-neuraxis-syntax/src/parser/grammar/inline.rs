//! # Inline-Level Grammar
//!
//! Inline elements are the formatting within blocks: links, emphasis, code spans.
//! Unlike blocks, inline parsing is driven by **special characters** rather than
//! line-start patterns.
//!
//! ## Dispatch Logic
//!
//! The [`inline_element`] function checks the current token:
//!
//! | Token | Possible Element |
//! |-------|-----------------|
//! | `[` | Link or wikilink |
//! | `` ` `` | Code span |
//! | `*` | Emphasis or strong |
//! | (other) | Plain text |
//!
//! ## Wikilinks vs Standard Links
//!
//! We support both:
//! - **Wikilinks**: `[[page]]` or `[[page|display text]]` (MDNX extension)
//! - **Standard links**: `[text](url)` (CommonMark)
//!
//! Disambiguation: if we see `[[` (two brackets), it's a wikilink. Otherwise,
//! we try to parse a standard link and fall back to plain text.
//!
//! ## Error Tolerance
//!
//! Inline parsing is lenient:
//! - Unclosed `[[` still produces a WIKILINK node (containing the unclosed content)
//! - `[text]` without `(url)` becomes an INLINE node (bracket as plain text)
//! - Unmatched `*` is consumed as plain text
//!
//! This ensures we always produce a valid tree that preserves all bytes.
//!
//! ## Supported Syntax
//!
//! - Emphasis: `*em*`, `_em_`, `**strong**`, `__strong__`
//! - Strikethrough: `~~text~~`
//! - Images: `![alt](url)`
//! - Autolinks: `<https://url>`
//! - Goal references: `((uuid))` (MDNX extension)
//! - Properties: `name:: value` (MDNX extension)

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

/// Parse inline content until newline or EOF.
///
/// This is the main entry point called by block parsers. It consumes tokens
/// until it hits a newline, dispatching to specific inline element handlers.
pub fn inline_until_newline(p: &mut Parser<'_, '_>) {
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        inline_element(p);
    }
}

/// Parse a single inline element.
fn inline_element(p: &mut Parser<'_, '_>) {
    match p.current() {
        SyntaxKind::LBRACKET => {
            // Could be wikilink [[...]] or standard link [...]()
            if p.nth(1) == SyntaxKind::LBRACKET {
                wikilink(p);
            } else {
                link_or_text(p);
            }
        }
        SyntaxKind::BACKTICK => code_span(p),
        SyntaxKind::STAR => emphasis_or_strong(p, SyntaxKind::STAR),
        SyntaxKind::UNDERSCORE => emphasis_or_strong(p, SyntaxKind::UNDERSCORE),
        SyntaxKind::TILDE => strikethrough(p),
        SyntaxKind::EXCLAIM => {
            // Could be image ![alt](url)
            if p.nth(1) == SyntaxKind::LBRACKET {
                image(p);
            } else {
                p.bump();
            }
        }
        SyntaxKind::LPAREN => {
            // Could be goal reference ((uuid))
            if p.nth(1) == SyntaxKind::LPAREN {
                block_ref(p);
            } else {
                p.bump();
            }
        }
        SyntaxKind::LT => autolink(p),
        SyntaxKind::TEXT => {
            // Check for property pattern: TEXT COLON COLON
            if p.nth(1) == SyntaxKind::COLON && p.nth(2) == SyntaxKind::COLON {
                property(p);
            } else {
                p.bump();
            }
        }
        _ => {
            // Plain text - just consume the token
            p.bump();
        }
    }
}

/// Parse a wikilink: [[target]] or [[target|alias]]
fn wikilink(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume opening [[
    debug_assert!(p.at(SyntaxKind::LBRACKET));
    p.bump(); // [
    p.bump(); // [

    // Consume content until ]] or newline
    let mut found_close = false;
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::RBRACKET) && p.nth(1) == SyntaxKind::RBRACKET {
            p.bump(); // ]
            p.bump(); // ]
            found_close = true;
            break;
        }
        p.bump();
    }

    if found_close {
        m.complete(p, SyntaxKind::WIKILINK);
    } else {
        // Unclosed wikilink - treat as error but keep the node
        m.complete(p, SyntaxKind::WIKILINK);
    }
}

/// Parse a standard link [text](url) or plain text.
fn link_or_text(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume opening [
    debug_assert!(p.at(SyntaxKind::LBRACKET));
    p.bump();

    // Consume text until ]
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::RBRACKET) {
        // Handle nested inline elements in link text
        match p.current() {
            SyntaxKind::BACKTICK => code_span(p),
            SyntaxKind::STAR => emphasis_or_strong(p, SyntaxKind::STAR),
            SyntaxKind::UNDERSCORE => emphasis_or_strong(p, SyntaxKind::UNDERSCORE),
            _ => p.bump(),
        }
    }

    // Check for ]
    if !p.eat(SyntaxKind::RBRACKET) {
        // Unclosed bracket - just text
        m.complete(p, SyntaxKind::INLINE);
        return;
    }

    // Check for (url)
    if p.at(SyntaxKind::LPAREN) {
        p.bump(); // (

        // Consume URL until )
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::RPAREN) {
            p.bump();
        }

        if p.eat(SyntaxKind::RPAREN) {
            m.complete(p, SyntaxKind::LINK);
        } else {
            m.complete(p, SyntaxKind::INLINE);
        }
    } else {
        // Just [text] without (url) - treat as inline
        m.complete(p, SyntaxKind::INLINE);
    }
}

/// Parse a code span `code`.
fn code_span(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Count opening backticks
    let mut open_count = 0;
    while p.at(SyntaxKind::BACKTICK) {
        p.bump();
        open_count += 1;
    }

    // Track whether we find content and closing backticks
    let mut has_content = false;
    let mut found_close = false;

    // Parse content until matching backticks
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::BACKTICK) {
            // Count consecutive backticks
            let mut close_count = 0;

            while p.nth(close_count) == SyntaxKind::BACKTICK {
                close_count += 1;
            }

            if close_count == open_count {
                // Matching close - consume the backticks
                for _ in 0..close_count {
                    p.bump();
                }
                found_close = true;
                break;
            } else {
                // Not matching - consume one backtick and continue
                p.bump();
                has_content = true;
            }
        } else {
            p.bump();
            has_content = true;
        }
    }

    // Only produce CODE_SPAN if properly closed with content
    if found_close && has_content {
        m.complete(p, SyntaxKind::CODE_SPAN);
    } else {
        // Unclosed or empty - abandon marker, tokens become plain text
        m.abandon(p);
    }
}

/// Look ahead to see if there's a matching close for a nested construct.
/// Used to decide whether `ahead_count` delimiters start a nested construct
/// or should be used to close the current one.
fn has_matching_close(p: &Parser<'_, '_>, delimiter: SyntaxKind, ahead_count: usize) -> bool {
    // We want to nest with `ahead_count` delimiters. But we can only consume
    // up to 2 (max for strong). So the nested construct would use min(ahead_count, 2).
    let nested_count = ahead_count.min(2);

    // Scan ahead (past the current delimiter run) looking for a matching close
    let mut offset = ahead_count; // Start after the delimiter run
    loop {
        let token = p.nth(offset);
        if token == SyntaxKind::NEWLINE || token == SyntaxKind::EOF {
            return false; // No matching close found
        }
        if token == delimiter {
            // Count consecutive delimiters at this position
            let mut count = 0;
            while p.nth(offset + count) == delimiter {
                count += 1;
            }
            if count >= nested_count {
                return true; // Found matching close
            }
            offset += count;
        } else {
            offset += 1;
        }
    }
}

/// Parse emphasis *text* or strong **text** (or underscore variants).
fn emphasis_or_strong(p: &mut Parser<'_, '_>, delimiter: SyntaxKind) {
    let m = p.start();

    // Count opening delimiters
    let mut open_count = 0;
    while p.at(delimiter) && open_count < 2 {
        p.bump();
        open_count += 1;
    }

    if open_count == 0 {
        m.abandon(p);
        return;
    }

    // Track whether we find content and closing delimiters
    let mut has_content = false;
    let mut found_close = false;

    // Parse content until matching delimiters
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(delimiter) {
            // Count all consecutive delimiters ahead
            let mut ahead_count = 0;
            while p.nth(ahead_count) == delimiter {
                ahead_count += 1;
            }

            if ahead_count < open_count {
                // Fewer delimiters - definitely nested (emphasis inside strong)
                emphasis_or_strong(p, delimiter);
                has_content = true;
            } else if ahead_count == open_count {
                // Exact match - close
                for _ in 0..open_count {
                    p.bump();
                }
                found_close = true;
                break;
            } else {
                // More delimiters - look ahead for matching close
                // If we find a matching close for nested, nest. Otherwise close.
                if has_matching_close(p, delimiter, ahead_count) {
                    emphasis_or_strong(p, delimiter);
                    has_content = true;
                } else {
                    // No matching close for nested - close current with open_count
                    for _ in 0..open_count {
                        p.bump();
                    }
                    found_close = true;
                    break;
                }
            }
        } else {
            p.bump();
            has_content = true;
        }
    }

    // Only produce EMPHASIS/STRONG if properly closed with content
    // Otherwise the downstream content range calculation would be invalid
    if found_close && has_content {
        let kind = if open_count >= 2 {
            SyntaxKind::STRONG
        } else {
            SyntaxKind::EMPHASIS
        };
        m.complete(p, kind);
    } else {
        // Unclosed or empty - abandon marker, tokens become plain text
        m.abandon(p);
    }
}

/// Parse image ![alt](url).
fn image(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume !
    debug_assert!(p.at(SyntaxKind::EXCLAIM));
    p.bump();

    // Consume [
    debug_assert!(p.at(SyntaxKind::LBRACKET));
    p.bump();

    // Consume alt text until ]
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::RBRACKET) {
        p.bump();
    }

    // Check for ]
    if !p.eat(SyntaxKind::RBRACKET) {
        m.complete(p, SyntaxKind::INLINE);
        return;
    }

    // Check for (url)
    if p.at(SyntaxKind::LPAREN) {
        p.bump(); // (

        // Consume URL until )
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::RPAREN) {
            p.bump();
        }

        if p.eat(SyntaxKind::RPAREN) {
            m.complete(p, SyntaxKind::IMAGE);
        } else {
            m.complete(p, SyntaxKind::INLINE);
        }
    } else {
        // Just ![text] without (url) - treat as inline
        m.complete(p, SyntaxKind::INLINE);
    }
}

/// Parse property `name:: value`.
fn property(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume property name (TEXT)
    debug_assert!(p.at(SyntaxKind::TEXT));
    p.bump();

    // Consume ::
    debug_assert!(p.at(SyntaxKind::COLON));
    p.bump();
    debug_assert!(p.at(SyntaxKind::COLON));
    p.bump();

    // Consume optional whitespace after ::
    p.eat(SyntaxKind::WHITESPACE);

    // Consume value until end of line
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }

    m.complete(p, SyntaxKind::PROPERTY);
}

/// Parse autolink <url>.
fn autolink(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume opening <
    debug_assert!(p.at(SyntaxKind::LT));
    p.bump();

    // Consume content until >
    let mut found_close = false;
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::GT) {
            p.bump();
            found_close = true;
            break;
        }
        p.bump();
    }

    if found_close {
        m.complete(p, SyntaxKind::AUTOLINK);
    } else {
        // Unclosed < - treat as inline text
        m.complete(p, SyntaxKind::INLINE);
    }
}

/// Parse goal reference ((uuid)).
fn block_ref(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume opening ((
    debug_assert!(p.at(SyntaxKind::LPAREN));
    p.bump(); // (
    debug_assert!(p.at(SyntaxKind::LPAREN));
    p.bump(); // (

    // Consume content until ))
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::RPAREN) && p.nth(1) == SyntaxKind::RPAREN {
            // Found closing ))
            p.bump(); // )
            p.bump(); // )
            break;
        }
        p.bump();
    }

    m.complete(p, SyntaxKind::BLOCK_REF);
}

/// Parse strikethrough ~~text~~.
fn strikethrough(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Count opening tildes (need exactly 2)
    let mut open_count = 0;
    while p.at(SyntaxKind::TILDE) && open_count < 2 {
        p.bump();
        open_count += 1;
    }

    if open_count < 2 {
        // Not enough tildes for strikethrough - abandon marker
        m.abandon(p);
        return;
    }

    // Track whether we find content and closing tildes
    let mut has_content = false;
    let mut found_close = false;

    // Parse content until matching ~~
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::TILDE) && p.nth(1) == SyntaxKind::TILDE {
            // Found closing ~~
            p.bump(); // ~
            p.bump(); // ~
            found_close = true;
            break;
        }
        p.bump();
        has_content = true;
    }

    // Only produce STRIKETHROUGH if properly closed with content
    if found_close && has_content {
        m.complete(p, SyntaxKind::STRIKETHROUGH);
    } else {
        // Unclosed or empty - abandon marker, tokens become plain text
        m.abandon(p);
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use crate::syntax_kind::SyntaxKind;

    fn find_node(
        tree: &crate::syntax_kind::SyntaxNode,
        kind: SyntaxKind,
    ) -> Option<crate::syntax_kind::SyntaxNode> {
        if tree.kind() == kind {
            return Some(tree.clone());
        }
        for child in tree.children() {
            if let Some(found) = find_node(&child, kind) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn parse_wikilink() {
        let tree = parse("See [[page]].\n");
        let wikilink = find_node(&tree, SyntaxKind::WIKILINK).unwrap();
        assert!(wikilink.text().to_string().contains("page"));
    }

    #[test]
    fn parse_wikilink_with_alias() {
        let tree = parse("See [[page|display text]].\n");
        let wikilink = find_node(&tree, SyntaxKind::WIKILINK).unwrap();
        let text = wikilink.text().to_string();
        assert!(text.contains("page"));
        assert!(text.contains("display text"));
    }

    #[test]
    fn parse_standard_link() {
        let tree = parse("Click [here](https://example.com).\n");
        let link = find_node(&tree, SyntaxKind::LINK).unwrap();
        let text = link.text().to_string();
        assert!(text.contains("here"));
        assert!(text.contains("example.com"));
    }

    #[test]
    fn parse_code_span() {
        let tree = parse("Use `code` here.\n");
        let code = find_node(&tree, SyntaxKind::CODE_SPAN).unwrap();
        assert!(code.text().to_string().contains("code"));
    }

    #[test]
    fn parse_double_backtick_code_span() {
        let tree = parse("Use ``code with ` backtick`` here.\n");
        let code = find_node(&tree, SyntaxKind::CODE_SPAN).unwrap();
        assert!(code.text().to_string().contains("backtick"));
    }

    #[test]
    fn parse_emphasis() {
        let tree = parse("This is *emphasized* text.\n");
        let em = find_node(&tree, SyntaxKind::EMPHASIS).unwrap();
        assert!(em.text().to_string().contains("emphasized"));
    }

    #[test]
    fn parse_emphasis_at_eof_no_newline() {
        // Bug repro: emphasis at EOF without trailing newline
        // Previously this was parsed as a list (STAR treated as list marker)
        let tree = parse("*emphasis*");
        let em = find_node(&tree, SyntaxKind::EMPHASIS).unwrap();
        assert_eq!(em.text().to_string(), "*emphasis*");
    }

    #[test]
    fn parse_strong() {
        let tree = parse("This is **strong** text.\n");
        let strong = find_node(&tree, SyntaxKind::STRONG).unwrap();
        assert!(strong.text().to_string().contains("strong"));
    }

    #[test]
    fn inline_preserves_text() {
        let input = "Text with [[wikilink]] and [link](url) and `code` and *em*.\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    // === Phase 1: Underscore emphasis ===

    #[test]
    fn parse_underscore_emphasis() {
        let tree = parse("This is _emphasized_ text.\n");
        let em = find_node(&tree, SyntaxKind::EMPHASIS).unwrap();
        assert!(em.text().to_string().contains("emphasized"));
    }

    #[test]
    fn parse_underscore_strong() {
        let tree = parse("This is __strong__ text.\n");
        let strong = find_node(&tree, SyntaxKind::STRONG).unwrap();
        assert!(strong.text().to_string().contains("strong"));
    }

    #[test]
    fn parse_mixed_emphasis() {
        let input = "Both *star* and _underscore_ work.\n";
        let tree = parse(input);
        let ems: Vec<_> = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::EMPHASIS)
            .collect();
        assert_eq!(ems.len(), 2, "Should have two EMPHASIS nodes");
        assert_eq!(tree.text().to_string(), input);
    }

    // === Phase 1: Strikethrough ===

    #[test]
    fn parse_strikethrough() {
        let tree = parse("This is ~~deleted~~ text.\n");
        let strike = find_node(&tree, SyntaxKind::STRIKETHROUGH).unwrap();
        assert!(strike.text().to_string().contains("deleted"));
    }

    #[test]
    fn strikethrough_preserves_text() {
        let input = "Text with ~~strikethrough~~ and *emphasis*.\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);

        let strike = find_node(&tree, SyntaxKind::STRIKETHROUGH);
        assert!(strike.is_some());
    }

    // === Phase 1: Images ===

    #[test]
    fn parse_image() {
        let tree = parse("See ![alt text](image.png) here.\n");
        let img = find_node(&tree, SyntaxKind::IMAGE).unwrap();
        let text = img.text().to_string();
        assert!(text.contains("alt text"));
        assert!(text.contains("image.png"));
    }

    #[test]
    fn image_preserves_text() {
        let input = "An ![image](url) and a [link](url).\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);

        let img = find_node(&tree, SyntaxKind::IMAGE);
        let link = find_node(&tree, SyntaxKind::LINK);
        assert!(img.is_some(), "Should have IMAGE node");
        assert!(link.is_some(), "Should have LINK node");
    }

    // === Phase 2: Goal references ===

    #[test]
    fn parse_block_ref() {
        let tree = parse("See ((abc-123-def)) for details.\n");
        let goal = find_node(&tree, SyntaxKind::BLOCK_REF).unwrap();
        assert!(goal.text().to_string().contains("abc-123-def"));
    }

    #[test]
    fn block_ref_preserves_text() {
        let input = "Link to ((goal-uuid)) here.\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    #[test]
    fn single_paren_not_block_ref() {
        // Single (text) should not become BLOCK_REF
        let tree = parse("Normal (parentheses) here.\n");
        let goal = find_node(&tree, SyntaxKind::BLOCK_REF);
        assert!(goal.is_none(), "Single parens should not be BLOCK_REF");
    }

    #[test]
    fn unclosed_block_ref() {
        // Unclosed (( should still produce a BLOCK_REF node (containing unclosed content)
        let tree = parse("See ((unclosed here.\n");
        let goal = find_node(&tree, SyntaxKind::BLOCK_REF);
        assert!(goal.is_some(), "Unclosed goal ref still produces node");
        assert_eq!(tree.text().to_string(), "See ((unclosed here.\n");
    }

    // === Phase 2: Autolinks ===

    #[test]
    fn parse_autolink() {
        let tree = parse("Visit <https://example.com> for info.\n");
        let autolink = find_node(&tree, SyntaxKind::AUTOLINK).unwrap();
        assert!(autolink.text().to_string().contains("example.com"));
    }

    #[test]
    fn autolink_preserves_text() {
        let input = "Link: <https://rust-lang.org> here.\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    #[test]
    fn lt_without_gt_not_autolink() {
        // < followed by text without > should not crash
        let tree = parse("Less than 5 < 10 works.\n");
        assert_eq!(tree.text().to_string(), "Less than 5 < 10 works.\n");
    }

    #[test]
    fn autolink_at_line_start() {
        // Autolink on its own line should be AUTOLINK in PARAGRAPH, not HTML_BLOCK
        let tree = parse("<https://example.com>\n");
        let autolink = find_node(&tree, SyntaxKind::AUTOLINK);
        assert!(autolink.is_some(), "Should parse as AUTOLINK");

        let html_block = find_node(&tree, SyntaxKind::HTML_BLOCK);
        assert!(html_block.is_none(), "Should NOT be HTML_BLOCK");
    }

    // === Phase 2: Properties ===

    #[test]
    fn parse_property() {
        let tree = parse("status:: DONE\n");
        let prop = find_node(&tree, SyntaxKind::PROPERTY).unwrap();
        let text = prop.text().to_string();
        assert!(text.contains("status"));
        assert!(text.contains("DONE"));
    }

    #[test]
    fn property_in_text() {
        let tree = parse("Task priority:: high here.\n");
        let prop = find_node(&tree, SyntaxKind::PROPERTY).unwrap();
        assert!(prop.text().to_string().contains("priority"));
    }

    #[test]
    fn single_colon_not_property() {
        let tree = parse("Time: 10:30 AM\n");
        let prop = find_node(&tree, SyntaxKind::PROPERTY);
        assert!(prop.is_none(), "Single colon should not be PROPERTY");
    }

    #[test]
    fn property_preserves_text() {
        let input = "- task with status:: DONE inline\n";
        let tree = parse(input);
        assert_eq!(tree.text().to_string(), input);
    }

    // === TDD: Unclosed inline constructs should not produce specialized nodes ===
    // These tests document the expected behavior: unclosed delimiters should NOT
    // produce EMPHASIS/STRONG/CODE_SPAN/STRIKETHROUGH nodes because downstream
    // processing (snapshot) computes inner content ranges by subtracting delimiter
    // lengths, which produces invalid ranges for unclosed constructs.

    #[test]
    fn unclosed_emphasis_not_emphasis_node() {
        // *word without closing * should NOT produce EMPHASIS
        // because snapshot computes content as (start+1)..(end-1) which assumes closing delimiter
        let tree = parse("*unclosed\n");
        let em = find_node(&tree, SyntaxKind::EMPHASIS);
        assert!(
            em.is_none(),
            "Unclosed *word should NOT produce EMPHASIS node"
        );
        // But text should be preserved
        assert_eq!(tree.text().to_string(), "*unclosed\n");
    }

    #[test]
    fn unclosed_strong_not_strong_node() {
        // **word without closing ** should NOT produce STRONG
        let tree = parse("**unclosed\n");
        let strong = find_node(&tree, SyntaxKind::STRONG);
        assert!(
            strong.is_none(),
            "Unclosed **word should NOT produce STRONG node"
        );
        assert_eq!(tree.text().to_string(), "**unclosed\n");
    }

    #[test]
    fn empty_emphasis_not_emphasis_node() {
        // ** alone (no content between delimiters) should NOT produce STRONG
        // Test truly empty: just ** at EOF
        let tree = parse("**\n");
        let strong = find_node(&tree, SyntaxKind::STRONG);
        assert!(strong.is_none(), "Empty ** should NOT produce STRONG node");
        assert_eq!(tree.text().to_string(), "**\n");
    }

    #[test]
    fn unclosed_code_span_not_code_node() {
        // `code without closing backtick should NOT produce CODE_SPAN
        let tree = parse("`unclosed\n");
        let code = find_node(&tree, SyntaxKind::CODE_SPAN);
        assert!(
            code.is_none(),
            "Unclosed `code should NOT produce CODE_SPAN node"
        );
        assert_eq!(tree.text().to_string(), "`unclosed\n");
    }

    #[test]
    fn unclosed_strikethrough_not_strike_node() {
        // ~~text without closing ~~ should NOT produce STRIKETHROUGH
        let tree = parse("~~unclosed\n");
        let strike = find_node(&tree, SyntaxKind::STRIKETHROUGH);
        assert!(
            strike.is_none(),
            "Unclosed ~~text should NOT produce STRIKETHROUGH node"
        );
        assert_eq!(tree.text().to_string(), "~~unclosed\n");
    }

    // === Nested inline support ===

    #[test]
    fn parse_nested_emphasis_in_strong() {
        // **bold with *nested* text**
        let tree = parse("**bold with *nested* text**\n");
        let strong = find_node(&tree, SyntaxKind::STRONG).unwrap();
        let em = find_node(&strong, SyntaxKind::EMPHASIS);
        assert!(em.is_some(), "STRONG should contain nested EMPHASIS");
    }

    #[test]
    fn parse_nested_strong_in_emphasis() {
        // *italic with **nested** text*
        let tree = parse("*italic with **nested** text*\n");
        let em = find_node(&tree, SyntaxKind::EMPHASIS).unwrap();
        let strong = find_node(&em, SyntaxKind::STRONG);
        assert!(strong.is_some(), "EMPHASIS should contain nested STRONG");
    }

    #[test]
    fn parse_adjacent_strong_emphasis() {
        // **bold***italic* - three stars in middle should close strong, start emphasis
        let tree = parse("**bold***italic*\n");
        assert_eq!(tree.text().to_string(), "**bold***italic*\n");
        let strong = find_node(&tree, SyntaxKind::STRONG);
        let em = find_node(&tree, SyntaxKind::EMPHASIS);
        assert!(strong.is_some(), "Should have STRONG");
        assert!(em.is_some(), "Should have EMPHASIS");
    }

    #[test]
    fn parse_triple_delimiter() {
        // ***text*** - should be strong containing emphasis
        let tree = parse("***text***\n");
        assert_eq!(tree.text().to_string(), "***text***\n");
        let strong = find_node(&tree, SyntaxKind::STRONG).expect("Should have STRONG");
        let em = find_node(&strong, SyntaxKind::EMPHASIS);
        assert!(em.is_some(), "STRONG should contain nested EMPHASIS");
    }
}
