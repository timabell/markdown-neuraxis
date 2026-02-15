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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::blocks::kinds::FenceSig;

    /// Helper to create a LineClass for testing state transitions.
    fn line_class(
        start: usize,
        end: usize,
        is_blank: bool,
        remainder: &str,
        fence_sig: Option<FenceSig>,
    ) -> LineClass {
        LineClass {
            line: Span { start, end },
            is_blank,
            quote_depth: 0,
            remainder_span: Span { start, end },
            remainder_text: remainder.to_string(),
            fence_sig,
        }
    }

    #[test]
    fn fence_opened_then_blank_line_continues_fence() {
        let mut builder = BlockBuilder::new();

        // Open a backtick fence
        builder.push(&line_class(
            0,
            8,
            false,
            "```rust\n",
            Some(FenceSig::Backticks),
        ));

        // Push a blank line inside the fence
        builder.push(&line_class(8, 9, true, "\n", None));

        // Fence should still be open (no blocks emitted yet)
        assert!(builder.out.is_empty());
        assert!(builder.in_fence());

        // Close the fence
        builder.push(&line_class(
            9,
            13,
            false,
            "```\n",
            Some(FenceSig::Backticks),
        ));

        // Now we should have one block
        assert_eq!(builder.out.len(), 1);
        assert!(matches!(
            builder.out[0].kind,
            BlockKind::FencedCode {
                kind: FenceKind::Backticks
            }
        ));
    }

    #[test]
    fn paragraph_followed_immediately_by_fence() {
        let mut builder = BlockBuilder::new();

        // Start a paragraph
        builder.push(&line_class(0, 6, false, "hello\n", None));

        // Immediately open a fence (should flush paragraph first)
        builder.push(&line_class(
            6,
            14,
            false,
            "```rust\n",
            Some(FenceSig::Backticks),
        ));

        // Paragraph should be emitted
        assert_eq!(builder.out.len(), 1);
        assert!(matches!(builder.out[0].kind, BlockKind::Paragraph));

        // Fence should be open
        assert!(builder.in_fence());

        // Close fence
        builder.push(&line_class(
            14,
            18,
            false,
            "```\n",
            Some(FenceSig::Backticks),
        ));

        // Now two blocks
        let blocks = builder.finish();
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0].kind, BlockKind::Paragraph));
        assert!(matches!(
            blocks[1].kind,
            BlockKind::FencedCode {
                kind: FenceKind::Backticks
            }
        ));
    }

    #[test]
    fn fence_closed_then_immediate_fence_open() {
        let mut builder = BlockBuilder::new();

        // Open and close a backtick fence
        builder.push(&line_class(0, 4, false, "```\n", Some(FenceSig::Backticks)));
        builder.push(&line_class(4, 8, false, "```\n", Some(FenceSig::Backticks)));

        assert_eq!(builder.out.len(), 1);

        // Immediately open another fence (tildes this time)
        builder.push(&line_class(8, 12, false, "~~~\n", Some(FenceSig::Tildes)));
        builder.push(&line_class(12, 16, false, "~~~\n", Some(FenceSig::Tildes)));

        let blocks = builder.finish();
        assert_eq!(blocks.len(), 2);
        assert!(matches!(
            blocks[0].kind,
            BlockKind::FencedCode {
                kind: FenceKind::Backticks
            }
        ));
        assert!(matches!(
            blocks[1].kind,
            BlockKind::FencedCode {
                kind: FenceKind::Tildes
            }
        ));
    }

    #[test]
    fn multiple_consecutive_blank_lines() {
        let mut builder = BlockBuilder::new();

        // Start a paragraph
        builder.push(&line_class(0, 6, false, "hello\n", None));

        // Multiple blank lines
        builder.push(&line_class(6, 7, true, "\n", None));
        builder.push(&line_class(7, 8, true, "\n", None));
        builder.push(&line_class(8, 9, true, "\n", None));

        // Paragraph should be emitted after first blank
        assert_eq!(builder.out.len(), 1);

        // Another paragraph
        builder.push(&line_class(9, 15, false, "world\n", None));

        let blocks = builder.finish();
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0].kind, BlockKind::Paragraph));
        assert!(matches!(blocks[1].kind, BlockKind::Paragraph));
        // First paragraph span
        assert_eq!(blocks[0].span.start, 0);
        assert_eq!(blocks[0].span.end, 6);
        // Second paragraph span
        assert_eq!(blocks[1].span.start, 9);
        assert_eq!(blocks[1].span.end, 15);
    }

    #[test]
    fn empty_document_produces_no_blocks() {
        let builder = BlockBuilder::new();
        let blocks = builder.finish();
        assert!(blocks.is_empty());
    }

    #[test]
    fn only_blank_lines_produce_no_blocks() {
        let mut builder = BlockBuilder::new();
        builder.push(&line_class(0, 1, true, "\n", None));
        builder.push(&line_class(1, 2, true, "\n", None));
        let blocks = builder.finish();
        assert!(blocks.is_empty());
    }
}
