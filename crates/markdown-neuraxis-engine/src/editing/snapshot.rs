//! # Snapshot V2: Tree-Structured Document Projection
//!
//! This module provides ergonomic primitives for the editor UI by exposing
//! the document structure as a tree of blocks with inline segments.
//!
//! ## Design Goals (from ADR-0012)
//!
//! - Keep all "wtf is this string" complexity in the snapshot layer
//! - Editor gets clean primitives without understanding markdown syntax
//! - Use segments for both rendering and editing ranges

use std::ops::Range;

use markdown_neuraxis_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, parse};

use crate::editing::{Anchor, AnchorId};

/// Content of a block: either leaf (no children) or nested children
#[derive(Debug, Clone, PartialEq)]
pub enum BlockContent {
    /// Leaf block - content available via segments
    Leaf,
    /// Container block with child blocks
    Children(Vec<Block>),
}

/// A segment of inline content with source byte range.
/// The InlineNode may contain recursively nested formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSegment {
    /// The kind of segment with its content
    pub kind: InlineNode,
    /// Byte range in source (for verification/debugging)
    pub range: Range<usize>,
}

/// Recursive inline node for nested formatting (ADR-0013)
#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    /// Plain text content
    Text(String),
    /// Strong emphasis (**text**) - contains children for nested formatting
    Strong(Vec<InlineNode>),
    /// Emphasis (*text*) - contains children for nested formatting
    Emphasis(Vec<InlineNode>),
    /// Inline code (`code`) - leaf node
    Code(String),
    /// Strikethrough (~~text~~) - leaf node for now
    Strikethrough(String),
    /// Wiki link [[target]] or [[target|alias]]
    WikiLink {
        target: String,
        alias: Option<String>,
    },
    /// Standard markdown link [text](url)
    Link { text: String, url: String },
    /// Image ![alt](url)
    Image { alt: String, url: String },
    /// Hard line break (two trailing spaces + newline)
    HardBreak,
    /// Soft line break (newline absorbed during line wrapping, renders as space)
    SoftBreak,
}

/// The kind of block
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    /// Root document container
    Root,
    /// List container (wraps LIST_ITEMs)
    List { ordered: bool },
    /// Individual list item
    ListItem { marker: String },
    /// Blockquote (can span multiple lines)
    BlockQuote,
    /// Paragraph
    Paragraph,
    /// ATX heading
    Heading { level: u8 },
    /// Fenced code block
    FencedCode { language: Option<String> },
    /// Thematic break
    ThematicBreak,
}

/// A block in the document tree
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Block identifier for stable references
    pub id: AnchorId,
    /// What kind of block this is
    pub kind: BlockKind,
    /// This block's full span in the source
    pub node_range: Range<usize>,
    /// Inline content for rendering. Top-level siblings with byte ranges;
    /// each segment's InlineNode may contain recursive nested formatting.
    pub segments: Vec<InlineSegment>,
    /// Block content (text or children)
    pub content: BlockContent,
}

impl Block {
    /// Byte range of this block's content in the source.
    ///
    /// For leaf blocks, this equals `node_range`.
    /// For blocks with children, this is the range up to the last segment's end -
    /// includes the marker and text content, but excludes structural whitespace
    /// and nested blocks.
    pub fn content_range(&self) -> Range<usize> {
        match &self.content {
            BlockContent::Leaf => self.node_range.clone(),
            BlockContent::Children(_) => {
                // Use last segment's end to exclude structural whitespace
                // between content and nested children
                if let Some(last_segment) = self.segments.last() {
                    self.node_range.start..last_segment.range.end
                } else {
                    // No segments - use full node_range
                    self.node_range.clone()
                }
            }
        }
    }
}

/// Tree-structured document snapshot
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Root-level blocks
    pub blocks: Vec<Block>,
}

/// Create a snapshot from a document
pub fn create_snapshot(doc: &crate::editing::Document) -> Snapshot {
    let source = doc.text();
    if source.is_empty() {
        return Snapshot { blocks: vec![] };
    }

    // Parse using Rowan parser
    let tree = parse(&source);
    let mut blocks = Vec::new();

    // Process top-level children, passing anchors for ID lookup
    let anchors = &doc.anchors;
    for child in tree.children() {
        if let Some(block) = process_node(&source, child, anchors) {
            blocks.push(block);
        }
    }

    // Consolidate consecutive blockquotes into single blocks
    let blocks = consolidate_blockquotes(blocks, &source);

    Snapshot { blocks }
}

