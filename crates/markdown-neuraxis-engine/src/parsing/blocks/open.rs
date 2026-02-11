use super::kinds::{CodeFence, FenceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOpen {
    FencedCode { kind: FenceKind },
    // Later: Heading, ThematicBreak, HtmlBlock...
}

pub fn try_open_leaf(remainder: &str) -> Option<BlockOpen> {
    // Precedence: fence beats everything else.
    if let Some(sig) = CodeFence::sig(remainder) {
        return Some(BlockOpen::FencedCode {
            kind: CodeFence::kind(sig),
        });
    }
    None
}
