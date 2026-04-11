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

/// Parse inline content until pipe, newline, or EOF.
///
/// Used for table cells where pipes delimit cell boundaries.
pub fn inline_until_pipe_or_newline(p: &mut Parser<'_, '_>) {
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::PIPE) {
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
            // Parse other inline elements (wikilinks, code, links, etc.)
            inline_element(p);
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

// All parsing behavior is verified by snapshot tests in tests/snapshots/.
// Edge cases are in tests/snapshots/malformed/ and tests/snapshots/combinations/.
