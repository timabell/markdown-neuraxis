//! Sink for converting parser events into a Rowan green tree.

use rowan::GreenNodeBuilder;

use crate::lexer::Token;
use crate::parser::event::Event;
use crate::syntax_kind::{SyntaxKind, SyntaxNode};

/// Converts parser events and tokens into a Rowan syntax tree.
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
