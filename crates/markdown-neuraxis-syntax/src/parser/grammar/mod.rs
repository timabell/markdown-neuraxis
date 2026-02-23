//! Grammar rules for Markdown parsing.

mod block;
mod inline;

use crate::parser::Parser;
use crate::syntax_kind::SyntaxKind;

/// Parse the root document.
pub fn root(p: &mut Parser<'_, '_>) {
    let m = p.start();

    while !p.at_end() {
        block::block(p);
    }

    m.complete(p, SyntaxKind::ROOT);
}
