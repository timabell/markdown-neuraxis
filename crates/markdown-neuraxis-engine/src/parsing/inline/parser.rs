use crate::parsing::rope::span::Span;

use super::{
    cursor::Cursor,
    kinds::{CodeSpan, WikiLink},
    types::InlineNode,
};

/// Parses inline content into a sequence of [`InlineNode`]s.
///
/// # Arguments
/// - `base`: Byte offset in the rope where `s` begins (for absolute span positions)
/// - `s`: The string content to parse (typically a paragraph's content span)
///
/// # Raw Zone Precedence
/// Code spans are checked first and suppress all other parsing inside them.
/// `[[link]]` inside backticks is parsed as a code span, not a wikilink.
///
/// # Returns
/// A vector of inline nodes covering the entire input. Text between special
/// constructs is emitted as `InlineNode::Text`.
pub fn parse_inline(base: usize, s: &str) -> Vec<InlineNode> {
    let mut cur = Cursor::new(s, base);
    let mut out = vec![];
    let mut text_start = cur.pos();

    // Helper to flush accumulated text as a Text node
    fn flush_text(out: &mut Vec<InlineNode>, start: usize, end: usize) {
        if end > start {
            out.push(InlineNode::Text(Span { start, end }));
        }
    }

    while !cur.eof() {
        // Try constructs in precedence order (code spans first = raw zone)
        if let Some(node) = try_parse_code_span(&mut cur) {
            flush_text(&mut out, text_start, span_of(&node).start);
            text_start = span_of(&node).end;
            out.push(node);
            continue;
        }
        if let Some(node) = try_parse_wikilink(&mut cur) {
            flush_text(&mut out, text_start, span_of(&node).start);
            text_start = span_of(&node).end;
            out.push(node);
            continue;
        }
        cur.bump();
    }

    flush_text(&mut out, text_start, cur.pos());
    out
}

/// Extracts the full span from any inline node variant.
fn span_of(n: &InlineNode) -> Span {
    match n {
        InlineNode::Text(sp) => *sp,
        InlineNode::CodeSpan { full, .. } => *full,
        InlineNode::WikiLink { full, .. } => *full,
    }
}

/// Attempts to parse a code span starting at the current position.
///
/// Returns `None` if not at a backtick or if the code span isn't closed.
/// On failure, cursor position is restored.
fn try_parse_code_span(cur: &mut Cursor<'_>) -> Option<InlineNode> {
    if cur.peek() != Some(CodeSpan::TICK) {
        return None;
    }

    let saved = cur.clone();
    let start = cur.pos();
    cur.bump(); // `
    let inner_start = cur.pos();

    while !cur.eof() {
        if cur.peek() == Some(CodeSpan::TICK) {
            break;
        }
        cur.bump();
    }
    let inner_end = cur.pos();

    if cur.peek() != Some(CodeSpan::TICK) {
        // Not closed, restore cursor
        *cur = saved;
        return None;
    }
    cur.bump(); // closing `
    let end = cur.pos();

    Some(InlineNode::CodeSpan {
        full: Span { start, end },
        inner: Span {
            start: inner_start,
            end: inner_end,
        },
    })
}

/// Attempts to parse a wikilink starting at the current position.
///
/// Handles both `[[target]]` and `[[target|alias]]` forms.
/// Returns `None` if not at `[[` or if the wikilink isn't closed.
/// On failure, cursor position is restored.
fn try_parse_wikilink(cur: &mut Cursor<'_>) -> Option<InlineNode> {
    if !cur.starts_with(WikiLink::OPEN) {
        return None;
    }

    let saved = cur.clone();
    let start = cur.pos();
    cur.bump_n(WikiLink::OPEN.len());
    let target_start = cur.pos();

    while !cur.eof() {
        if cur.peek() == Some(WikiLink::ALIAS) {
            break;
        }
        if cur.starts_with(WikiLink::CLOSE) {
            break;
        }
        cur.bump();
    }
    let target_end = cur.pos();

    let mut alias = None;
    if cur.peek() == Some(WikiLink::ALIAS) {
        cur.bump(); // |
        let alias_start = cur.pos();
        while !cur.eof() {
            if cur.starts_with(WikiLink::CLOSE) {
                break;
            }
            cur.bump();
        }
        let alias_end = cur.pos();
        alias = Some(Span {
            start: alias_start,
            end: alias_end,
        });
    }

    if !cur.starts_with(WikiLink::CLOSE) {
        // Not closed, restore cursor
        *cur = saved;
        return None;
    }
    cur.bump_n(WikiLink::CLOSE.len());
    let end = cur.pos();

    Some(InlineNode::WikiLink {
        full: Span { start, end },
        target: Span {
            start: target_start,
            end: target_end,
        },
        alias,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_text() {
        let nodes = parse_inline(0, "hello world");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(
            nodes[0],
            InlineNode::Text(Span { start: 0, end: 11 })
        ));
    }

    #[test]
    fn parse_code_span() {
        let nodes = parse_inline(0, "`code`");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::CodeSpan { full, inner } => {
                assert_eq!(*full, Span { start: 0, end: 6 });
                assert_eq!(*inner, Span { start: 1, end: 5 });
            }
            _ => panic!("expected CodeSpan"),
        }
    }

    #[test]
    fn parse_wikilink_simple() {
        let nodes = parse_inline(0, "[[target]]");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::WikiLink {
                full,
                target,
                alias,
            } => {
                assert_eq!(*full, Span { start: 0, end: 10 });
                assert_eq!(*target, Span { start: 2, end: 8 });
                assert!(alias.is_none());
            }
            _ => panic!("expected WikiLink"),
        }
    }

    #[test]
    fn parse_wikilink_with_alias() {
        let nodes = parse_inline(0, "[[target|alias]]");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::WikiLink {
                full,
                target,
                alias,
            } => {
                assert_eq!(*full, Span { start: 0, end: 16 });
                assert_eq!(*target, Span { start: 2, end: 8 });
                assert_eq!(*alias, Some(Span { start: 9, end: 14 }));
            }
            _ => panic!("expected WikiLink"),
        }
    }

    #[test]
    fn code_span_suppresses_wikilink() {
        let nodes = parse_inline(0, "`[[not a link]]`");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0], InlineNode::CodeSpan { .. }));
    }

    #[test]
    fn unclosed_wikilink_becomes_text() {
        let nodes = parse_inline(0, "[[unclosed link");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(
            nodes[0],
            InlineNode::Text(Span { start: 0, end: 15 })
        ));
    }

    #[test]
    fn unclosed_code_span_becomes_text() {
        let nodes = parse_inline(0, "`unclosed code");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(
            nodes[0],
            InlineNode::Text(Span { start: 0, end: 14 })
        ));
    }
}
