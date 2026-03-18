//! # Snapshot V2: Tree-Structured Document Projection
//!
//! This module provides ergonomic primitives for the editor UI by exposing
//! the document structure as a tree with per-line range information.
//!
//! ## Design Goals (from ADR-0012)
//!
//! - Keep all "wtf is this string" complexity in the snapshot layer
//! - Editor gets clean primitives without understanding markdown syntax
//! - Support both content editing and full-line editing modes

use std::ops::Range;

use markdown_neuraxis_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, parse};

use crate::editing::{Anchor, AnchorId};

/// Per-line range information for multi-line blocks
#[derive(Debug, Clone, PartialEq)]
pub struct LineInfo {
    /// Full line range including all prefixes
    pub full: Range<usize>,
    /// Prefix range (indent + markers like "> " or "- ")
    pub prefix: Range<usize>,
    /// Content range (actual text after prefix)
    pub content: Range<usize>,
}

impl LineInfo {
    /// Create a new LineInfo, clamping content range to be valid.
    /// If content_start > content_end (e.g., whitespace-only line), returns empty range.
    pub fn new(
        full: Range<usize>,
        prefix: Range<usize>,
        content_start: usize,
        content_end: usize,
    ) -> Self {
        Self {
            full,
            prefix,
            content: content_start.min(content_end)..content_end,
        }
    }
}

/// Content of a block: either leaf (no children) or nested children
#[derive(Debug, Clone, PartialEq)]
pub enum BlockContent {
    /// Leaf block - text extracted via LineInfo ranges
    Leaf,
    /// Container block with child blocks
    Children(Vec<Block>),
}

/// A segment of inline content ready for UI rendering.
/// UIs can iterate directly over segments to render all text.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSegment {
    /// The kind of segment with its content
    pub kind: SegmentKind,
    /// Byte range in source (for verification/debugging)
    pub range: Range<usize>,
}

/// The kind of inline segment
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentKind {
    /// Plain text content
    Text(String),
    /// Strong emphasis (**text**)
    Strong(String),
    /// Emphasis (*text*)
    Emphasis(String),
    /// Inline code (`code`)
    Code(String),
    /// Strikethrough (~~text~~)
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
    /// Hard line break
    HardBreak,
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
    /// Top-level ancestor's span (for "edit full block" behavior)
    pub root_range: Range<usize>,
    /// Per-line breakdown with prefix/content ranges
    pub lines: Vec<LineInfo>,
    /// Flat list of inline segments for UI rendering.
    /// Includes Text segments for gaps between formatted inlines.
    pub segments: Vec<InlineSegment>,
    /// Block content (text or children)
    pub content: BlockContent,
}

/// Tree-structured document snapshot
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Root-level blocks
    pub blocks: Vec<Block>,
}

/// Format a snapshot as a readable string for snapshot testing
pub fn format_snapshot(snapshot: &Snapshot, source: &str) -> String {
    let mut result = String::new();
    for block in &snapshot.blocks {
        format_block(&mut result, block, source, 0);
    }
    result
}

fn format_block(out: &mut String, block: &Block, source: &str, indent: usize) {
    use std::fmt::Write;

    let prefix = "  ".repeat(indent);

    // Block header
    writeln!(
        out,
        "{}{:?} [{}..{}]",
        prefix, block.kind, block.node_range.start, block.node_range.end
    )
    .unwrap();

    // Root range (if different)
    if block.root_range != block.node_range {
        writeln!(out, "{}  root_range: {:?}", prefix, block.root_range).unwrap();
    }

    // Lines
    if !block.lines.is_empty() {
        writeln!(out, "{}  lines:", prefix).unwrap();
        for line in &block.lines {
            let prefix_text = &source[line.prefix.clone()];
            let content_text = &source[line.content.clone()];
            writeln!(
                out,
                "{}    full:{:?} prefix:{:?}{:?} content:{:?}{:?}",
                prefix,
                line.full,
                line.prefix,
                prefix_text.replace('\n', "\\n"),
                line.content,
                content_text.replace('\n', "\\n")
            )
            .unwrap();
        }
    }

    // Segments
    if !block.segments.is_empty() {
        writeln!(out, "{}  segments:", prefix).unwrap();
        for segment in &block.segments {
            format_segment(out, segment, &prefix);
        }
    }

    // Content
    match &block.content {
        BlockContent::Leaf => {
            // Text extracted via lines[].content ranges - no separate field needed
        }
        BlockContent::Children(children) => {
            writeln!(out, "{}  children:", prefix).unwrap();
            for child in children {
                format_block(out, child, source, indent + 2);
            }
        }
    }
}

