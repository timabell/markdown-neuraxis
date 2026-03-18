//! # Parser Events
//!
//! Events are the intermediate representation between parsing and tree building.
//! Instead of building the tree directly, the parser emits a **flat sequence**
//! of events that describe the tree structure.
//!
//! ## Why Events?
//!
//! This indirection provides several benefits:
//!
//! 1. **Decoupling**: Grammar code doesn't know about Rowan internals
//! 2. **Forward parent links**: Enables the "precede" pattern for left-recursion
//! 3. **Simplicity**: Events are easy to reason about and debug
//! 4. **Flexibility**: Could theoretically target different tree backends
//!
//! ## Event Types
//!
//! The four event types form a simple protocol:
//!
//! ```text
//! Start(HEADING)     ← Begin a HEADING node
//!   Token(HASH)      ← Add a HASH token
//!   Token(WHITESPACE)
//!   Token(TEXT)
//!   Token(NEWLINE)
//! Finish             ← End the HEADING node
//! ```
//!
//! The Sink processes these in order, maintaining a stack of open nodes.
//! Start pushes, Finish pops.
//!
//! ## Forward Parent Links
//!
//! The `forward_parent` field in `Start` handles cases where we need to wrap
//! an already-parsed node. Instead of restructuring the event list, we store
//! a link that says "when you process me, also process that other Start first."
//!
//! The Sink resolves these links by following the chain and opening nodes
//! in the correct (outermost-first) order.

use crate::syntax_kind::SyntaxKind;

/// An event emitted by the parser during tree construction.
///
/// Events form a flat representation of the tree that the [`Sink`](super::sink::Sink)
/// converts into an actual Rowan tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Begin a new composite node.
    ///
    /// The `kind` specifies what type of node (PARAGRAPH, HEADING, etc.).
    ///
    /// The `forward_parent` field is used for the "precede" pattern:
    /// if set, it points to another `Start` event that should become
    /// this node's parent. The Sink follows these links to build the
    /// correct nesting structure.
    Start {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },

    /// Add a token to the current node.
    ///
    /// The `kind` is typically the same as the lexer token, but can differ
    /// (e.g., grouping multiple raw tokens into one semantic token).
    ///
    /// The `n_raw_tokens` field says how many lexer tokens this event
    /// consumes. Usually 1, but can be more when grouping.
    Token { kind: SyntaxKind, n_raw_tokens: u8 },

    /// Finish the current node.
    ///
    /// Must be paired with a preceding `Start`. The Sink pops the node
    /// stack when it sees this.
    Finish,

    /// A placeholder that will be replaced.
    ///
    /// When `parser.start()` is called, a `Placeholder` is pushed. Later,
    /// `marker.complete()` replaces it with a real `Start`, or
    /// `marker.abandon()` leaves it (the Sink ignores placeholders).
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