/// Consolidate consecutive blockquotes into single blocks.
///
/// In Markdown, consecutive lines starting with `>` form a single blockquote.
/// The parser produces separate BLOCK_QUOTE nodes for each line, so we merge
/// them here into a coherent structure:
///
/// - Consecutive leaf blockquotes: merge segments into one block
/// - Nested blockquotes (`> > text`): become children of the merged block
/// - Mixed content (`> text` followed by `> > nested`): text in segments, nested in children
///
/// Blockquotes are considered consecutive if they're adjacent or separated only
/// by whitespace (indentation). A blank line (newline character in the gap)
/// breaks the sequence.
fn consolidate_blockquotes(blocks: Vec<Block>, source: &str) -> Vec<Block> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        if blocks[i].kind == BlockKind::BlockQuote {
            // Start of a blockquote run - find extent and consolidate
            let run_start = i;
            let mut run_end = i + 1;

            // Find all consecutive blockquotes (adjacent or separated only by indentation)
            while run_end < blocks.len()
                && blocks[run_end].kind == BlockKind::BlockQuote
                && is_whitespace_only_gap(
                    source,
                    blocks[run_end - 1].node_range.end,
                    blocks[run_end].node_range.start,
                )
            {
                run_end += 1;
            }

            if run_end - run_start == 1 {
                // Single blockquote - no consolidation needed, but recurse into children
                let mut block = blocks[i].clone();
                if let BlockContent::Children(children) = block.content {
                    block.content =
                        BlockContent::Children(consolidate_blockquotes(children, source));
                }
                result.push(block);
            } else {
                // Multiple consecutive blockquotes - merge them
                result.push(merge_blockquote_run(&blocks[run_start..run_end], source));
            }
            i = run_end;
        } else {
            // Non-blockquote: recurse into children if present
            let mut block = blocks[i].clone();
            if let BlockContent::Children(children) = block.content {
                block.content = BlockContent::Children(consolidate_blockquotes(children, source));
            }
            result.push(block);
            i += 1;
        }
    }

    result
}

/// Check if the gap between two ranges allows consolidation.
/// Returns true if blocks should be consolidated, false if separated by blank line.
///
/// Blockquotes are consecutive if the gap contains only:
/// - Whitespace (spaces/tabs) for indentation
/// - At most one newline followed by non-newline content
///
/// A blank line (empty line) is detected as a newline followed immediately by
/// another newline (or end of gap), which breaks the sequence.
fn is_whitespace_only_gap(source: &str, end: usize, start: usize) -> bool {
    if start <= end {
        // Adjacent or overlapping - consolidate
        return true;
    }
    let gap = &source[end..start];

    // Check for blank line pattern: newline followed by only whitespace and/or newline
    // A gap like "\n" (single newline, nothing after) indicates a blank line
    // A gap like "\n  " (newline then spaces) indicates indentation continuation
    // A gap like "\n\n" or "\n  \n" indicates a blank line

    let chars: Vec<char> = gap.chars().collect();
    let newline_count = chars.iter().filter(|&&c| c == '\n').count();

    if newline_count == 0 {
        // No newlines - just whitespace, consolidate
        return chars.iter().all(|&c| c == ' ' || c == '\t');
    }

    if newline_count >= 2 {
        // Two or more newlines means blank line - don't consolidate
        return false;
    }

    // Exactly one newline - check if there's content after it
    // If the gap is just "\n" (newline at end with nothing after), it's a blank line
    // If there's whitespace after the newline, it's indentation for continuation
    if let Some(nl_pos) = chars.iter().position(|&c| c == '\n') {
        let after_newline = &chars[nl_pos + 1..];
        // Must have some non-empty content after newline (indentation)
        !after_newline.is_empty()
    } else {
        true
    }
}

/// Unconditionally merge all consecutive blockquotes in the list.
/// Used for children of already-merged parents where gap checking isn't needed.
fn merge_consecutive_blockquotes(blocks: Vec<Block>, source: &str) -> Vec<Block> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        if blocks[i].kind == BlockKind::BlockQuote {
            // Find extent of consecutive blockquotes (no gap checking)
            let run_start = i;
            let mut run_end = i + 1;
            while run_end < blocks.len() && blocks[run_end].kind == BlockKind::BlockQuote {
                run_end += 1;
            }

            if run_end - run_start == 1 {
                // Single blockquote - recurse into children
                let mut block = blocks[i].clone();
                if let BlockContent::Children(children) = block.content {
                    block.content =
                        BlockContent::Children(merge_consecutive_blockquotes(children, source));
                }
                result.push(block);
            } else {
                // Multiple consecutive - merge them unconditionally
                result.push(merge_blockquote_run_unchecked(
                    &blocks[run_start..run_end],
                    source,
                ));
            }
            i = run_end;
        } else {
            // Non-blockquote: recurse into children
            let mut block = blocks[i].clone();
            if let BlockContent::Children(children) = block.content {
                block.content =
                    BlockContent::Children(merge_consecutive_blockquotes(children, source));
            }
            result.push(block);
            i += 1;
        }
    }

    result
}

/// Create a SoftBreak segment to represent an absorbed newline between merged lines.
/// The range is placed at the end of the previous segments (where the newline was).
fn soft_break_segment(prev_segments: &[InlineSegment]) -> InlineSegment {
    let pos = prev_segments.last().map(|s| s.range.end).unwrap_or(0);
    InlineSegment {
        kind: InlineNode::SoftBreak,
        range: pos..pos,
    }
}

