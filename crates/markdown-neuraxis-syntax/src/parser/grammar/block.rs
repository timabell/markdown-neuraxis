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
//! ## Supported Block Types
//!
//! - ATX headings: `# heading`
//! - Setext headings: `Title\n====`
//! - Blockquotes: `> quote`
//! - Lists: `-`, `*`, `+`, `1.`
//! - Task checkboxes: `- [ ]`, `- [x]`
//! - Fenced code: `` ``` `` and `~~~`
//! - Indented code: 4+ spaces at line start
//! - Thematic breaks: `---`, `***`
//! - HTML blocks: `<div>...</div>`

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

use super::inline;

/// Calculate the visual width of whitespace (tabs count as 4 spaces).
fn whitespace_width(text: &str) -> usize {
    text.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

/// Check if token at offset would interrupt a paragraph (start a new block).
/// Used to determine when to end paragraph continuation.
fn interrupts_paragraph(p: &Parser<'_, '_>, offset: usize) -> bool {
    match p.nth(offset) {
        // Headings and blockquotes always interrupt
        SyntaxKind::HASH | SyntaxKind::GT => true,
        // List markers interrupt (dash/star/plus followed by space)
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            p.nth(offset + 1) == SyntaxKind::WHITESPACE
        }
        // Numbered list markers interrupt (digit(s) + dot + space)
        SyntaxKind::TEXT => {
            p.nth(offset + 1) == SyntaxKind::DOT && p.nth(offset + 2) == SyntaxKind::WHITESPACE
        }
        // Code fences interrupt
        SyntaxKind::BACKTICK | SyntaxKind::TILDE => is_code_fence_at(p, offset),
        _ => false,
    }
}

/// Parse a single block element.
///
/// This is the main dispatch function for block parsing. It skips blank lines,
/// then examines the first token to determine the block type.
pub fn block(p: &mut Parser<'_, '_>) {
    // Skip blank lines (empty or whitespace-only)
    loop {
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        } else if p.at(SyntaxKind::WHITESPACE) && p.nth(1) == SyntaxKind::NEWLINE {
            // Whitespace-only line - skip both tokens
            p.bump();
            p.bump();
        } else {
            break;
        }
    }

    if p.at_end() {
        return;
    }

    // Detect block type at line start
    match p.current() {
        SyntaxKind::HASH => heading(p),
        SyntaxKind::GT => blockquote(p),
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            // Could be frontmatter (--- at doc start with closing ---), thematic break, list item, or paragraph
            if p.at_document_start() && is_frontmatter_start(p) {
                frontmatter(p);
            } else if is_thematic_break(p) {
                thematic_break(p);
            } else if is_bullet_list_start(p) {
                list(p);
            } else {
                // Not a list (e.g., *emphasis* at start of line)
                paragraph(p);
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
                list(p);
            } else {
                paragraph(p);
            }
        }
        SyntaxKind::LT => {
            // Could be HTML block or autolink in paragraph
            if is_html_block_start(p) {
                html_block(p);
            } else {
                paragraph(p);
            }
        }
        SyntaxKind::WHITESPACE => {
            // Indented content - could be indented code, nested list item, or continuation
            if is_indented_code_block(p) {
                indented_code(p);
            } else if is_indented_list_item(p) {
                list_item_indented(p);
            } else {
                paragraph(p);
            }
        }
        _ => paragraph(p),
    }
}

