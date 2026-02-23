//! Parser events following the rust-analyzer model.
//!
//! Events are an intermediate representation between parsing and CST construction.
//! This allows the parser to emit a flat sequence of events that the sink then
//! converts into a tree structure.

use crate::syntax_kind::SyntaxKind;

/// An event emitted by the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Start a new node.
    ///
    /// The `forward_parent` field is used for left-recursion handling.
    /// It points to the index of a preceding `Start` event that should
    /// become this node's parent. This is resolved during tree construction.
    Start {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },
    /// A token to add to the current node.
    Token { kind: SyntaxKind, n_raw_tokens: u8 },
    /// Finish the current node.
    Finish,
    /// Placeholder event (replaced during tree construction).
    Placeholder,
}

impl Event {
    /// Create a start event with no forward parent.
    pub fn start(kind: SyntaxKind) -> Self {
        Event::Start {
            kind,
            forward_parent: None,
        }
    }

    /// Create a token event for a single raw token.
    pub fn token(kind: SyntaxKind) -> Self {
        Event::Token {
            kind,
            n_raw_tokens: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_start_creation() {
        let event = Event::start(SyntaxKind::PARAGRAPH);
        assert_eq!(
            event,
            Event::Start {
                kind: SyntaxKind::PARAGRAPH,
                forward_parent: None
            }
        );
    }

    #[test]
    fn event_token_creation() {
        let event = Event::token(SyntaxKind::TEXT);
        assert_eq!(
            event,
            Event::Token {
                kind: SyntaxKind::TEXT,
                n_raw_tokens: 1
            }
        );
    }
}