/// Merge blockquotes without gap checking (used for children of merged parents).
/// Content is organized into Paragraph children, with nested BlockQuotes interspersed.
fn merge_blockquote_run_unchecked(blocks: &[Block], source: &str) -> Block {
    assert!(!blocks.is_empty());

    let mut children: Vec<Block> = Vec::new();
    // Current paragraph being built
    let mut para_segments: Vec<InlineSegment> = Vec::new();
    let mut para_start: Option<usize> = None;
    let mut para_end: usize = 0;
    let mut para_id: Option<AnchorId> = None;

    // Helper to finalize current paragraph into children
    let finalize_paragraph = |children: &mut Vec<Block>,
                              segments: &mut Vec<InlineSegment>,
                              start: &mut Option<usize>,
                              end: usize,
                              id: &mut Option<AnchorId>| {
        if !segments.is_empty() {
            let range = start.unwrap_or(0)..end;
            children.push(Block {
                id: id
                    .take()
                    .expect("paragraph must have ID from first contributing block"),
                kind: BlockKind::Paragraph,
                node_range: range,
                segments: std::mem::take(segments),
                content: BlockContent::Leaf,
            });
            *start = None;
        }
    };

    for block in blocks {
        // If block has nested children, finalize current para and add them
        if let BlockContent::Children(nested) = &block.content {
            finalize_paragraph(
                &mut children,
                &mut para_segments,
                &mut para_start,
                para_end,
                &mut para_id,
            );
            children.extend(nested.clone());
        }

        if block.segments.is_empty() {
            // Empty blockquote line - finalize current paragraph
            finalize_paragraph(
                &mut children,
                &mut para_segments,
                &mut para_start,
                para_end,
                &mut para_id,
            );
        } else {
            // Content line - add to current paragraph
            if !para_segments.is_empty() {
                // Insert SoftBreak between consecutive lines (unless after HardBreak)
                let last_is_hardbreak = para_segments
                    .last()
                    .is_some_and(|s: &InlineSegment| matches!(s.kind, InlineNode::HardBreak));
                if !last_is_hardbreak {
                    para_segments.push(soft_break_segment(&para_segments));
                }
            } else {
                // First content of this paragraph - capture start position and ID
                para_start = Some(block.node_range.start);
                para_id = Some(block.id);
            }
            para_segments.extend(block.segments.clone());
            para_end = block.node_range.end;
        }
    }

    // Finalize any remaining paragraph
    finalize_paragraph(
        &mut children,
        &mut para_segments,
        &mut para_start,
        para_end,
        &mut para_id,
    );

    // Recursively merge any consecutive blockquotes in children
    let children = merge_consecutive_blockquotes(children, source);

    let first = blocks.first().unwrap();
    let last = blocks.last().unwrap();
    let merged_range = first.node_range.start..last.node_range.end;
    let id = first.id;

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range: merged_range,
        segments: vec![], // BlockQuote content is now in Paragraph children
        content,
    }
}

/// Merge a run of consecutive blockquote blocks into a single block.
/// Content is organized into Paragraph children, with nested BlockQuotes interspersed.
fn merge_blockquote_run(blocks: &[Block], source: &str) -> Block {
    assert!(!blocks.is_empty());

    let mut children: Vec<Block> = Vec::new();
    // Current paragraph being built
    let mut para_segments: Vec<InlineSegment> = Vec::new();
    let mut para_start: Option<usize> = None;
    let mut para_end: usize = 0;
    let mut para_id: Option<AnchorId> = None;

    // Helper to finalize current paragraph into children
    let finalize_paragraph = |children: &mut Vec<Block>,
                              segments: &mut Vec<InlineSegment>,
                              start: &mut Option<usize>,
                              end: usize,
                              id: &mut Option<AnchorId>| {
        if !segments.is_empty() {
            let range = start.unwrap_or(0)..end;
            children.push(Block {
                id: id
                    .take()
                    .expect("paragraph must have ID from first contributing block"),
                kind: BlockKind::Paragraph,
                node_range: range,
                segments: std::mem::take(segments),
                content: BlockContent::Leaf,
            });
            *start = None;
        }
    };

    for block in blocks {
        // If block has nested children, finalize current para and add them
        if let BlockContent::Children(nested) = &block.content {
            finalize_paragraph(
                &mut children,
                &mut para_segments,
                &mut para_start,
                para_end,
                &mut para_id,
            );
            children.extend(nested.clone());
        }

        if block.segments.is_empty() {
            // Empty blockquote line - finalize current paragraph
            finalize_paragraph(
                &mut children,
                &mut para_segments,
                &mut para_start,
                para_end,
                &mut para_id,
            );
        } else {
            // Content line - add to current paragraph
            if !para_segments.is_empty() {
                // Insert SoftBreak between consecutive lines (unless after HardBreak)
                let last_is_hardbreak = para_segments
                    .last()
                    .is_some_and(|s: &InlineSegment| matches!(s.kind, InlineNode::HardBreak));
                if !last_is_hardbreak {
                    para_segments.push(soft_break_segment(&para_segments));
                }
            } else {
                // First content of this paragraph - capture start position and ID
                para_start = Some(block.node_range.start);
                para_id = Some(block.id);
            }
            para_segments.extend(block.segments.clone());
            para_end = block.node_range.end;
        }
    }

    // Finalize any remaining paragraph
    finalize_paragraph(
        &mut children,
        &mut para_segments,
        &mut para_start,
        para_end,
        &mut para_id,
    );

    // Recursively merge any consecutive blockquotes in children
    let children = merge_consecutive_blockquotes(children, source);

    // Compute merged range
    let first = blocks.first().unwrap();
    let last = blocks.last().unwrap();
    let merged_range = first.node_range.start..last.node_range.end;

    // Use the first block's ID (it represents the start of the blockquote)
    let id = first.id;

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range: merged_range,
        segments: vec![], // BlockQuote content is now in Paragraph children
        content,
    }
}

