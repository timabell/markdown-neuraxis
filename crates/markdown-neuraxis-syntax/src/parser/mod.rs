//! Event-based parser for Markdown following the rust-analyzer model.

pub mod event;
pub mod sink;

mod grammar;

use crate::lexer::{Token, lex};
use crate::syntax_kind::{SyntaxKind, SyntaxNode};
use event::Event;
use sink::Sink;

/// Parser for Markdown with event-based tree construction.
pub struct Parser<'t, 'input> {
    tokens: &'t [Token<'input>],
    pos: usize,
    events: Vec<Event>,
}

impl<'t, 'input> Parser<'t, 'input> {
    /// Create a new parser from a slice of tokens.
    pub fn new(tokens: &'t [Token<'input>]) -> Self {
        Self {
            tokens,
            pos: 0,
            events: Vec::new(),
        }
    }

    /// Parse the tokens and return a syntax tree.
    pub fn parse(mut self) -> SyntaxNode {
        grammar::root(&mut self);
        let sink = Sink::new(self.tokens, self.events);
        sink.finish()
    }

    /// Start a new node and return a marker.
    pub fn start(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Placeholder);
        Marker {
            pos,
            completed: false,
        }
    }

    /// Current token kind, or EOF if past end.
    pub fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Look ahead n tokens.
    pub fn nth(&self, n: usize) -> SyntaxKind {
        self.tokens
            .get(self.pos + n)
            .map(|t| t.kind)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// Check if at end of input.
    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Check if current token is of given kind.
    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Consume the current token if it matches.
    pub fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current token unconditionally.
    pub fn bump(&mut self) {
        if !self.at_end() {
            let kind = self.current();
            self.events.push(Event::token(kind));
            self.pos += 1;
        }
    }

    /// Consume n tokens as a single composite token.
    pub fn bump_n(&mut self, n: usize, kind: SyntaxKind) {
        if self.pos + n <= self.tokens.len() {
            self.events.push(Event::Token {
                kind,
                n_raw_tokens: n as u8,
            });
            self.pos += n;
        }
    }

    /// Get the text of the current token.
    pub fn current_text(&self) -> &'input str {
        self.tokens.get(self.pos).map(|t| t.text).unwrap_or("")
    }

    /// Check if we're at the start of a line (after newline or at start).
    pub fn at_line_start(&self) -> bool {
        if self.pos == 0 {
            return true;
        }
        // Check if previous token was a newline
        self.tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.kind == SyntaxKind::NEWLINE)
            .unwrap_or(false)
    }

    /// Remaining tokens count.
    pub fn remaining(&self) -> usize {
        self.tokens.len().saturating_sub(self.pos)
    }
}

/// A marker for a node being constructed.
///
/// Must be either completed or abandoned before being dropped.
#[must_use]
pub struct Marker {
    pos: usize,
    completed: bool,
}

impl Marker {
    /// Complete the node with the given kind.
    pub fn complete(mut self, p: &mut Parser<'_, '_>, kind: SyntaxKind) -> CompletedMarker {
        self.completed = true;
        let event_at_pos = &mut p.events[self.pos];
        assert!(matches!(event_at_pos, Event::Placeholder));
        *event_at_pos = Event::Start {
            kind,
            forward_parent: None,
        };
        p.events.push(Event::Finish);
        CompletedMarker { pos: self.pos }
    }

    /// Abandon the marker without creating a node.
    pub fn abandon(mut self, p: &mut Parser<'_, '_>) {
        self.completed = true;
        if self.pos == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Placeholder) => {}
                _ => unreachable!(),
            }
        }
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!("Marker must be either completed or abandoned");
        }
    }
}

/// A marker for a completed node.
#[derive(Debug, Clone, Copy)]
pub struct CompletedMarker {
    pos: usize,
}

impl CompletedMarker {
    /// Wrap this node in a new parent node (precede pattern).
    pub fn precede(self, p: &mut Parser<'_, '_>) -> Marker {
        let new_pos = p.events.len();
        p.events.push(Event::Placeholder);

        // Update the original Start event to point to this new parent
        if let Event::Start { forward_parent, .. } = &mut p.events[self.pos] {
            *forward_parent = Some(new_pos);
        }

        Marker {
            pos: new_pos,
            completed: false,
        }
    }
}

/// Parse markdown source into a syntax tree.
pub fn parse(source: &str) -> SyntaxNode {
    let tokens = lex(source);
    let parser = Parser::new(&tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_empty_input() {
        let tree = parse("");
        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert_eq!(tree.children().count(), 0);
    }

    #[test]
    fn parse_preserves_all_text() {
        let input = "Hello, world!";
        let tree = parse(input);
        assert_eq!(tree.text(), input);
    }

    #[test]
    fn parse_simple_paragraph() {
        let input = "Hello";
        let tree = parse(input);

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        let para = tree.children().next().unwrap();
        assert_eq!(para.kind(), SyntaxKind::PARAGRAPH);
    }

    #[test]
    fn marker_must_be_completed() {
        let result = std::panic::catch_unwind(|| {
            let tokens = lex("test");
            let mut parser = Parser::new(&tokens);
            let _marker = parser.start();
            // Marker dropped without completion - should panic
        });
        assert!(result.is_err());
    }

    #[test]
    fn marker_can_be_abandoned() {
        let tokens = lex("test");
        let mut parser = Parser::new(&tokens);
        let marker = parser.start();
        marker.abandon(&mut parser);
        // Should not panic
    }
}