/// Check if current position starts valid frontmatter (--- with closing ---)
fn is_frontmatter_start(p: &Parser<'_, '_>) -> bool {
    // Must be exactly 3 dashes followed by newline
    if p.current() != SyntaxKind::DASH {
        return false;
    }

    let mut count = 0;
    let mut i = 0;

    while p.nth(i) == SyntaxKind::DASH {
        count += 1;
        i += 1;
    }

    // Must be exactly 3 dashes, then newline (not EOF - need content)
    if count != 3 || p.nth(i) != SyntaxKind::NEWLINE {
        return false;
    }

    // Look ahead to find closing ---
    i += 1; // skip newline
    loop {
        // Skip to start of next line
        while p.nth(i) != SyntaxKind::EOF && p.nth(i) != SyntaxKind::NEWLINE {
            i += 1;
        }
        if p.nth(i) == SyntaxKind::EOF {
            return false; // No closing fence found
        }
        i += 1; // skip newline

        // Check if this line is ---
        if p.nth(i) == SyntaxKind::DASH {
            let mut dash_count = 0;
            let mut j = i;
            while p.nth(j) == SyntaxKind::DASH {
                dash_count += 1;
                j += 1;
            }
            if dash_count == 3 && (p.nth(j) == SyntaxKind::NEWLINE || p.nth(j) == SyntaxKind::EOF) {
                return true; // Found closing fence
            }
        }

        if p.nth(i) == SyntaxKind::EOF {
            return false;
        }
    }
}

/// Check if current position is a frontmatter fence (exactly ---)
fn is_frontmatter_fence(p: &Parser<'_, '_>) -> bool {
    if p.current() != SyntaxKind::DASH {
        return false;
    }

    let mut count = 0;
    let mut i = 0;

    while p.nth(i) == SyntaxKind::DASH {
        count += 1;
        i += 1;
    }

    count == 3 && (p.nth(i) == SyntaxKind::NEWLINE || p.nth(i) == SyntaxKind::EOF)
}

/// Parse YAML frontmatter block
fn frontmatter(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume opening ---
    p.bump(); // -
    p.bump(); // -
    p.bump(); // -
    p.eat(SyntaxKind::NEWLINE);

    // Consume content until closing --- or EOF
    loop {
        if p.at_end() {
            break;
        }

        // Check for closing fence at line start
        if p.at(SyntaxKind::DASH) && is_frontmatter_fence(p) {
            // Consume closing ---
            p.bump(); // -
            p.bump(); // -
            p.bump(); // -
            p.eat(SyntaxKind::NEWLINE);
            break;
        }

        // Consume the line
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        p.eat(SyntaxKind::NEWLINE);
    }

    m.complete(p, SyntaxKind::FRONTMATTER);
}

/// Check if current position starts an HTML block (<tag...)
fn is_html_block_start(p: &Parser<'_, '_>) -> bool {
    if p.current() != SyntaxKind::LT {
        return false;
    }

    // Look at what follows <
    let next = p.nth(1);

    // If followed by TEXT, check if it's a URL scheme (autolink) or HTML tag
    if next == SyntaxKind::TEXT {
        let text = p.nth_text(1);
        // Common URL schemes - these are autolinks, not HTML
        if text.starts_with("http")
            || text.starts_with("https")
            || text.starts_with("ftp")
            || text.starts_with("mailto")
        {
            return false;
        }
        // It's a tag name
        return true;
    }

    // <! for comments or doctype
    if next == SyntaxKind::EXCLAIM {
        return true;
    }

    false
}

/// Parse an HTML block
fn html_block(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume until blank line or EOF
    loop {
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }

        if !p.eat(SyntaxKind::NEWLINE) {
            break;
        }

        // Blank line ends HTML block
        if p.at(SyntaxKind::NEWLINE) || p.at_end() {
            break;
        }
    }

    m.complete(p, SyntaxKind::HTML_BLOCK);
}

/// Check if current position is a setext heading underline (=== or ---)
fn is_setext_underline(p: &Parser<'_, '_>) -> bool {
    let marker = p.current();
    if !matches!(marker, SyntaxKind::EQUALS | SyntaxKind::DASH) {
        return false;
    }

    // Need at least 3 markers for a setext underline
    let mut count = 0;
    let mut i = 0;

    while p.nth(i) != SyntaxKind::EOF && p.nth(i) != SyntaxKind::NEWLINE {
        match p.nth(i) {
            k if k == marker => count += 1,
            SyntaxKind::WHITESPACE => {} // trailing whitespace OK
            _ => return false,           // any other character invalidates
        }
        i += 1;
    }

    count >= 3
}

