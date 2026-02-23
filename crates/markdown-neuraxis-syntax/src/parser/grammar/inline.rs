//! Inline-level grammar rules.

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

/// Parse inline content until newline or EOF.
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
        SyntaxKind::STAR => emphasis_or_strong(p),
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
            SyntaxKind::STAR => emphasis_or_strong(p),
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

/// Parse emphasis *text* or strong **text**.
fn emphasis_or_strong(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Count opening stars
    let mut open_count = 0;
    while p.at(SyntaxKind::STAR) && open_count < 2 {
        p.bump();
        open_count += 1;
    }

    if open_count == 0 {
        m.abandon(p);
        return;
    }

    // Parse content until matching stars
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        if p.at(SyntaxKind::STAR) {
            // Count consecutive stars
            let mut close_count = 0;
            while p.nth(close_count) == SyntaxKind::STAR && close_count < open_count {
                close_count += 1;
            }

            if close_count >= open_count {
                // Matching close
                for _ in 0..open_count {
                    p.bump();
                }
                break;
            } else {
                // Not enough stars - consume and continue
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
}
