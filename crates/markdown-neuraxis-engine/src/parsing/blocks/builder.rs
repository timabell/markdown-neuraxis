use crate::parsing::rope::span::Span;

use super::{
    classify::LineClass,
    containers::ContainerPath,
    kinds::{CodeFence, FenceKind},
    open::{BlockOpen, try_open_leaf},
    types::{BlockKind, BlockNode},
};

/// Internal state for the current leaf block being built.
#[derive(Debug, Clone, Copy)]
enum LeafState {
    /// No leaf block is currently open.
    None,
    /// Building a paragraph.
    Paragraph {
        start: Span,
        content_start: Span,
        last_line_end: usize,
    },
    /// Inside a fenced code block (raw zone).
    Fence {
        kind: FenceKind,
        start: Span,
        last_line_end: usize,
    },
}

/// State machine for building blocks from classified lines.
///
/// Phase 2 of block parsing: receives [`LineClass`] values and emits
/// [`BlockNode`]s as blocks open and close.
///
/// # Usage
///
/// ```ignore
/// let mut builder = BlockBuilder::new();
/// for line in lines {
///     let class = classifier.classify(&line);
///     builder.push(&class);
/// }
/// let blocks = builder.finish();
/// ```
pub struct BlockBuilder {
    containers: ContainerPath,
    leaf: LeafState,
    out: Vec<BlockNode>,
}

impl BlockBuilder {
    /// Creates a new block builder with empty state.
    pub fn new() -> Self {
        Self {
            containers: ContainerPath::default(),
            leaf: LeafState::None,
            out: vec![],
        }
    }

    /// Processes a classified line, updating internal state and emitting blocks as needed.
    pub fn push(&mut self, c: &LineClass) {
        self.containers.set_blockquote_depth(c.quote_depth);

        if self.in_fence() {
            self.consume_fence_line(c);
            return;
        }

        if c.is_blank {
            self.flush_paragraph();
            return;
        }

        if let Some(open) = try_open_leaf(&c.remainder_text) {
            self.flush_paragraph();
            self.open_leaf(open, c.line);
            return;
        }

        self.extend_paragraph(c.line, c.remainder_span);
    }

    /// Finishes parsing and returns all emitted blocks.
    ///
    /// Flushes any in-progress paragraph or unterminated fence.
    pub fn finish(mut self) -> Vec<BlockNode> {
        // EOF flush
        self.flush_paragraph();
        self.flush_fence();
        self.out
    }

    /// Returns true if currently inside a fenced code block.
    fn in_fence(&self) -> bool {
        matches!(self.leaf, LeafState::Fence { .. })
    }

    /// Opens a new leaf block based on the detected opener.
    fn open_leaf(&mut self, open: BlockOpen, line: Span) {
        match open {
            BlockOpen::FencedCode { kind } => {
                self.leaf = LeafState::Fence {
                    kind,
                    start: line,
                    last_line_end: line.end,
                }
            }
        }
    }

    /// Processes a line while inside a fenced code block.
    ///
    /// Updates the fence span and closes it if a matching fence is found.
    fn consume_fence_line(&mut self, c: &LineClass) {
        let (kind, start, _last_end) = match self.leaf {
            LeafState::Fence {
                kind,
                start,
                last_line_end,
            } => (kind, start, last_line_end),
            _ => return,
        };

        // Update last line end
        self.leaf = LeafState::Fence {
            kind,
            start,
            last_line_end: c.line.end,
        };

        // Close if this line "looks like fence" with same sig.
        if CodeFence::closes(kind, c.fence_sig) {
            self.out.push(BlockNode {
                containers: self.containers.0.clone(),
                kind: BlockKind::FencedCode { kind },
                span: Span {
                    start: start.start,
                    end: c.line.end,
                },
                content_span: Span {
                    start: start.start,
                    end: c.line.end,
                },
            });
            self.leaf = LeafState::None;
        }
    }

    /// Extends the current paragraph or starts a new one.
    fn extend_paragraph(&mut self, line: Span, content_span: Span) {
        match self.leaf {
            LeafState::Paragraph {
                start,
                content_start,
                ..
            } => {
                self.leaf = LeafState::Paragraph {
                    start,
                    content_start,
                    last_line_end: line.end,
                };
            }
            _ => {
                self.leaf = LeafState::Paragraph {
                    start: line,
                    content_start: content_span,
                    last_line_end: line.end,
                };
            }
        }
    }

    /// Emits the current paragraph block if one is in progress.
    ///
    /// Restores non-paragraph leaf state (e.g., fence) if not a paragraph.
    fn flush_paragraph(&mut self) {
        let prev = std::mem::replace(&mut self.leaf, LeafState::None);
        if let LeafState::Paragraph {
            start,
            content_start,
            last_line_end,
        } = prev
        {
            self.out.push(BlockNode {
                containers: self.containers.0.clone(),
                kind: BlockKind::Paragraph,
                span: Span {
                    start: start.start,
                    end: last_line_end,
                },
                content_span: Span {
                    start: content_start.start,
                    end: last_line_end,
                },
            });
        } else {
            self.leaf = prev; // put back non-paragraph leaf (e.g. fence)
        }
    }

    /// Emits an unterminated fence block at EOF.
    fn flush_fence(&mut self) {
        let prev = std::mem::replace(&mut self.leaf, LeafState::None);
        if let LeafState::Fence {
            kind,
            start,
            last_line_end,
        } = prev
        {
            // Unterminated fence: emit as fence block anyway
            self.out.push(BlockNode {
                containers: self.containers.0.clone(),
                kind: BlockKind::FencedCode { kind },
                span: Span {
                    start: start.start,
                    end: last_line_end,
                },
                content_span: Span {
                    start: start.start,
                    end: last_line_end,
                },
            });
        }
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}