/// Find the best matching anchor for a given byte range.
///
/// Matching strategy:
/// 1. Exact range match
/// 2. Start position match
/// 3. Fallback: generate deterministic ID from range
///
/// The start position match handles cases where anchor ranges may differ
/// (e.g., list item anchors stop before nested children) while Rowan
/// produces full ranges. Both should start at the same position.
fn find_anchor_for_range(anchors: &[Anchor], range: &Range<usize>) -> AnchorId {
    // First try: exact range match
    for anchor in anchors {
        if anchor.range == *range {
            return anchor.id;
        }
    }

    // Second try: start position match
    for anchor in anchors {
        if anchor.range.start == range.start {
            return anchor.id;
        }
    }

    // Fallback: generate deterministic ID from range
    generate_fallback_anchor_id(range)
}

/// Generate a fallback anchor ID when no matching anchor is found.
/// Uses a simple hash of the range for determinism.
fn generate_fallback_anchor_id(range: &Range<usize>) -> AnchorId {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    // Magic number to differentiate from static/dynamic IDs in anchors.rs
    let magic = 0xabcdef0123456789u64;
    magic.hash(&mut hasher);
    range.start.hash(&mut hasher);
    range.end.hash(&mut hasher);

    AnchorId(hasher.finish() as u128)
}

/// Process a Rowan node into a Block
fn process_node(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    match node.kind() {
        SyntaxKind::ORDERED_LIST => process_list(source, node, anchors, true),
        SyntaxKind::UNORDERED_LIST => process_list(source, node, anchors, false),
        SyntaxKind::LIST_ITEM => process_list_item(source, node, anchors),
        SyntaxKind::PARAGRAPH => process_paragraph(source, node, anchors),
        SyntaxKind::BLOCK_QUOTE => process_block_quote(source, node, anchors),
        SyntaxKind::HEADING => process_heading(source, node, anchors),
        SyntaxKind::FENCED_CODE => process_fenced_code(source, node, anchors),
        SyntaxKind::THEMATIC_BREAK => process_thematic_break(source, node, anchors),
        _ => None, // Skip unknown node types
    }
}

fn process_list(
    source: &str,
    node: SyntaxNode,
    anchors: &[Anchor],
    ordered: bool,
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let mut children = Vec::new();

    for child in node.children() {
        if let Some(block) = process_node(source, child, anchors) {
            children.push(block);
        }
    }

    // LIST containers don't have their own anchors - only leaf/editable
    // blocks (list_item, heading, fenced_code, etc.) get anchors.
    // Generate a fallback ID directly without trying to match anchors,
    // to avoid accidentally stealing an anchor from a child LIST_ITEM.
    let id = generate_fallback_anchor_id(&node_range);

    Some(Block {
        id,
        kind: BlockKind::List { ordered },
        node_range,
        segments: vec![],
        content: BlockContent::Children(children),
    })
}

fn process_list_item(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Extract marker from first line
    let first_line_content_end = text.find('\n').unwrap_or(text.len());
    let first_line = &text[..first_line_content_end];
    let marker = extract_list_marker(first_line);
    let marker_len = marker.len();

    // Content starts after marker, ends before newline
    let content_start = node_range.start + marker_len;
    let fallback_content_end = node_range.start + first_line_content_end;

    // Process children (nested content)
    // Skip PARAGRAPH children - segments are extracted separately below.
    let mut children = Vec::new();
    for child in node.children() {
        // Skip PARAGRAPH inside list items - the list item already extracted its text
        if child.kind() == SyntaxKind::PARAGRAPH {
            continue;
        }
        if let Some(block) = process_node(source, child, anchors) {
            children.push(block);
        }
    }

    // Consolidate consecutive blockquotes in children
    let children = consolidate_blockquotes(children, source);

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    // Look up anchor ID for this list item
    let id = find_anchor_for_range(anchors, &node_range);

    // Extract segments from the list item's content
    // We look in the PARAGRAPH child (if present) since that's where inlines live
    let segments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::PARAGRAPH)
        .map(|para| {
            // Use paragraph's range, but start after the list marker
            // and exclude trailing newline
            let para_range = para.text_range();
            let para_start: usize = para_range.start().into();
            let mut para_end: usize = para_range.end().into();
            // Strip trailing newline - not needed for editing or segment extraction
            if para_end > para_start && source.as_bytes().get(para_end - 1) == Some(&b'\n') {
                para_end -= 1;
            }
            let content_range = content_start.max(para_start)..para_end;
            extract_segments(&para, source, content_range)
        })
        .unwrap_or_else(|| {
            // No paragraph child - use first line content range as fallback
            let fallback_range = content_start..fallback_content_end;
            if !fallback_range.is_empty() {
                let text = &source[fallback_range.clone()];
                if !text.is_empty() {
                    return vec![InlineSegment {
                        kind: InlineNode::Text(text.to_string()),
                        range: fallback_range,
                    }];
                }
            }
            vec![]
        });

    Some(Block {
        id,
        kind: BlockKind::ListItem { marker },
        node_range,
        segments,
        content,
    })
}

