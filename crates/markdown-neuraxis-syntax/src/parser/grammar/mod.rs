//! # Grammar Rules
//!
//! This module contains the grammar rules that drive parsing. Each function
//! takes a `&mut Parser` and uses its methods to:
//!
//! 1. Inspect the current token (`p.current()`, `p.at()`, `p.nth()`)
//! 2. Consume tokens (`p.bump()`, `p.eat()`)
//! 3. Build tree structure (`p.start()` → marker → `complete()`/`abandon()`)
//!
//! ## Module Structure
//!
//! - [`block`] - Block-level elements (headings, paragraphs, lists, code blocks)
//! - [`inline`] - Inline elements (links, emphasis, code spans)
//!
//! ## Writing Grammar Rules
//!
//! A typical grammar function looks like:
//!
//! ```ignore
//! fn heading(p: &mut Parser) {
//!     let m = p.start();           // 1. Start a node
//!
//!     while p.at(SyntaxKind::HASH) {  // 2. Consume tokens
//!         p.bump();
//!     }
//!     p.eat(SyntaxKind::WHITESPACE);
//!     inline::inline_until_newline(p); // 3. Call other grammar rules
//!     p.eat(SyntaxKind::NEWLINE);
//!
//!     m.complete(p, SyntaxKind::HEADING); // 4. Complete the node
//! }
//! ```
//!
//! ## Error Recovery
//!
//! Grammar functions should be lenient - produce a tree even for invalid input.
//! When something unexpected happens:
//!
//! - Wrap unexpected tokens in an ERROR node
//! - Or just consume them into the current node
//! - Avoid panicking or returning errors
//!
//! The goal is a valid tree that preserves all input bytes.

mod block;
mod inline;

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

/// Parse the root document.
///
/// This is the entry point for parsing. It creates a ROOT node containing
/// all top-level blocks in the document.
pub fn root(p: &mut Parser<'_, '_>) {
    let m = p.start();

    while !p.at_end() {
        block::block(p);
    }

    m.complete(p, SyntaxKind::ROOT);
}