/// Check if current position starts a bullet list item (marker + whitespace)
fn is_bullet_list_start(p: &Parser<'_, '_>) -> bool {
    matches!(
        p.current(),
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS
    ) && p.nth(1) == SyntaxKind::WHITESPACE
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
    is_code_fence_at(p, 0)
}

/// Check if position at offset starts a code fence.
fn is_code_fence_at(p: &Parser<'_, '_>, offset: usize) -> bool {
    let marker = p.nth(offset);
    if !matches!(marker, SyntaxKind::BACKTICK | SyntaxKind::TILDE) {
        return false;
    }

    // Count consecutive markers
    let mut count = 0;
    let mut i = offset;

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

/// Parse a list (consecutive list items wrapped in LIST node).
/// `sibling_indent_len` is the whitespace length expected for sibling items (0 for root level).
fn list_ext(p: &mut Parser<'_, '_>, sibling_indent_len: usize) {
    let m = p.start();

    // Parse the first list item and track whether it's ordered
    let is_ordered = is_numbered_list_item(p);
    if is_ordered {
        list_item_numbered(p, sibling_indent_len);
    } else {
        list_item(p, sibling_indent_len);
    }

    // Continue parsing list items at the same level
    loop {
        // Skip blank lines (empty or whitespace-only) within the list
        let mut blank_count = 0;
        loop {
            if p.at(SyntaxKind::NEWLINE) {
                blank_count += 1;
                p.bump();
            } else if p.at(SyntaxKind::WHITESPACE) && p.nth(1) == SyntaxKind::NEWLINE {
                // Whitespace-only line counts as blank
                blank_count += 1;
                p.bump();
                p.bump();
            } else {
                break;
            }
        }

        if p.at_end() {
            break;
        }

        // For nested lists, siblings must be indented at the correct level
        if sibling_indent_len > 0 {
            if !p.at(SyntaxKind::WHITESPACE) {
                break; // Outdented - not part of this nested list
            }
            // Check that whitespace width matches expected sibling indent
            let ws_width = whitespace_width(p.current_text());
            if ws_width != sibling_indent_len {
                break; // Different indent level - not a sibling in this list
            }
            // Check what follows BEFORE consuming whitespace
            let after_ws = p.nth(1);
            let is_list_item = match after_ws {
                SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
                    p.nth(2) == SyntaxKind::WHITESPACE
                }
                SyntaxKind::TEXT => {
                    p.nth(2) == SyntaxKind::DOT && p.nth(3) == SyntaxKind::WHITESPACE
                }
                _ => false,
            };
            if !is_list_item || blank_count > 0 {
                break; // Not a sibling list item
            }
            // Now consume indentation and parse item
            p.bump();
            match p.current() {
                SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
                    list_item(p, sibling_indent_len);
                }
                SyntaxKind::TEXT => {
                    list_item_numbered(p, sibling_indent_len);
                }
                _ => unreachable!(), // already checked above
            }
        } else {
            // Root level list - items start without indentation
            match p.current() {
                SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
                    if is_thematic_break(p) {
                        break; // Not a list item
                    }
                    if blank_count > 0 {
                        break;
                    }
                    list_item(p, 0);
                }
                SyntaxKind::TEXT if is_numbered_list_item(p) => {
                    if blank_count > 0 {
                        break;
                    }
                    list_item_numbered(p, 0);
                }
                _ => break, // Not a list item, end the list
            }
        }
    }

    let kind = if is_ordered {
        SyntaxKind::ORDERED_LIST
    } else {
        SyntaxKind::UNORDERED_LIST
    };
    m.complete(p, kind);
}