fn process_paragraph(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());

    // Content range: strip trailing newline if present
    let content_end = if node_range.end > node_range.start
        && source.as_bytes().get(node_range.end - 1) == Some(&b'\n')
    {
        node_range.end - 1
    } else {
        node_range.end
    };
    let content_range = node_range.start..content_end;
    let segments = extract_segments(&node, source, content_range);

    let id = find_anchor_for_range(anchors, &node_range);

    Some(Block {
        id,
        kind: BlockKind::Paragraph,
        node_range,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_block_quote(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Check for nested blockquotes in children
    let mut children = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::BLOCK_QUOTE
            && let Some(block) = process_block_quote(source, child, anchors)
        {
            children.push(block);
        }
    }

    let id = find_anchor_for_range(anchors, &node_range);

    // If blockquote has nested blockquotes, all content belongs to the innermost level
    // so outer levels should have empty segments
    let (content, segments) = if children.is_empty() {
        // Leaf blockquote: extract segments from content (after "> " prefix)
        let prefix_len = text.find(|c: char| c != '>' && c != ' ').unwrap_or(0);
        let content_start = node_range.start + prefix_len;
        let content_end = if text.ends_with('\n') {
            node_range.end - 1
        } else {
            node_range.end
        };
        let segments = extract_segments(&node, source, content_start..content_end);
        (BlockContent::Leaf, segments)
    } else {
        // Nested blockquote: content belongs to children, no segments at this level
        (BlockContent::Children(children), vec![])
    };

    Some(Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range,
        segments,
        content,
    })
}

