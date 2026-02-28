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
                break;
            } else {
                // Not matching - consume one backtick and continue
                p.bump();
            }
        } else {
            p.bump();
        }
    }

    m.complete(p, SyntaxKind::CODE_SPAN);
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

    // Parse content until matching delimiters
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(delimiter) {
            // Count consecutive delimiters
            let mut close_count = 0;
            while p.nth(close_count) == delimiter && close_count < open_count {
                close_count += 1;
            }

            if close_count >= open_count {
                // Matching close
                for _ in 0..open_count {
                    p.bump();
                }
                break;
            } else {
                // Not enough delimiters - consume and continue
                p.bump();
            }
        } else {
            p.bump();
        }
    }

    let kind = if open_count >= 2 {
        SyntaxKind::STRONG
    } else {
        SyntaxKind::EMPHASIS
    };

    m.complete(p, kind);
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
        // Not enough tildes for strikethrough - just plain text
        m.complete(p, SyntaxKind::INLINE);
        return;
    }

    // Parse content until matching ~~
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::TILDE) && p.nth(1) == SyntaxKind::TILDE {
            // Found closing ~~
            p.bump(); // ~
            p.bump(); // ~
            break;
        }
        p.bump();
    }

    m.complete(p, SyntaxKind::STRIKETHROUGH);
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
}
