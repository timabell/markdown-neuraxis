//! # Sink - Building the Rowan Tree from Events
//!
//! The Sink is the final stage of parsing. It consumes the flat event stream
//! and builds the actual Rowan syntax tree using `GreenNodeBuilder`.
//!
//! ## How It Works
//!
//! The Sink processes events in order:
//!
//! 1. **Start** → Call `builder.start_node(kind)`
//! 2. **Token** → Call `builder.token(kind, text)` (text comes from the token stream)
//! 3. **Finish** → Call `builder.finish_node()`
//! 4. **Placeholder** → Skip (these are abandoned markers)
//!
//! ## Forward Parent Resolution
//!
//! The tricky part is handling `forward_parent` links. When a `Start` event
//! has a `forward_parent`, it means "that other node should be my parent."
//!
//! We resolve this by:
//! 1. Following the chain of forward_parent links
//! 2. Collecting all the node kinds in order
//! 3. Starting them in **reverse** order (outermost first)
//!
//! For example, if we have:
//! ```text
//! Start(A, forward_parent: 2)  // index 0
//! Token(x)                      // index 1
//! Start(B, forward_parent: None) // index 2
//! Finish                        // for B
//! Finish                        // for A
//! ```
//!
//! The chain is: A → B (index 2). We collect [A, B], reverse to [B, A],
//! and start nodes in that order. Result: B contains A contains token x.
//!
//! ## Token Grouping
//!
//! The `n_raw_tokens` field in Token events allows grouping multiple lexer
//! tokens into one tree token. The Sink concatenates the text from
//! `n_raw_tokens` consecutive tokens.

use rowan::GreenNodeBuilder;

use crate::lexer::Token;
use crate::parser::event::Event;
use crate::syntax_kind::{SyntaxKind, SyntaxNode};

/// Converts parser events and tokens into a Rowan syntax tree.
///
/// ## Usage
///
/// ```ignore
/// let sink = Sink::new(&tokens, events);
/// let tree = sink.finish();
/// ```
///
/// The `finish()` method consumes the sink and returns the root `SyntaxNode`.
pub struct Sink<'t, 'input> {
    builder: GreenNodeBuilder<'static>,
    tokens: &'t [Token<'input>],
    cursor: usize,
    events: Vec<Event>,
}

impl<'t, 'input> Sink<'t, 'input> {
    /// Create a new sink.
    pub fn new(tokens: &'t [Token<'input>], events: Vec<Event>) -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
            tokens,
            cursor: 0,
            events,
        }
    }

    /// Consume the sink and build the syntax tree.
    pub fn finish(mut self) -> SyntaxNode {
        // Process forward_parent links to create proper tree structure
        let mut forward_parents = Vec::new();

        for i in 0..self.events.len() {
            match std::mem::replace(&mut self.events[i], Event::Placeholder) {
                Event::Start {
                    kind,
                    forward_parent,
                } => {
                    // Collect forward parent chain
                    forward_parents.push(kind);
                    let mut fp = forward_parent;

                    while let Some(parent_idx) = fp {
                        match std::mem::replace(&mut self.events[parent_idx], Event::Placeholder) {
                            Event::Start {
                                kind,
                                forward_parent,
                            } => {
                                fp = forward_parent;
                                forward_parents.push(kind);
                            }
                            _ => unreachable!(),
                        }
                    }

                    // Start nodes in reverse order (outermost first)
                    for kind in forward_parents.drain(..).rev() {
                        self.builder.start_node(kind.into());
                    }
                }
                Event::Token { kind, n_raw_tokens } => {
                    self.token(kind, n_raw_tokens as usize);
                }
                Event::Finish => {
                    self.builder.finish_node();
                }
                Event::Placeholder => {}
            }
        }

        SyntaxNode::new_root(self.builder.finish())
    }

    fn token(&mut self, kind: SyntaxKind, n_raw_tokens: usize) {
        // Accumulate text from n_raw_tokens
        let start = self.cursor;
        self.cursor += n_raw_tokens;
        let text: String = self.tokens[start..self.cursor]
            .iter()
            .map(|t| t.text)
            .collect();
        self.builder.token(kind.into(), &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn token(kind: SyntaxKind, text: &str) -> Token<'_> {
        Token { kind, text }
    }

    #[test]
    fn sink_builds_simple_tree() {
        let tokens = vec![token(SyntaxKind::TEXT, "hello")];

        let events = vec![
            Event::start(SyntaxKind::ROOT),
            Event::start(SyntaxKind::PARAGRAPH),
            Event::token(SyntaxKind::TEXT),
            Event::Finish,
            Event::Finish,
        ];

        let sink = Sink::new(&tokens, events);
        let tree = sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert_eq!(tree.children().count(), 1);
    }

    #[test]
    fn sink_preserves_text() {
        let input = "hello";
        let tokens = lex(input);

        let events = vec![
            Event::start(SyntaxKind::ROOT),
            Event::start(SyntaxKind::PARAGRAPH),
            Event::Token {
                kind: SyntaxKind::TEXT,
                n_raw_tokens: 1, // 1 TEXT token containing "hello"
            },
            Event::Finish,
            Event::Finish,
        ];

        let sink = Sink::new(&tokens, events);
        let tree = sink.finish();

        assert_eq!(tree.text().to_string(), input);
    }
}