fn process_heading(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Count # for level
    let level = text.chars().take_while(|&c| c == '#').count() as u8;
    let prefix_len = level as usize + 1; // # + space

    // Content: after prefix, before trailing newline
    let content_start = node_range.start + prefix_len;
    let content_end = if text.ends_with('\n') {
        node_range.end - 1
    } else {
        node_range.end
    };

    let id = find_anchor_for_range(anchors, &node_range);
    let segments = extract_segments(&node, source, content_start..content_end);

    Some(Block {
        id,
        kind: BlockKind::Heading { level },
        node_range,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_fenced_code(source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Extract language from first line
    let first_line_end = text.find('\n').unwrap_or(text.len());
    let first_line = &text[..first_line_end];
    let language = first_line
        .trim_start_matches('`')
        .trim_start_matches('~')
        .trim();
    let language = if language.is_empty() {
        None
    } else {
        Some(language.to_string())
    };

    let id = find_anchor_for_range(anchors, &node_range);

    // Extract code content between opening and closing fences
    let segments = if let Some(first_newline) = text.find('\n') {
        let content_start = node_range.start + first_newline + 1;
        // Find last line (closing fence) by finding last newline before the closing fence.
        // We need to trim any trailing newline that comes AFTER the closing fence,
        // otherwise rfind finds that trailing newline instead of the one before the fence.
        let text_without_trailing = text.trim_end_matches('\n');
        let last_newline = text_without_trailing
            .rfind('\n')
            .unwrap_or(text_without_trailing.len());
        // Content ends at the last newline (before closing fence line)
        let content_end = node_range.start + last_newline;
        if content_start < content_end {
            let code_text = &source[content_start..content_end];
            vec![InlineSegment {
                kind: InlineNode::Text(code_text.to_string()),
                range: content_start..content_end,
            }]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Some(Block {
        id,
        kind: BlockKind::FencedCode { language },
        node_range,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_thematic_break(_source: &str, node: SyntaxNode, anchors: &[Anchor]) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());

    // Thematic breaks don't have their own anchors in the current model,
    // so we generate a fallback ID from the range
    let id = find_anchor_for_range(anchors, &node_range);

    Some(Block {
        id,
        kind: BlockKind::ThematicBreak,
        node_range,
        segments: vec![],
        content: BlockContent::Leaf,
    })
}

/// Extract list marker like "- " or "* " or "1. "
fn extract_list_marker(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let indent = line.len() - trimmed.len();
        line[..indent + 2].to_string()
    } else if let Some(dot_pos) = trimmed.find(". ") {
        let num_part = &trimmed[..dot_pos];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            let indent = line.len() - trimmed.len();
            line[..indent + dot_pos + 2].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

/// Intermediate inline info during extraction (position and inline node)
struct InlineInfo {
    range: Range<usize>,
    node: InlineNode,
}

/// Extract segments from a node, producing a list ready for UI rendering.
/// Handles inline formatting (emphasis, strong, code, links, etc.) and fills
/// gaps with Text segments so the entire content is covered.
fn extract_segments(
    node: &SyntaxNode,
    source: &str,
    content_range: Range<usize>,
) -> Vec<InlineSegment> {
    if content_range.is_empty() {
        return vec![];
    }

    // Collect inline elements with their ranges
    let inlines = collect_inlines(node, source);

    // Build segments, filling gaps with Text
    build_segments_with_gaps(&inlines, source, content_range)
}

/// Collect inline elements from a node, recursively extracting children for STRONG/EMPHASIS.
/// Hard breaks (trailing spaces + newline) are only detected in block types where they
/// are semantically meaningful (paragraphs, list items, blockquotes), not in headings.
fn collect_inlines(node: &SyntaxNode, source: &str) -> Vec<InlineInfo> {
    // Determine if hard breaks should be detected based on block context
    let detect_hard_breaks = matches!(
        node.kind(),
        SyntaxKind::PARAGRAPH | SyntaxKind::BLOCK_QUOTE | SyntaxKind::LIST_ITEM
    );

    let mut inlines = Vec::new();

    // Collect children to allow lookahead for hard break detection
    let children: Vec<_> = node.children_with_tokens().collect();

    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        let range: Range<usize> = {
            let r = child.text_range();
            (r.start().into())..(r.end().into())
        };
        let text = &source[range.clone()];

        let info: Option<InlineInfo> = match child {
            SyntaxElement::Token(token) => match token.kind() {
                // Detect hard break pattern: WHITESPACE (2+ trailing spaces) + NEWLINE
                // Only in contexts where hard breaks are semantically meaningful
                SyntaxKind::WHITESPACE if detect_hard_breaks && text.ends_with("  ") => {
                    // Check if next token is NEWLINE
                    if let Some(SyntaxElement::Token(next)) = children.get(i + 1) {
                        if next.kind() == SyntaxKind::NEWLINE {
                            // This is a hard break - combine both tokens
                            let next_range = next.text_range();
                            let combined_range = range.start..(next_range.end().into());
                            i += 1; // Skip the NEWLINE token
                            Some(InlineInfo {
                                range: combined_range,
                                node: InlineNode::HardBreak,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            },
            SyntaxElement::Node(child_node) => match child_node.kind() {
                SyntaxKind::EMPHASIS => {
                    // *text* or _text_ - skip marker on each side
                    let content_range = (range.start + 1)..(range.end - 1);
                    let children = extract_inline_children(child_node, source, content_range);
                    Some(InlineInfo {
                        range: range.clone(),
                        node: InlineNode::Emphasis(children),
                    })
                }
                SyntaxKind::STRONG => {
                    // **text** or __text__ - skip 2 markers on each side
                    let content_range = (range.start + 2)..(range.end - 2);
                    let children = extract_inline_children(child_node, source, content_range);
                    Some(InlineInfo {
                        range: range.clone(),
                        node: InlineNode::Strong(children),
                    })
                }
                SyntaxKind::CODE_SPAN => {
                    // `code` - skip backtick on each side
                    let content = (range.start + 1)..(range.end - 1);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        node: InlineNode::Code(content_text),
                    })
                }
                SyntaxKind::LINK => parse_link(text).map(|(link_text, url)| InlineInfo {
                    range: range.clone(),
                    node: InlineNode::Link {
                        text: link_text,
                        url,
                    },
                }),
                SyntaxKind::WIKILINK => parse_wikilink(text).map(|(target, alias)| InlineInfo {
                    range: range.clone(),
                    node: InlineNode::WikiLink { target, alias },
                }),
                SyntaxKind::IMAGE => parse_image(text).map(|(alt, url)| InlineInfo {
                    range: range.clone(),
                    node: InlineNode::Image { alt, url },
                }),
                SyntaxKind::STRIKETHROUGH => {
                    // ~~text~~ - skip 2 markers on each side
                    let content = (range.start + 2)..(range.end - 2);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        node: InlineNode::Strikethrough(content_text),
                    })
                }
                _ => None,
            },
        };

        if let Some(info) = info {
            inlines.push(info);
        }
        i += 1;
    }

    // Sort inlines by start position
    inlines.sort_by_key(|i| i.range.start);
    inlines
}

/// Extract children from an inline container node (STRONG/EMPHASIS).
/// Recursively collects nested inlines and fills gaps with Text nodes.
fn extract_inline_children(
    node: &SyntaxNode,
    source: &str,
    content_range: Range<usize>,
) -> Vec<InlineNode> {
    if content_range.is_empty() {
        return vec![];
    }

    // Collect nested inline elements
    let nested_inlines = collect_inlines(node, source);

    // Filter to inlines within content range and build with gap-filling
    let mut children = Vec::new();
    let mut cursor = content_range.start;

    for inline in &nested_inlines {
        // Skip inlines outside content range
        if inline.range.end <= content_range.start || inline.range.start >= content_range.end {
            continue;
        }

        // Add Text node for gap before this inline
        if inline.range.start > cursor {
            let text_end = inline.range.start.min(content_range.end);
            let text = &source[cursor..text_end];
            if !text.is_empty() {
                children.push(InlineNode::Text(text.to_string()));
            }
        }

        // Add the inline node
        children.push(inline.node.clone());

        cursor = inline.range.end.max(cursor);
    }

    // Add trailing Text node
    if cursor < content_range.end {
        let text = &source[cursor..content_range.end];
        if !text.is_empty() {
            children.push(InlineNode::Text(text.to_string()));
        }
    }

    // If no inlines found, entire content is plain text
    if children.is_empty() && !content_range.is_empty() {
        let text = &source[content_range.clone()];
        if !text.is_empty() {
            children.push(InlineNode::Text(text.to_string()));
        }
    }

    children
}

/// Convert text with newlines into Text segments with SoftBreak between lines.
/// Newlines are converted to SoftBreak to ensure consistent rendering across all block types.
fn text_to_segments_with_softbreaks(text: &str, start: usize) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut cursor = start;

    for (i, part) in text.split('\n').enumerate() {
        // Add SoftBreak between lines (not before first)
        if i > 0 {
            segments.push(InlineSegment {
                kind: InlineNode::SoftBreak,
                range: cursor..cursor, // Zero-length at newline position
            });
            cursor += 1; // Skip the newline character
        }

        if !part.is_empty() {
            let part_end = cursor + part.len();
            segments.push(InlineSegment {
                kind: InlineNode::Text(part.to_string()),
                range: cursor..part_end,
            });
            cursor = part_end;
        }
    }

    segments
}

/// Build InlineSegment list from collected inlines, filling gaps with Text segments.
fn build_segments_with_gaps(
    inlines: &[InlineInfo],
    source: &str,
    content_range: Range<usize>,
) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut cursor = content_range.start;

    for inline in inlines {
        // Skip inlines outside content range
        if inline.range.end <= content_range.start || inline.range.start >= content_range.end {
            continue;
        }

        // Add Text segment(s) for gap before this inline
        if inline.range.start > cursor {
            let text_end = inline.range.start.min(content_range.end);
            let text = &source[cursor..text_end];
            if !text.is_empty() {
                segments.extend(text_to_segments_with_softbreaks(text, cursor));
            }
        }

        // Add the inline segment
        segments.push(InlineSegment {
            kind: inline.node.clone(),
            range: inline.range.clone(),
        });

        cursor = inline.range.end.max(cursor);
    }

    // Add trailing Text segment(s)
    if cursor < content_range.end {
        let text = &source[cursor..content_range.end];
        if !text.is_empty() {
            segments.extend(text_to_segments_with_softbreaks(text, cursor));
        }
    }

    // If no inlines found, entire content is plain text
    if segments.is_empty() && !content_range.is_empty() {
        let text = &source[content_range.clone()];
        if !text.is_empty() {
            segments.extend(text_to_segments_with_softbreaks(text, content_range.start));
        }
    }

    segments
}

/// Parse [text](url) into (text, url) strings
fn parse_link(text: &str) -> Option<(String, String)> {
    let close_bracket = text.find(']')?;
    let open_paren = text[close_bracket..].find('(')? + close_bracket;
    let close_paren = text.rfind(')')?;

    let link_text = text[1..close_bracket].to_string();
    let url = text[open_paren + 1..close_paren].to_string();
    Some((link_text, url))
}

/// Parse [[target]] or [[target|alias]] into (target, Option<alias>) strings
fn parse_wikilink(text: &str) -> Option<(String, Option<String>)> {
    let inner = &text[2..text.len() - 2];
    if let Some(pipe_pos) = inner.find('|') {
        Some((
            inner[..pipe_pos].to_string(),
            Some(inner[pipe_pos + 1..].to_string()),
        ))
    } else {
        Some((inner.to_string(), None))
    }
}

/// Parse ![alt](url) into (alt, url) strings
fn parse_image(text: &str) -> Option<(String, String)> {
    let close_bracket = text.find(']')?;
    let open_paren = text[close_bracket..].find('(')? + close_bracket;
    let close_paren = text.rfind(')')?;

    let alt = text[2..close_bracket].to_string();
    let url = text[open_paren + 1..close_paren].to_string();
    Some((alt, url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::Document;

    // ============ Snapshot formatting (test-only) ============

    /// Format a snapshot as a readable string for snapshot testing.
    fn insta_format_snapshot(snapshot: &Snapshot) -> String {
        let mut result = String::new();
        for block in &snapshot.blocks {
            insta_format_block(&mut result, block, 0);
        }
        result
    }

    fn insta_format_block(out: &mut String, block: &Block, indent: usize) {
        use std::fmt::Write;

        let prefix = "  ".repeat(indent);

        // Block header
        writeln!(
            out,
            "{}{:?} [{}..{}]",
            prefix, block.kind, block.node_range.start, block.node_range.end
        )
        .unwrap();

        // Segments
        if !block.segments.is_empty() {
            writeln!(out, "{}  segments:", prefix).unwrap();
            for segment in &block.segments {
                insta_format_segment(out, segment, &prefix);
            }
        }

        // Content
        match &block.content {
            BlockContent::Leaf => {
                // Content available via segments
            }
            BlockContent::Children(children) => {
                writeln!(out, "{}  children:", prefix).unwrap();
                for child in children {
                    insta_format_block(out, child, indent + 2);
                }
            }
        }
    }

    fn insta_format_segment(out: &mut String, segment: &InlineSegment, prefix: &str) {
        use std::fmt::Write;

        let spaces = "    ";
        let range = &segment.range;
        match &segment.kind {
            InlineNode::Text(text) => {
                writeln!(
                    out,
                    "{}{}Text [{}..{}] {:?}",
                    prefix, spaces, range.start, range.end, text
                )
                .unwrap();
            }
            InlineNode::Strong(children) => {
                writeln!(
                    out,
                    "{}{}Strong [{}..{}]",
                    prefix, spaces, range.start, range.end
                )
                .unwrap();
                for child in children {
                    insta_format_inline_node(out, child, prefix, 6);
                }
            }
            InlineNode::Emphasis(children) => {
                writeln!(
                    out,
                    "{}{}Emphasis [{}..{}]",
                    prefix, spaces, range.start, range.end
                )
                .unwrap();
                for child in children {
                    insta_format_inline_node(out, child, prefix, 6);
                }
            }
            InlineNode::Code(text) => {
                writeln!(
                    out,
                    "{}{}Code [{}..{}] {:?}",
                    prefix, spaces, range.start, range.end, text
                )
                .unwrap();
            }
            InlineNode::Strikethrough(text) => {
                writeln!(
                    out,
                    "{}{}Strikethrough [{}..{}] {:?}",
                    prefix, spaces, range.start, range.end, text
                )
                .unwrap();
            }
            InlineNode::WikiLink { target, alias } => {
                if let Some(alias) = alias {
                    writeln!(
                        out,
                        "{}{}WikiLink [{}..{}] target:{:?} alias:{:?}",
                        prefix, spaces, range.start, range.end, target, alias
                    )
                    .unwrap();
                } else {
                    writeln!(
                        out,
                        "{}{}WikiLink [{}..{}] target:{:?}",
                        prefix, spaces, range.start, range.end, target
                    )
                    .unwrap();
                }
            }
            InlineNode::Link { text, url } => {
                writeln!(
                    out,
                    "{}{}Link [{}..{}] text:{:?} url:{:?}",
                    prefix, spaces, range.start, range.end, text, url
                )
                .unwrap();
            }
            InlineNode::Image { alt, url } => {
                writeln!(
                    out,
                    "{}{}Image [{}..{}] alt:{:?} url:{:?}",
                    prefix, spaces, range.start, range.end, alt, url
                )
                .unwrap();
            }
            InlineNode::HardBreak => {
                writeln!(
                    out,
                    "{}{}HardBreak [{}..{}]",
                    prefix, spaces, range.start, range.end
                )
                .unwrap();
            }
            InlineNode::SoftBreak => {
                writeln!(
                    out,
                    "{}{}SoftBreak [{}..{}]",
                    prefix, spaces, range.start, range.end
                )
                .unwrap();
            }
        }
    }

    fn insta_format_inline_node(out: &mut String, node: &InlineNode, prefix: &str, indent: usize) {
        use std::fmt::Write;

        let spaces = " ".repeat(indent);
        match node {
            InlineNode::Text(text) => {
                writeln!(out, "{}{}Text {:?}", prefix, spaces, text).unwrap();
            }
            InlineNode::Strong(children) => {
                writeln!(out, "{}{}Strong", prefix, spaces).unwrap();
                for child in children {
                    insta_format_inline_node(out, child, prefix, indent + 2);
                }
            }
            InlineNode::Emphasis(children) => {
                writeln!(out, "{}{}Emphasis", prefix, spaces).unwrap();
                for child in children {
                    insta_format_inline_node(out, child, prefix, indent + 2);
                }
            }
            InlineNode::Code(text) => {
                writeln!(out, "{}{}Code {:?}", prefix, spaces, text).unwrap();
            }
            InlineNode::Strikethrough(text) => {
                writeln!(out, "{}{}Strikethrough {:?}", prefix, spaces, text).unwrap();
            }
            InlineNode::WikiLink { target, alias } => {
                if let Some(alias) = alias {
                    writeln!(
                        out,
                        "{}{}WikiLink target:{:?} alias:{:?}",
                        prefix, spaces, target, alias
                    )
                    .unwrap();
                } else {
                    writeln!(out, "{}{}WikiLink target:{:?}", prefix, spaces, target).unwrap();
                }
            }
            InlineNode::Link { text, url } => {
                writeln!(
                    out,
                    "{}{}Link text:{:?} url:{:?}",
                    prefix, spaces, text, url
                )
                .unwrap();
            }
            InlineNode::Image { alt, url } => {
                writeln!(out, "{}{}Image alt:{:?} url:{:?}", prefix, spaces, alt, url).unwrap();
            }
            InlineNode::HardBreak => {
                writeln!(out, "{}{}HardBreak", prefix, spaces).unwrap();
            }
            InlineNode::SoftBreak => {
                writeln!(out, "{}{}SoftBreak", prefix, spaces).unwrap();
            }
        }
    }

    // ============ Snapshot tests ============

    /// Run a snapshot test for a given .md file.
    /// Called by generated test functions (see build.rs).
    /// `rel_path` is relative to tests/snapshots/, e.g., "blocks/heading_h1".
    fn snapshot_test(rel_path: &str) {
        // Shared input files at workspace root
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let input_path = workspace_root
            .join("tests/snapshots")
            .join(format!("{rel_path}.md"));
        let input = std::fs::read_to_string(&input_path).unwrap();

        let doc = Document::from_bytes(input.as_bytes()).unwrap();

        let snapshot = create_snapshot(&doc);
        let formatted = insta_format_snapshot(&snapshot);

        // Snapshot output goes in crate-local directory, mirroring input structure
        let crate_snap_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/editing/snapshots");
        let rel_dir = std::path::Path::new(rel_path).parent();
        let snapshot_dir = match rel_dir {
            Some(d) if !d.as_os_str().is_empty() => crate_snap_dir.join(d),
            _ => crate_snap_dir.clone(),
        };
        let snapshot_name = std::path::Path::new(rel_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path(snapshot_dir);
        settings.bind(|| {
            insta::assert_snapshot!(snapshot_name, formatted);
        });

        // All parses must preserve input bytes (lossless)
        assert_eq!(doc.text(), input, "{rel_path}: roundtrip failed");
    }

    // Generated by build.rs - one test per .md file in snapshots/
    // All parsing → snapshot behavior is verified by snapshot tests.
    // Edge cases are in tests/snapshots/malformed/.
    include!(concat!(env!("OUT_DIR"), "/snapshot_v2_tests.rs"));
}