/// Parse a root-level list.
fn list(p: &mut Parser<'_, '_>) {
    list_ext(p, 0);
}

/// Parse a nested list (inside a list item) at the given sibling indentation level.
fn nested_list(p: &mut Parser<'_, '_>, sibling_indent_len: usize) {
    list_ext(p, sibling_indent_len);
}

/// Parse a list item.
///
/// A list item contains:
/// - A marker (-, *, +) followed by space
/// - Blocks at the item's content indent level (paragraphs, blockquotes, nested lists, code)
///
/// `sibling_indent_len` is the whitespace length for sibling items at the same level.
fn list_item(p: &mut Parser<'_, '_>, sibling_indent_len: usize) {
    let m = p.start();

    // Consume the marker (-, *, +)
    p.bump();

    // Consume required space after marker
    if !p.eat(SyntaxKind::WHITESPACE) {
        // Not a valid list item, treat as paragraph
        m.abandon(p);
        return paragraph(p);
    }

    // Content must be indented by marker width (2 for "- ")
    let content_indent = sibling_indent_len + 2;

    // Parse blocks within this list item
    blocks_in_list_item(p, content_indent, sibling_indent_len);

    m.complete(p, SyntaxKind::LIST_ITEM);
}

/// Parse blocks within a list item context.
///
/// This is the unified block parsing for list items. It loops, dispatching to
/// the appropriate block parser (paragraph, blockquote, nested list, code block).
/// Paragraph is the fallback - any content that isn't a recognized block marker
/// becomes a paragraph.
///
/// `content_indent` is the minimum indent for content to belong to this list item.
/// `sibling_indent` is the indent where sibling list items would appear.
fn blocks_in_list_item(p: &mut Parser<'_, '_>, content_indent: usize, sibling_indent: usize) {
    // Check for checkbox at start: [ ] or [x] or [X]
    if is_checkbox(p) {
        checkbox(p);
        p.eat(SyntaxKind::WHITESPACE);
    }

    // First block: we're right after "- ", parse immediately (no indent check)
    dispatch_block_in_list_item(p, content_indent, sibling_indent);

    // Subsequent blocks: check indent, then parse
    loop {
        // Check for blank lines - but don't consume them yet!
        // We need to see what comes after to decide if content continues.
        let mut lookahead = 0;
        while p.nth(lookahead) == SyntaxKind::NEWLINE {
            lookahead += 1;
        }

        // If blank lines lead to EOF or outdented content, stop here
        // (let list_ext handle blank line detection for list termination)
        if lookahead > 0 {
            let after_blanks = p.nth(lookahead);
            if after_blanks == SyntaxKind::EOF {
                break;
            }
            // If next content is at column 0, let caller handle it
            if after_blanks != SyntaxKind::WHITESPACE {
                break;
            }
        }

        // Now consume the blank lines we peeked at
        for _ in 0..lookahead {
            p.bump();
        }

        if p.at_end() {
            break;
        }

        // Must have indentation
        if !p.at(SyntaxKind::WHITESPACE) {
            break; // Outdented to column 0 - end of item
        }

        let ws_width = whitespace_width(p.current_text());

        // If below sibling indent, we're outdented - end of item
        if ws_width < sibling_indent {
            break;
        }

        // Check what follows the whitespace
        let after_ws = p.nth(1);

        // Blank line (whitespace-only) - skip it
        if after_ws == SyntaxKind::NEWLINE || after_ws == SyntaxKind::EOF {
            p.bump(); // consume whitespace
            continue;
        }

        // If at exactly sibling indent, content belongs to parent level
        // (for nested lists where sibling_indent > 0)
        if ws_width == sibling_indent && sibling_indent > 0 {
            break;
        }

        // Content belongs to this item - consume indent and dispatch
        p.bump();
        dispatch_block_in_list_item(p, content_indent, sibling_indent);
    }
}

