use super::kinds::{CodeFence, FenceKind};

/// Signals that a new leaf block should be opened.
///
/// Returned by [`try_open_leaf`] when the line remainder matches a block opener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOpen {
    /// Open a fenced code block.
    FencedCode {
        /// Whether backticks or tildes were used.
        kind: FenceKind,
    },
    // Later: Heading, ThematicBreak, HtmlBlock...
}

/// Attempts to detect a leaf block opener in the line remainder.
///
/// This is the single dispatch point for block opener precedence.
/// Currently only detects fenced code blocks; will be extended for
/// headings, thematic breaks, etc.
///
/// Returns `None` if the line should continue/start a paragraph.
pub fn try_open_leaf(remainder: &str) -> Option<BlockOpen> {
    // Precedence: fence beats everything else.
    if let Some(sig) = CodeFence::sig(remainder) {
        return Some(BlockOpen::FencedCode {
            kind: CodeFence::kind(sig),
        });
    }
    None
}