fn format_segment(out: &mut String, segment: &InlineSegment, prefix: &str) {
    use std::fmt::Write;

    let range = &segment.range;
    match &segment.kind {
        SegmentKind::Text(text) => {
            writeln!(
                out,
                "{}    Text [{}..{}] {:?}",
                prefix, range.start, range.end, text
            )
            .unwrap();
        }
        SegmentKind::Strong(text) => {
            writeln!(
                out,
                "{}    Strong [{}..{}] {:?}",
                prefix, range.start, range.end, text
            )
            .unwrap();
        }
        SegmentKind::Emphasis(text) => {
            writeln!(
                out,
                "{}    Emphasis [{}..{}] {:?}",
                prefix, range.start, range.end, text
            )
            .unwrap();
        }
        SegmentKind::Code(text) => {
            writeln!(
                out,
                "{}    Code [{}..{}] {:?}",
                prefix, range.start, range.end, text
            )
            .unwrap();
        }
        SegmentKind::Strikethrough(text) => {
            writeln!(
                out,
                "{}    Strikethrough [{}..{}] {:?}",
                prefix, range.start, range.end, text
            )
            .unwrap();
        }
        SegmentKind::WikiLink { target, alias } => {
            if let Some(alias) = alias {
                writeln!(
                    out,
                    "{}    WikiLink [{}..{}] target:{:?} alias:{:?}",
                    prefix, range.start, range.end, target, alias
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}    WikiLink [{}..{}] target:{:?}",
                    prefix, range.start, range.end, target
                )
                .unwrap();
            }
        }
        SegmentKind::Link { text, url } => {
            writeln!(
                out,
                "{}    Link [{}..{}] text:{:?} url:{:?}",
                prefix, range.start, range.end, text, url
            )
            .unwrap();
        }
        SegmentKind::Image { alt, url } => {
            writeln!(
                out,
                "{}    Image [{}..{}] alt:{:?} url:{:?}",
                prefix, range.start, range.end, alt, url
            )
            .unwrap();
        }
        SegmentKind::HardBreak => {
            writeln!(
                out,
                "{}    HardBreak [{}..{}]",
                prefix, range.start, range.end
            )
            .unwrap();
        }
    }
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
        if let Some(block) = process_node(&source, child, None, anchors) {
            blocks.push(block);
        }
    }

    Snapshot { blocks }
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
fn process_node(
    source: &str,
    node: SyntaxNode,
    root_range: Option<Range<usize>>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range = (text_range.start().into())..(text_range.end().into());
    let root_range = root_range.unwrap_or_else(|| node_range.clone());

    match node.kind() {
        SyntaxKind::ORDERED_LIST => process_list(source, node, root_range, anchors, true),
        SyntaxKind::UNORDERED_LIST => process_list(source, node, root_range, anchors, false),
        SyntaxKind::LIST_ITEM => process_list_item(source, node, root_range, anchors),
        SyntaxKind::PARAGRAPH => process_paragraph(source, node, root_range, anchors),
        SyntaxKind::BLOCK_QUOTE => process_block_quote(source, node, root_range, anchors),
        SyntaxKind::HEADING => process_heading(source, node, root_range, anchors),
        SyntaxKind::FENCED_CODE => process_fenced_code(source, node, root_range, anchors),
        SyntaxKind::THEMATIC_BREAK => process_thematic_break(source, node, root_range, anchors),
        _ => None, // Skip unknown node types
    }
}

fn process_list(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
    ordered: bool,
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let mut children = Vec::new();

    for child in node.children() {
        if let Some(block) = process_node(source, child, Some(root_range.clone()), anchors) {
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
        node_range: node_range.clone(),
        root_range: node_range,
        lines: vec![],    // List container has no lines of its own
        segments: vec![], // List containers have no segments
        content: BlockContent::Children(children),
    })
}

fn process_list_item(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Extract marker from first line
    let first_line_content_end = text.find('\n').unwrap_or(text.len());
    let first_line = &text[..first_line_content_end];
    let marker = extract_list_marker(first_line);
    let marker_len = marker.len();

    // First line info - full includes the newline if present
    let line_start = node_range.start;
    let line_end_with_newline = if first_line_content_end < text.len()
        && text.as_bytes().get(first_line_content_end) == Some(&b'\n')
    {
        node_range.start + first_line_content_end + 1
    } else {
        node_range.start + first_line_content_end
    };
    let content_start = line_start + marker_len;
    let content_end = node_range.start + first_line_content_end;

    let first_line_info = LineInfo::new(
        line_start..line_end_with_newline,
        line_start..(line_start + marker_len),
        content_start,
        content_end,
    );

    // Process children (nested content)
    // Note: Skip PARAGRAPH children since LIST_ITEM already extracts its text from lines.
    // We only want to process nested structural elements like LIST, BLOCK_QUOTE, FENCED_CODE.
    let mut children = Vec::new();
    for child in node.children() {
        // Skip PARAGRAPH inside list items - the list item already extracted its text
        if child.kind() == SyntaxKind::PARAGRAPH {
            continue;
        }
        if let Some(block) = process_node(source, child, Some(root_range.clone()), anchors) {
            children.push(block);
        }
    }

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    // Look up anchor ID for this list item
    let id = find_anchor_for_range(anchors, &node_range);

    // Extract segments from the list item's content
    // We look in the PARAGRAPH child (if present) since that's where inlines live
    let lines_vec = vec![first_line_info];
    let content_range = compute_content_range(&lines_vec);
    let segments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::PARAGRAPH)
        .map(|para| extract_segments(&para, source, content_range.clone()))
        .unwrap_or_else(|| {
            // No paragraph child, create a single Text segment for the content
            if !content_range.is_empty() {
                let text = &source[content_range.clone()];
                if !text.is_empty() {
                    return vec![InlineSegment {
                        kind: SegmentKind::Text(text.to_string()),
                        range: content_range.clone(),
                    }];
                }
            }
            vec![]
        });

    Some(Block {
        id,
        kind: BlockKind::ListItem { marker },
        node_range,
        root_range,
        lines: lines_vec,
        segments,
        content,
    })
}