/// Dispatch to the appropriate block parser based on current token.
/// Paragraph is the fallback for any unrecognized content.
fn dispatch_block_in_list_item(
    p: &mut Parser<'_, '_>,
    content_indent: usize,
    sibling_indent: usize,
) {
    match p.current() {
        SyntaxKind::HASH => heading(p),
        SyntaxKind::GT => blockquote(p),
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            if p.nth(1) == SyntaxKind::WHITESPACE {
                // Nested list
                nested_list(p, content_indent);
            } else {
                // Not a list marker (e.g., emphasis), treat as paragraph
                paragraph_in_list_item(p, content_indent, sibling_indent);
            }
        }
        SyntaxKind::TEXT => {
            if is_numbered_list_item(p) {
                nested_list(p, content_indent);
            } else {
                paragraph_in_list_item(p, content_indent, sibling_indent);
            }
        }
        SyntaxKind::BACKTICK | SyntaxKind::TILDE => {
            if is_code_fence(p) {
                fenced_code(p);
            } else {
                paragraph_in_list_item(p, content_indent, sibling_indent);
            }
        }
        _ => paragraph_in_list_item(p, content_indent, sibling_indent),
    }
}

/// Parse a paragraph within a list item, with indent-aware continuation.
///
/// Parses inline content, then loops for continuation lines that are:
/// - Not outdented below content_indent
/// - Not block markers (which end the paragraph)
///
/// `content_indent` is the expected content column for this list item.
/// `_sibling_indent` is unused but kept for API consistency.
fn paragraph_in_list_item(p: &mut Parser<'_, '_>, content_indent: usize, _sibling_indent: usize) {
    let para = p.start();

    // Parse first line
    inline::inline_until_newline(p);

    if !p.eat(SyntaxKind::NEWLINE) {
        para.complete(p, SyntaxKind::PARAGRAPH);
        return;
    }

    // Check for continuation lines
    loop {
        if p.at_end() {
            break;
        }

        // Blank line ends paragraph
        if p.at(SyntaxKind::NEWLINE) {
            break;
        }

        // Must have some indentation
        if !p.at(SyntaxKind::WHITESPACE) {
            break;
        }

        let ws_width = whitespace_width(p.current_text());

        // Below content indent = outdented, end paragraph
        if ws_width < content_indent {
            break;
        }

        // Blank line (whitespace-only)
        if p.nth(1) == SyntaxKind::NEWLINE || p.nth(1) == SyntaxKind::EOF {
            break;
        }

        // Block markers interrupt the paragraph
        if interrupts_paragraph(p, 1) {
            break;
        }

        // Continuation line - consume indentation and content
        p.bump();
        inline::inline_until_newline(p);
        if !p.eat(SyntaxKind::NEWLINE) {
            break;
        }
    }

    para.complete(p, SyntaxKind::PARAGRAPH);
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

/// Check if current position is a checkbox: [ ] or [x] or [X]
fn is_checkbox(p: &Parser<'_, '_>) -> bool {
    if p.current() != SyntaxKind::LBRACKET {
        return false;
    }

    // Check for [ ] or [x] or [X]
    let inner = p.nth(1);
    let close = p.nth(2);

    if close != SyntaxKind::RBRACKET {
        return false;
    }

    match inner {
        SyntaxKind::WHITESPACE => true, // [ ]
        SyntaxKind::TEXT => {
            // [x] or [X]
            let text = p.nth_text(1);
            text == "x" || text == "X"
        }
        _ => false,
    }
}

/// Parse a checkbox [ ] or [x]
fn checkbox(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Consume [
    p.bump();
    // Consume space or x/X
    p.bump();
    // Consume ]
    p.bump();

    m.complete(p, SyntaxKind::CHECKBOX);
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
/// `sibling_indent_len` is the whitespace length for sibling items at the same level.
fn list_item_numbered(p: &mut Parser<'_, '_>, sibling_indent_len: usize) {
    let m = p.start();

    // Consume the number
    let number_len = p.current_text().len();
    p.bump();

    // Consume the dot
    p.bump();

    // Consume the required space
    if !p.eat(SyntaxKind::WHITESPACE) {
        m.abandon(p);
        return paragraph(p);
    }

    // Content indent = sibling indent + number length + ". " (dot + space)
    let content_indent = sibling_indent_len + number_len + 2;

    // Parse blocks within this list item
    blocks_in_list_item(p, content_indent, sibling_indent_len);

    m.complete(p, SyntaxKind::LIST_ITEM);
}

/// Check if current position starts an indented code block (4+ spaces not followed by list marker)
fn is_indented_code_block(p: &Parser<'_, '_>) -> bool {
    if p.current() != SyntaxKind::WHITESPACE {
        return false;
    }

    // Need at least 4 spaces/tab
    let ws_width = whitespace_width(p.current_text());

    if ws_width < 4 {
        return false;
    }

    // Must NOT be followed by a list marker
    let after_ws = p.nth(1);
    match after_ws {
        SyntaxKind::DASH | SyntaxKind::STAR | SyntaxKind::PLUS => {
            // List marker - not indented code
            false
        }
        SyntaxKind::TEXT => {
            // Could be numbered list like "1."
            !(p.nth(2) == SyntaxKind::DOT && p.nth(3) == SyntaxKind::WHITESPACE)
        }
        _ => true,
    }
}

/// Parse an indented code block (4+ spaces at line start)
fn indented_code(p: &mut Parser<'_, '_>) {
    let m = p.start();

    // Parse consecutive indented lines
    loop {
        // Consume the whitespace and line content
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }

        // Consume newline
        if !p.eat(SyntaxKind::NEWLINE) {
            break;
        }

        // Check if next line continues the code block (4+ spaces)
        if p.at(SyntaxKind::WHITESPACE) {
            let ws_width = whitespace_width(p.current_text());
            if ws_width >= 4 {
                continue; // Continue code block
            }
        }

        // Not a continuation - end code block
        break;
    }

    m.complete(p, SyntaxKind::INDENTED_CODE);
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

/// Parse an indented list item (nested list) - wraps in LIST for consistency
fn list_item_indented(p: &mut Parser<'_, '_>) {
    // Consume leading whitespace (indentation)
    while p.at(SyntaxKind::WHITESPACE) {
        p.bump();
    }

    // Now we should be at a list marker - delegate to list()
    list(p);
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

        // Check for closing fence (may have leading whitespace for indented code)
        let fence_offset = if p.at(SyntaxKind::WHITESPACE) { 1 } else { 0 };

        if p.nth(fence_offset) == fence_marker {
            let mut close_len = 0;

            // Peek ahead to count fence markers
            while p.nth(fence_offset + close_len) == fence_marker {
                close_len += 1;
            }

            if close_len >= fence_len {
                // This is the closing fence - consume indentation if present
                if fence_offset > 0 {
                    p.bump();
                }
                // Consume the fence markers
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

/// Parse a paragraph (default block), possibly converting to setext heading.
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

        // Check for setext heading underline (=== or ---)
        if is_setext_underline(p) {
            // Consume the underline
            while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
                p.bump();
            }
            p.eat(SyntaxKind::NEWLINE);
            m.complete(p, SyntaxKind::SETEXT_HEADING);
            return;
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
            SyntaxKind::WHITESPACE => {
                // Check for indented list item
                if is_indented_list_item(p) {
                    break;
                }
            }
            _ => {}
        }

        // Continue paragraph - but we already consumed the newline, so just loop
    }

    m.complete(p, SyntaxKind::PARAGRAPH);
}

// All parsing behavior is verified by snapshot tests in tests/snapshots/.
// Edge cases are in tests/snapshots/malformed/ and tests/snapshots/combinations/.
