//! # Parser - Event-Based Tree Construction
//!
//! This module implements the core parsing logic, transforming a token stream
//! into a syntax tree using the **event-based** architecture from rust-analyzer.
//!
//! ## Why Event-Based Parsing?
//!
//! Traditional recursive descent parsers build the tree directly during parsing.
//! This has problems:
//!
//! 1. **Deep nesting can overflow the stack** (Markdown can nest arbitrarily)
//! 2. **Backtracking is expensive** when you've already built tree nodes
//! 3. **Error recovery is tricky** when partially-built nodes exist
//!
//! Instead, we emit a flat list of **events** ([`Event`]) that describe the
//! tree structure. The [`Sink`] then builds the actual Rowan tree from events.
//!
//! ## The Event Model
//!
//! Parsing produces events like:
//! ```text
//! Start(HEADING)
//! Token(HASH)
//! Token(WHITESPACE)
//! Token(TEXT)
//! Token(NEWLINE)
//! Finish
//! ```
//!
//! The Sink processes these in order, calling `start_node()` for Start,
//! `token()` for Token, and `finish_node()` for Finish.
//!
//! ## The Marker System
//!
//! The key innovation is the [`Marker`] type, which makes tree construction
//! **type-safe at compile time**. When you call `parser.start()`, you get a
//! `Marker`. This marker **must** be either:
//!
//! - Completed with `marker.complete(parser, KIND)` → emits Start+Finish
//! - Abandoned with `marker.abandon(parser)` → removes the placeholder
//!
//! If you drop a marker without doing either, **the program panics**. This
//! prevents accidentally leaving the tree in an inconsistent state.
//!
//! ```ignore
//! let m = parser.start();           // Get a marker
//! parser.bump();                    // Consume some tokens
//! m.complete(parser, SyntaxKind::PARAGRAPH);  // MUST complete or abandon
//! ```
//!
//! ## Forward Parent Links
//!
//! Sometimes we need to wrap an already-parsed node in a new parent (for
//! left-recursion or binary expressions). The `CompletedMarker::precede()`
//! method handles this by creating a **forward parent link** that the Sink
//! resolves when building the tree.
//!
//! ## Module Structure
//!
//! - [`event`] - The Event enum
//! - [`sink`] - Converts events to Rowan tree
//! - [`grammar`] - Grammar rules (root, block, inline)
//!
//! ## Public API
//!
//! The main entry point is [`parse`]:
//!
//! ```
//! use markdown_neuraxis_syntax::parse;
//!
//! let tree = parse("# Hello\n");
//! println!("{:#?}", tree);
//! ```

pub mod event;
pub mod sink;

mod grammar;

use crate::lexer::{Token, lex};
use crate::syntax_kind::{SyntaxKind, SyntaxNode};
use event::Event;
use sink::Sink;

/// The parser state machine.
///
/// Holds the token stream, current position, and accumulated events.
/// Grammar functions receive `&mut Parser` and use its methods to:
///
/// - Inspect tokens: `current()`, `nth()`, `at()`, `at_end()`
/// - Consume tokens: `bump()`, `eat()`
/// - Build structure: `start()` → `Marker` → `complete()`/`abandon()`
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
/// This is the heart of the type-safe tree building system. When you call
/// `parser.start()`, a `Placeholder` event is pushed and you get a `Marker`
/// pointing to it.
///
/// ## The Must-Use Contract
///
/// The `#[must_use]` attribute and the `Drop` impl together enforce that
/// every marker is either:
///
/// - **Completed** via `marker.complete(parser, KIND)` - converts the
///   placeholder to a `Start` event and pushes a `Finish` event
/// - **Abandoned** via `marker.abandon(parser)` - removes the placeholder
///   (only works if nothing was pushed after it)
///
/// If you drop a marker without doing either, **the program panics**. This
/// catches bugs at runtime rather than producing corrupt trees.
///
/// ## Example
///
/// ```ignore
/// fn paragraph(p: &mut Parser) {
///     let m = p.start();  // Reserve a spot for the node
///
///     // Parse content...
///     while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
///         p.bump();
///     }
///
///     m.complete(p, SyntaxKind::PARAGRAPH);  // Finalize the node
/// }
/// ```
#[must_use = "Markers must be completed or abandoned, dropping them is a bug"]
pub struct Marker {
    /// Position in the events vector where our Placeholder lives
    pos: usize,
    /// Tracks whether complete() or abandon() was called
    completed: bool,
}

impl Marker {
    /// Complete this marker, creating a node of the given kind.
    ///
    /// This:
    /// 1. Replaces the `Placeholder` at our position with `Start { kind, ... }`
    /// 2. Pushes a `Finish` event
    /// 3. Returns a `CompletedMarker` for potential `precede()` calls
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

    /// Abandon this marker without creating a node.
    ///
    /// Use this when you speculatively started a node but decided not to
    /// create it (e.g., the input didn't match what you expected).
    ///
    /// **Note**: This only removes the placeholder if it's the last event.
    /// If other events were pushed after `start()`, the placeholder becomes
    /// inert and is ignored by the Sink.
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

/// A marker for a node that has been completed.
///
/// The only thing you can do with a `CompletedMarker` is call `precede()`
/// to wrap the completed node in a new parent. This is useful for handling
/// left-recursion or wrapping expressions after the fact.
///
/// ## The Precede Pattern
///
/// Sometimes you parse something and only later realize it needs a wrapper:
///
/// ```ignore
/// // We parsed "a" as an expression
/// let expr = parse_atom(p);  // Returns CompletedMarker
///
/// // Oh, there's a "+" - this is actually a binary expression!
/// if p.at(SyntaxKind::PLUS) {
///     let bin_expr = expr.precede(p);  // Start a new node BEFORE "a"
///     p.bump();  // consume "+"
///     parse_atom(p);  // parse "b"
///     bin_expr.complete(p, SyntaxKind::BIN_EXPR);
/// }
/// // Result: BIN_EXPR containing [atom "a", "+", atom "b"]
/// ```
///
/// This works by setting a `forward_parent` link that the Sink resolves.
#[derive(Debug, Clone, Copy)]
pub struct CompletedMarker {
    /// Position of the Start event for this completed node
    pos: usize,
}

impl CompletedMarker {
    /// Create a new parent node that will contain this node.
    ///
    /// Returns a new `Marker` that, when completed, will become the parent
    /// of the node at `self.pos`.
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