fn process_paragraph(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Split into lines and create LineInfo for each
    let mut lines = Vec::new();
    let mut pos = node_range.start;

    for line in text.split_inclusive('\n') {
        let line_end = pos + line.len();
        let trimmed_start = line.len() - line.trim_start().len();
        let content_end = if line.ends_with('\n') {
            line_end - 1
        } else {
            line_end
        };
        let content_start = pos + trimmed_start;

        lines.push(LineInfo::new(
            pos..line_end,
            pos..(pos + trimmed_start),
            content_start,
            content_end,
        ));

        pos = line_end;
    }

    // Extract segments from inline content
    let content_range = compute_content_range(&lines);
    let segments = extract_segments(&node, source, content_range);

    // Paragraphs don't have their own anchors in the current model,
    // so we generate a fallback ID from the range
    let id = find_anchor_for_range(anchors, &node_range);

    Some(Block {
        id,
        kind: BlockKind::Paragraph,
        node_range,
        root_range,
        lines,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_block_quote(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Split into lines
    let mut lines = Vec::new();
    let mut children = Vec::new();
    let mut pos = node_range.start;
    let mut is_first_line = true;

    for line in text.split_inclusive('\n') {
        let line_end = pos + line.len();

        // For the first line, find the actual line start (may include indentation not in node)
        let actual_line_start = if is_first_line {
            find_line_start(source, pos)
        } else {
            pos
        };
        is_first_line = false;

        // Get the full line text including any leading indentation
        let full_line = &source[actual_line_start..line_end];

        // Find prefix (leading whitespace + > markers)
        let prefix_end = find_blockquote_prefix_end(full_line);
        let content_end = if full_line.ends_with('\n') {
            line_end - 1
        } else {
            line_end
        };
        let content_start = actual_line_start + prefix_end;

        lines.push(LineInfo::new(
            actual_line_start..line_end,
            actual_line_start..(actual_line_start + prefix_end),
            content_start,
            content_end,
        ));

        pos = line_end;
    }

    // Check for nested blockquotes in children
    for child in node.children() {
        if child.kind() == SyntaxKind::BLOCK_QUOTE
            && let Some(block) = process_block_quote(source, child, root_range.clone(), anchors)
        {
            children.push(block);
        }
    }

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    // Blockquotes don't have their own anchors in the current model,
    // so we generate a fallback ID from the range
    let id = find_anchor_for_range(anchors, &node_range);

    // Extract segments from inline content
    let content_range = compute_content_range(&lines);
    let segments = extract_segments(&node, source, content_range);

    Some(Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range,
        root_range,
        lines,
        segments,
        content,
    })
}

fn process_heading(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());
    let text = &source[node_range.clone()];

    // Count # for level
    let level = text.chars().take_while(|&c| c == '#').count() as u8;
    let prefix_end = level as usize + 1; // # + space

    let content_end = if text.ends_with('\n') {
        node_range.end - 1
    } else {
        node_range.end
    };

    let line_info = LineInfo::new(
        node_range.clone(),
        node_range.start..(node_range.start + prefix_end),
        node_range.start + prefix_end,
        content_end,
    );

    // Look up anchor ID for this heading
    let id = find_anchor_for_range(anchors, &node_range);

    // Extract segments from inline content
    let lines_vec = vec![line_info];
    let content_range = compute_content_range(&lines_vec);
    let segments = extract_segments(&node, source, content_range);

    Some(Block {
        id,
        kind: BlockKind::Heading { level },
        node_range,
        root_range,
        lines: lines_vec,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_fenced_code(
    source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
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

    // Create line infos for all lines
    let mut lines = Vec::new();
    let mut pos = node_range.start;

    for line in text.split_inclusive('\n') {
        let line_end = pos + line.len();
        let content_end = if line.ends_with('\n') {
            line_end - 1
        } else {
            line_end
        };

        lines.push(LineInfo::new(
            pos..line_end,
            pos..pos, // Code blocks have no prefix
            pos,
            content_end,
        ));

        pos = line_end;
    }

    // Look up anchor ID for this fenced code block
    let id = find_anchor_for_range(anchors, &node_range);

    // Generate a single Text segment with the code content (middle lines, excluding fences)
    let segments = if lines.len() > 2 {
        // Skip first line (opening fence) and last line (closing fence)
        let content_start = lines[1].content.start;
        let content_end = lines[lines.len() - 2].content.end;
        let code_text = &source[content_start..content_end];
        if !code_text.is_empty() {
            vec![InlineSegment {
                kind: SegmentKind::Text(code_text.to_string()),
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
        root_range,
        lines,
        segments,
        content: BlockContent::Leaf,
    })
}

fn process_thematic_break(
    _source: &str,
    node: SyntaxNode,
    root_range: Range<usize>,
    anchors: &[Anchor],
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());

    // Thematic breaks don't have their own anchors in the current model,
    // so we generate a fallback ID from the range
    let id = find_anchor_for_range(anchors, &node_range);

    Some(Block {
        id,
        kind: BlockKind::ThematicBreak,
        node_range,
        root_range,
        lines: vec![],
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

/// Find where the blockquote prefix ends (whitespace + > markers + space)
fn find_blockquote_prefix_end(line: &str) -> usize {
    let mut pos = 0;
    let chars: Vec<char> = line.chars().collect();

    // Skip leading whitespace
    while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') {
        pos += 1;
    }

    // Skip > markers and spaces
    while pos < chars.len() && chars[pos] == '>' {
        pos += 1;
        // Skip optional space after >
        if pos < chars.len() && chars[pos] == ' ' {
            pos += 1;
        }
    }

    pos
}

/// Find the actual start of the line containing `pos` in the source.
/// Scans backwards from `pos` to find the previous newline (or start of document).
fn find_line_start(source: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    // Scan backwards to find previous newline
    let bytes = source.as_bytes();
    let mut i = pos - 1;
    while i > 0 && bytes[i] != b'\n' {
        i -= 1;
    }
    if bytes[i] == b'\n' {
        i + 1 // Position after the newline
    } else {
        0 // Start of document
    }
}

/// Intermediate inline info during extraction (position and how to create segment)
struct InlineInfo {
    range: Range<usize>,
    make_segment: Box<dyn Fn(&str) -> SegmentKind>,
}

/// Extract segments from a node, producing a flat list ready for UI rendering.
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

    // Collect inline elements with their ranges and segment constructors
    let mut inlines: Vec<InlineInfo> = Vec::new();

    for child in node.children_with_tokens() {
        let range: Range<usize> = {
            let r = child.text_range();
            (r.start().into())..(r.end().into())
        };
        let text = &source[range.clone()];

        let info: Option<InlineInfo> = match &child {
            SyntaxElement::Token(token) => match token.kind() {
                SyntaxKind::HARD_BREAK => Some(InlineInfo {
                    range: range.clone(),
                    make_segment: Box::new(|_| SegmentKind::HardBreak),
                }),
                _ => None,
            },
            SyntaxElement::Node(node) => match node.kind() {
                SyntaxKind::EMPHASIS => {
                    // *text* or _text_ - skip marker on each side
                    let content = (range.start + 1)..(range.end - 1);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        make_segment: Box::new(move |_| {
                            SegmentKind::Emphasis(content_text.clone())
                        }),
                    })
                }
                SyntaxKind::STRONG => {
                    // **text** or __text__ - skip 2 markers on each side
                    let content = (range.start + 2)..(range.end - 2);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        make_segment: Box::new(move |_| SegmentKind::Strong(content_text.clone())),
                    })
                }
                SyntaxKind::CODE_SPAN => {
                    // `code` - skip backtick on each side
                    let content = (range.start + 1)..(range.end - 1);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        make_segment: Box::new(move |_| SegmentKind::Code(content_text.clone())),
                    })
                }
                SyntaxKind::LINK => parse_link(text).map(|(link_text, url)| InlineInfo {
                    range: range.clone(),
                    make_segment: Box::new(move |_| SegmentKind::Link {
                        text: link_text.clone(),
                        url: url.clone(),
                    }),
                }),
                SyntaxKind::WIKILINK => parse_wikilink(text).map(|(target, alias)| InlineInfo {
                    range: range.clone(),
                    make_segment: Box::new(move |_| SegmentKind::WikiLink {
                        target: target.clone(),
                        alias: alias.clone(),
                    }),
                }),
                SyntaxKind::IMAGE => parse_image(text).map(|(alt, url)| InlineInfo {
                    range: range.clone(),
                    make_segment: Box::new(move |_| SegmentKind::Image {
                        alt: alt.clone(),
                        url: url.clone(),
                    }),
                }),
                SyntaxKind::STRIKETHROUGH => {
                    // ~~text~~ - skip 2 markers on each side
                    let content = (range.start + 2)..(range.end - 2);
                    let content_text = source[content].to_string();
                    Some(InlineInfo {
                        range: range.clone(),
                        make_segment: Box::new(move |_| {
                            SegmentKind::Strikethrough(content_text.clone())
                        }),
                    })
                }
                _ => None,
            },
        };

        if let Some(info) = info {
            inlines.push(info);
        }
    }

    // Sort inlines by start position
    inlines.sort_by_key(|i| i.range.start);

    // Build segments, filling gaps with Text
    let mut segments = Vec::new();
    let mut cursor = content_range.start;

    for inline in &inlines {
        // Skip inlines outside content range
        if inline.range.end <= content_range.start || inline.range.start >= content_range.end {
            continue;
        }

        // Add Text segment for gap before this inline
        if inline.range.start > cursor {
            let text_end = inline.range.start.min(content_range.end);
            let text = &source[cursor..text_end];
            if !text.is_empty() {
                segments.push(InlineSegment {
                    kind: SegmentKind::Text(text.to_string()),
                    range: cursor..text_end,
                });
            }
        }

        // Add the inline segment
        segments.push(InlineSegment {
            kind: (inline.make_segment)(source),
            range: inline.range.clone(),
        });

        cursor = inline.range.end.max(cursor);
    }

    // Add trailing Text segment
    if cursor < content_range.end {
        let text = &source[cursor..content_range.end];
        if !text.is_empty() {
            segments.push(InlineSegment {
                kind: SegmentKind::Text(text.to_string()),
                range: cursor..content_range.end,
            });
        }
    }

    // If no inlines found, entire content is plain text
    if segments.is_empty() && !content_range.is_empty() {
        let text = &source[content_range.clone()];
        if !text.is_empty() {
            segments.push(InlineSegment {
                kind: SegmentKind::Text(text.to_string()),
                range: content_range,
            });
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

/// Helper to compute content range from lines for segment generation.
/// For blocks with multiple lines, joins them as a single range.
fn compute_content_range(lines: &[LineInfo]) -> Range<usize> {
    if lines.is_empty() {
        return 0..0;
    }
    let start = lines.first().unwrap().content.start;
    let end = lines.last().unwrap().content.end;
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::Document;

    /// Run a snapshot test for a given .md file.
    /// Called by generated test functions (see build.rs).
    fn snapshot_test(name: &str) {
        let snapshot_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/editing/snapshots");
        let input_path = snapshot_dir.join(format!("{name}.md"));
        let input = std::fs::read_to_string(&input_path).unwrap();

        let mut doc = Document::from_bytes(input.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = create_snapshot(&doc);
        let formatted = format_snapshot(&snapshot, &input);

        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path(&snapshot_dir);
        settings.bind(|| {
            insta::assert_snapshot!(name, formatted);
        });
    }

    // Generated by build.rs - one test per .md file in snapshots/
    include!(concat!(env!("OUT_DIR"), "/snapshot_v2_tests.rs"));

    // Behavioral contract tests that must hold regardless of parser backend.

    #[test]
    fn test_snapshot_empty_document() {
        let doc = Document::from_bytes(b"").unwrap();
        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 0);
    }

    #[test]
    fn test_snapshot_simple_heading() {
        let mut doc = Document::from_bytes(b"# Hello World").unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::Heading { level: 1 });
        assert_eq!(block.node_range, 0..13);
        // Content should be "Hello World" (after "# ")
        assert_eq!(block.lines.len(), 1);
        assert_eq!(block.lines[0].content, 2..13);
    }

    #[test]
    fn test_snapshot_multiple_headings() {
        let text = "# Heading 1\n\n## Heading 2\n\n### Heading 3";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have 3 heading blocks
        let headings: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, BlockKind::Heading { .. }))
            .collect();
        assert_eq!(headings.len(), 3);

        assert_eq!(headings[0].kind, BlockKind::Heading { level: 1 });
        assert_eq!(headings[1].kind, BlockKind::Heading { level: 2 });
        assert_eq!(headings[2].kind, BlockKind::Heading { level: 3 });
    }

    #[test]
    fn test_snapshot_nested_list_structure() {
        let mut doc = Document::from_bytes(
            b"- parent item\n  - child item 1\n  - child item 2\n    - grandchild item\n",
        )
        .unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have a single top-level List block
        assert_eq!(snapshot.blocks.len(), 1);
        let list = &snapshot.blocks[0];
        assert_eq!(list.kind, BlockKind::List { ordered: false });

        // List should have children (the list items)
        if let BlockContent::Children(items) = &list.content {
            // Count total list items recursively (including nested items)
            fn count_items(blocks: &[Block]) -> usize {
                let mut count = 0;
                for block in blocks {
                    if matches!(block.kind, BlockKind::ListItem { .. }) {
                        count += 1;
                    }
                    if let BlockContent::Children(children) = &block.content {
                        count += count_items(children);
                    }
                }
                count
            }
            let total_items = count_items(items);
            assert_eq!(total_items, 4, "Should have 4 list items total");
        } else {
            panic!("List should have children");
        }
    }

    #[test]
    fn test_nested_list_items_have_unique_anchor_ids() {
        let mut doc = Document::from_bytes(
            b"- parent item\n  - child item 1\n  - child item 2\n    - grandchild item\n",
        )
        .unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Collect all anchor IDs recursively
        fn collect_ids(blocks: &[Block], ids: &mut std::collections::HashSet<AnchorId>) {
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. }) {
                    ids.insert(block.id);
                }
                if let BlockContent::Children(children) = &block.content {
                    collect_ids(children, ids);
                }
            }
        }

        let mut ids = std::collections::HashSet::new();
        collect_ids(&snapshot.blocks, &mut ids);

        // Should have 4 unique anchor IDs (one per list item)
        // Now that anchor wiring is complete, we can properly validate uniqueness
        assert_eq!(
            ids.len(),
            4,
            "Should have exactly 4 unique IDs for 4 list items"
        );

        // Also verify that none of them are AnchorId(0) - the old placeholder
        for id in &ids {
            assert_ne!(
                *id,
                AnchorId(0),
                "No list item should have AnchorId(0) placeholder"
            );
        }
    }

    #[test]
    fn test_snapshot_code_fence_language() {
        let text = "```rust\nfn main() {}\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(
            block.kind,
            BlockKind::FencedCode {
                language: Some("rust".to_string())
            }
        );
    }

    #[test]
    fn test_snapshot_code_fence_no_language() {
        let text = "```\nplain code\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::FencedCode { language: None });
    }

    #[test]
    fn test_snapshot_range_validity() {
        let text = "# Heading\n\n- Item 1\n- Item 2\n\nParagraph text";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // All ranges should be valid
        fn check_ranges(blocks: &[Block], doc_len: usize) {
            for block in blocks {
                assert!(
                    block.node_range.end <= doc_len,
                    "node_range {:?} exceeds document length {}",
                    block.node_range,
                    doc_len
                );
                assert!(
                    block.root_range.end <= doc_len,
                    "root_range {:?} exceeds document length {}",
                    block.root_range,
                    doc_len
                );
                for line in &block.lines {
                    assert!(
                        line.full.end <= doc_len,
                        "line.full {:?} exceeds document length {}",
                        line.full,
                        doc_len
                    );
                    assert!(
                        line.prefix.end <= line.full.end,
                        "line.prefix {:?} exceeds line.full {:?}",
                        line.prefix,
                        line.full
                    );
                    assert!(
                        line.content.end <= line.full.end,
                        "line.content {:?} exceeds line.full {:?}",
                        line.content,
                        line.full
                    );
                }
                if let BlockContent::Children(children) = &block.content {
                    check_ranges(children, doc_len);
                }
            }
        }

        check_ranges(&snapshot.blocks, text.len());
    }

    #[test]
    fn test_wikilink_inline_parsing() {
        let text = "Check out [[Page Name]] for details";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have a paragraph with wiki-link segment
        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::Paragraph);

        // Find wiki-link segment
        let wikilinks: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1, "Should have 1 wiki-link");

        if let SegmentKind::WikiLink { target, alias } = &wikilinks[0].kind {
            assert_eq!(target, "Page Name");
            assert!(alias.is_none());
        }
    }

    #[test]
    fn test_wikilink_with_alias() {
        let text = "See [[target|display text]] here";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let wikilinks: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1);

        if let SegmentKind::WikiLink { target, alias } = &wikilinks[0].kind {
            assert_eq!(target, "target");
            assert!(alias.is_some());
            assert_eq!(alias.as_ref().unwrap(), "display text");
        }
    }

    #[test]
    fn test_multiple_wikilinks() {
        let text = "See [[First]] and [[Second]] and [[Third]]";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let wikilinks: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 3, "Should have 3 wiki-links");
    }

    #[test]
    fn test_link_inline_parsing() {
        let text = "Click [here](https://example.com) for info";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let links: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::Link { .. }))
            .collect();
        assert_eq!(links.len(), 1, "Should have 1 link");

        if let SegmentKind::Link {
            text: link_text,
            url,
        } = &links[0].kind
        {
            assert_eq!(link_text, "here");
            assert_eq!(url, "https://example.com");
        }
    }

    #[test]
    fn test_emphasis_inline_parsing() {
        let text = "This is *emphasized* text";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let emphasis: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::Emphasis(_)))
            .collect();
        assert_eq!(emphasis.len(), 1, "Should have 1 emphasis");

        if let SegmentKind::Emphasis(content) = &emphasis[0].kind {
            assert_eq!(content, "emphasized");
        }
    }

    #[test]
    fn test_strong_inline_parsing() {
        let text = "This is **strong** text";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let strong: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::Strong(_)))
            .collect();
        assert_eq!(strong.len(), 1, "Should have 1 strong");

        if let SegmentKind::Strong(content) = &strong[0].kind {
            assert_eq!(content, "strong");
        }
    }

    #[test]
    fn test_code_span_inline_parsing() {
        let text = "Use `code` inline";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let block = &snapshot.blocks[0];
        let code: Vec<_> = block
            .segments
            .iter()
            .filter(|s| matches!(s.kind, SegmentKind::Code(_)))
            .collect();
        assert_eq!(code.len(), 1, "Should have 1 code span");

        if let SegmentKind::Code(content) = &code[0].kind {
            assert_eq!(content, "code");
        }
    }

    #[test]
    fn test_blockquote_content() {
        let text = "> This is a quote";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::BlockQuote);

        // Content should be stripped of > marker
        assert_eq!(block.lines.len(), 1);
        let content_text = &text[block.lines[0].content.clone()];
        assert_eq!(content_text, "This is a quote");
    }

    #[test]
    fn test_list_item_markers() {
        let text = "- dash item\n* asterisk item\n+ plus item";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Collect all list items
        fn collect_markers(blocks: &[Block]) -> Vec<String> {
            let mut markers = Vec::new();
            for block in blocks {
                if let BlockKind::ListItem { marker } = &block.kind {
                    markers.push(marker.clone());
                }
                if let BlockContent::Children(children) = &block.content {
                    markers.extend(collect_markers(children));
                }
            }
            markers
        }

        let markers = collect_markers(&snapshot.blocks);
        assert_eq!(markers.len(), 3);
        assert!(markers[0].contains('-'));
        assert!(markers[1].contains('*'));
        assert!(markers[2].contains('+'));
    }

    #[test]
    fn test_thematic_break() {
        let text = "Above\n\n---\n\nBelow";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let breaks: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, BlockKind::ThematicBreak))
            .collect();
        assert_eq!(breaks.len(), 1, "Should have 1 thematic break");
    }

    #[test]
    fn test_anchor_lookup_uses_document_anchors() {
        // Verify that snapshot blocks get their IDs from the document's anchor system
        let text = "# Heading\n\n- Item 1\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        // Get the document's anchors
        let doc_anchors: std::collections::HashMap<_, _> =
            doc.anchors.iter().map(|a| (a.range.start, a.id)).collect();

        let snapshot = doc.snapshot();

        // Find the heading block
        let heading = snapshot
            .blocks
            .iter()
            .find(|b| matches!(b.kind, BlockKind::Heading { .. }))
            .expect("Should have a heading");

        // Heading should have an ID from the document's anchor system
        assert!(
            doc_anchors.values().any(|&id| id == heading.id),
            "Heading ID should come from document anchors"
        );

        // Find list items
        fn find_list_items(blocks: &[Block]) -> Vec<&Block> {
            let mut items = Vec::new();
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. }) {
                    items.push(block);
                }
                if let BlockContent::Children(children) = &block.content {
                    items.extend(find_list_items(children));
                }
            }
            items
        }

        let list_items = find_list_items(&snapshot.blocks);
        assert_eq!(list_items.len(), 2, "Should have 2 list items");

        // Each list item should have an ID from the document's anchor system
        for item in &list_items {
            assert!(
                doc_anchors.values().any(|&id| id == item.id),
                "List item ID should come from document anchors"
            );
        }

        // All block IDs should be unique
        let mut seen_ids = std::collections::HashSet::new();
        seen_ids.insert(heading.id);
        for item in &list_items {
            assert!(
                seen_ids.insert(item.id),
                "Block IDs should be unique across all blocks"
            );
        }
    }

    #[test]
    fn test_anchor_lookup_fallback_for_unanchored_blocks() {
        // Paragraphs and other blocks without explicit anchors should get fallback IDs
        let text = "A paragraph\n\nAnother paragraph";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Should have 2 paragraphs
        let paragraphs: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, BlockKind::Paragraph))
            .collect();
        assert_eq!(paragraphs.len(), 2, "Should have 2 paragraphs");

        // Paragraphs should have different IDs (generated from their ranges)
        assert_ne!(
            paragraphs[0].id, paragraphs[1].id,
            "Different paragraphs should have different IDs"
        );

        // Neither should be AnchorId(0)
        assert_ne!(
            paragraphs[0].id,
            AnchorId(0),
            "Paragraph should not have placeholder ID"
        );
        assert_ne!(
            paragraphs[1].id,
            AnchorId(0),
            "Paragraph should not have placeholder ID"
        );
    }

    #[test]
    fn test_paragraph_with_whitespace_only_line() {
        // Regression: lines that are all whitespace caused invalid range (start > end)
        let text = "First line\n   \nThird line";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();
        let source = doc.text();

        for block in &snapshot.blocks {
            for line in &block.lines {
                assert!(
                    line.content.start <= line.content.end,
                    "Invalid content range: {:?}",
                    line.content
                );
                // Must not panic when slicing
                let _content = &source[line.content.clone()];
            }
        }
    }

    #[test]
    fn test_paragraph_with_empty_line() {
        // Edge case: paragraph containing just newline
        let text = "Line one\n\nLine two";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();
        let source = doc.text();

        for block in &snapshot.blocks {
            for line in &block.lines {
                assert!(
                    line.content.start <= line.content.end,
                    "Invalid content range: {:?}",
                    line.content
                );
                let _content = &source[line.content.clone()];
            }
        }
    }

    #[test]
    fn test_blockquote_with_whitespace_line() {
        // Regression: blockquote lines that are "> " only caused invalid range
        let text = "> First\n>    \n> Third";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();
        let source = doc.text();

        for block in &snapshot.blocks {
            for line in &block.lines {
                assert!(
                    line.content.start <= line.content.end,
                    "Invalid content range: {:?}",
                    line.content
                );
                let _content = &source[line.content.clone()];
            }
        }
    }

    // ============ InlineSegment tests ============

    #[test]
    fn test_segments_plain_text_paragraph() {
        // Plain text paragraph -> single Text segment
        let text = "Hello world";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("Hello world".to_string())
        );
    }

    #[test]
    fn test_segments_strong_emphasis() {
        // **bold** -> Strong segment with "bold" content
        let text = "**bold**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Strong("bold".to_string())
        );
    }

    #[test]
    fn test_segments_emphasis() {
        // *italic* -> Emphasis segment
        let text = "*italic*";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Emphasis("italic".to_string())
        );
    }

    #[test]
    fn test_segments_mixed_text_and_strong() {
        // Hello **world** -> [Text("Hello "), Strong("world")]
        let text = "Hello **world**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 2);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("Hello ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            SegmentKind::Strong("world".to_string())
        );
    }

    #[test]
    fn test_segments_wikilink() {
        // [[Page]] -> WikiLink segment
        let text = "[[Page]]";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::WikiLink {
                target: "Page".to_string(),
                alias: None
            }
        );
    }

    #[test]
    fn test_segments_wikilink_with_alias() {
        // [[target|display]] -> WikiLink segment with alias
        let text = "[[target|display]]";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::WikiLink {
                target: "target".to_string(),
                alias: Some("display".to_string())
            }
        );
    }

    #[test]
    fn test_segments_complex_mixed() {
        // See [[link]] and **bold** text
        let text = "See [[link]] and **bold** text";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        // Should be: [Text("See "), WikiLink, Text(" and "), Strong, Text(" text")]
        assert_eq!(block.segments.len(), 5);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("See ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            SegmentKind::WikiLink {
                target: "link".to_string(),
                alias: None
            }
        );
        assert_eq!(
            block.segments[2].kind,
            SegmentKind::Text(" and ".to_string())
        );
        assert_eq!(
            block.segments[3].kind,
            SegmentKind::Strong("bold".to_string())
        );
        assert_eq!(
            block.segments[4].kind,
            SegmentKind::Text(" text".to_string())
        );
    }

    #[test]
    fn test_segments_code_span() {
        // Use `code` inline
        let text = "Use `code` inline";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 3);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("Use ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            SegmentKind::Code("code".to_string())
        );
        assert_eq!(
            block.segments[2].kind,
            SegmentKind::Text(" inline".to_string())
        );
    }

    #[test]
    fn test_segments_link() {
        // [text](url)
        let text = "[click here](https://example.com)";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Link {
                text: "click here".to_string(),
                url: "https://example.com".to_string()
            }
        );
    }

    #[test]
    fn test_segments_image() {
        // ![alt](url)
        let text = "![image alt](https://example.com/img.png)";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Image {
                alt: "image alt".to_string(),
                url: "https://example.com/img.png".to_string()
            }
        );
    }

    #[test]
    fn test_segments_list_item() {
        // List items should also have segments
        let text = "- Item with **bold**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Find the list item
        fn find_list_item(blocks: &[Block]) -> Option<&Block> {
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. }) {
                    return Some(block);
                }
                if let BlockContent::Children(children) = &block.content
                    && let Some(found) = find_list_item(children)
                {
                    return Some(found);
                }
            }
            None
        }

        let item = find_list_item(&snapshot.blocks).expect("Should have list item");
        assert_eq!(item.segments.len(), 2);
        assert_eq!(
            item.segments[0].kind,
            SegmentKind::Text("Item with ".to_string())
        );
        assert_eq!(
            item.segments[1].kind,
            SegmentKind::Strong("bold".to_string())
        );
    }

    #[test]
    fn test_segments_heading() {
        // Headings should have segments
        let text = "# Heading with *emphasis*";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 2);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("Heading with ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            SegmentKind::Emphasis("emphasis".to_string())
        );
    }

    #[test]
    fn test_segments_code_block() {
        // Code blocks have a single Text segment with the raw content
        let text = "```\ncode\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        // Code blocks have a Text segment with the code content
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Text("code".to_string())
        );
    }

    #[test]
    fn test_segments_strikethrough() {
        // ~~strikethrough~~
        let text = "~~struck~~";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            SegmentKind::Strikethrough("struck".to_string())
        );
    }

    #[test]
    fn test_segments_hard_break() {
        // Hard break with trailing spaces
        let text = "Line one  \nLine two";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Find any block with HardBreak segment
        let has_hard_break = snapshot.blocks.iter().any(|b| {
            b.segments
                .iter()
                .any(|s| matches!(s.kind, SegmentKind::HardBreak))
        });
        assert!(has_hard_break, "Should have HardBreak segment");
    }

    #[test]
    fn test_segments_adjacent_formatting() {
        // Adjacent formatting: **bold***italic*
        let text = "**bold***italic*";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        // Should have Strong and Emphasis segments
        assert!(
            block
                .segments
                .iter()
                .any(|s| matches!(s.kind, SegmentKind::Strong(_)))
        );
        assert!(
            block
                .segments
                .iter()
                .any(|s| matches!(s.kind, SegmentKind::Emphasis(_)))
        );
    }
}
