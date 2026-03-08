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

/// An inline element within a block (emphasis, links, hard breaks, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct InlineElement {
    /// What kind of inline this is
    pub kind: InlineKind,
    /// Byte range in source
    pub range: Range<usize>,
}

/// The kind of inline element with ranges for display-relevant parts
#[derive(Debug, Clone, PartialEq)]
pub enum InlineKind {
    /// Hard line break (two+ trailing spaces before newline)
    HardBreak,
    /// Emphasis (*text* or _text_) - content is the text without markers
    Emphasis { content: Range<usize> },
    /// Strong emphasis (**text** or __text__) - content is the text without markers
    Strong { content: Range<usize> },
    /// Inline code (`code`) - content is the code without backticks
    Code { content: Range<usize> },
    /// Link [text](url) - separate ranges for display text and URL
    Link {
        text: Range<usize>,
        url: Range<usize>,
    },
    /// Wiki link [[target]] or [[target|alias]]
    WikiLink {
        target: Range<usize>,
        alias: Option<Range<usize>>,
    },
    /// Image ![alt](url) - separate ranges for alt text and URL
    Image {
        alt: Range<usize>,
        url: Range<usize>,
    },
    /// Strikethrough ~~text~~ - content is the text without markers
    Strikethrough { content: Range<usize> },
}

/// The kind of block
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    /// Root document container
    Root,
    /// List container (wraps LIST_ITEMs)
    List,
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
    /// Inline elements within this block (emphasis, links, hard breaks, etc.)
    pub inlines: Vec<InlineElement>,
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

    // Inlines
    if !block.inlines.is_empty() {
        writeln!(out, "{}  inlines:", prefix).unwrap();
        for inline in &block.inlines {
            format_inline(out, inline, source, &prefix);
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

fn format_inline(out: &mut String, inline: &InlineElement, source: &str, prefix: &str) {
    use std::fmt::Write;

    match &inline.kind {
        InlineKind::HardBreak => {
            writeln!(
                out,
                "{}    HardBreak [{}..{}]",
                prefix, inline.range.start, inline.range.end
            )
            .unwrap();
        }
        InlineKind::Emphasis { content } => {
            let text = &source[content.clone()];
            writeln!(
                out,
                "{}    Emphasis [{}..{}] content:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, content, text
            )
            .unwrap();
        }
        InlineKind::Strong { content } => {
            let text = &source[content.clone()];
            writeln!(
                out,
                "{}    Strong [{}..{}] content:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, content, text
            )
            .unwrap();
        }
        InlineKind::Code { content } => {
            let text = &source[content.clone()];
            writeln!(
                out,
                "{}    Code [{}..{}] content:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, content, text
            )
            .unwrap();
        }
        InlineKind::Link { text, url } => {
            let text_str = &source[text.clone()];
            let url_str = &source[url.clone()];
            writeln!(
                out,
                "{}    Link [{}..{}] text:{:?} {:?} url:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, text, text_str, url, url_str
            )
            .unwrap();
        }
        InlineKind::WikiLink { target, alias } => {
            let target_str = &source[target.clone()];
            if let Some(alias_range) = alias {
                let alias_str = &source[alias_range.clone()];
                writeln!(
                    out,
                    "{}    WikiLink [{}..{}] target:{:?} {:?} alias:{:?} {:?}",
                    prefix,
                    inline.range.start,
                    inline.range.end,
                    target,
                    target_str,
                    alias_range,
                    alias_str
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}    WikiLink [{}..{}] target:{:?} {:?}",
                    prefix, inline.range.start, inline.range.end, target, target_str
                )
                .unwrap();
            }
        }
        InlineKind::Image { alt, url } => {
            let alt_str = &source[alt.clone()];
            let url_str = &source[url.clone()];
            writeln!(
                out,
                "{}    Image [{}..{}] alt:{:?} {:?} url:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, alt, alt_str, url, url_str
            )
            .unwrap();
        }
        InlineKind::Strikethrough { content } => {
            let text = &source[content.clone()];
            writeln!(
                out,
                "{}    Strikethrough [{}..{}] content:{:?} {:?}",
                prefix, inline.range.start, inline.range.end, content, text
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
        SyntaxKind::LIST => process_list(source, node, root_range, anchors),
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
        kind: BlockKind::List,
        node_range: node_range.clone(),
        root_range: node_range,
        lines: vec![], // List container has no lines of its own
        inlines: vec![],
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

    // Extract inline elements from the list item's content
    // We look in the PARAGRAPH child (if present) since that's where inlines live
    let inlines = node
        .children()
        .find(|c| c.kind() == SyntaxKind::PARAGRAPH)
        .map(|para| extract_inlines(&para, source))
        .unwrap_or_default();

    Some(Block {
        id,
        kind: BlockKind::ListItem { marker },
        node_range,
        root_range,
        lines: vec![first_line_info],
        inlines,
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

    // Extract inline elements
    let inlines = extract_inlines(&node, source);

    // Paragraphs don't have their own anchors in the current model,
    // so we generate a fallback ID from the range
    let id = find_anchor_for_range(anchors, &node_range);

    Some(Block {
        id,
        kind: BlockKind::Paragraph,
        node_range,
        root_range,
        lines,
        inlines,
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

    Some(Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range,
        root_range,
        lines,
        inlines: extract_inlines(&node, source),
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

    Some(Block {
        id,
        kind: BlockKind::Heading { level },
        node_range,
        root_range,
        lines: vec![line_info],
        inlines: extract_inlines(&node, source),
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

    Some(Block {
        id,
        kind: BlockKind::FencedCode { language },
        node_range,
        root_range,
        lines,
        inlines: vec![], // Code blocks don't have inline formatting
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
        inlines: vec![],
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

/// Extract inline elements from a node's children.
/// Returns interesting inlines: HARD_BREAK, EMPHASIS, STRONG, CODE_SPAN, LINK, etc.
fn extract_inlines(node: &SyntaxNode, source: &str) -> Vec<InlineElement> {
    let mut inlines = Vec::new();

    for child in node.children_with_tokens() {
        let range: Range<usize> = {
            let r = child.text_range();
            (r.start().into())..(r.end().into())
        };
        let text = &source[range.clone()];

        let kind = match &child {
            SyntaxElement::Token(token) => match token.kind() {
                SyntaxKind::HARD_BREAK => Some(InlineKind::HardBreak),
                _ => None,
            },
            SyntaxElement::Node(node) => match node.kind() {
                SyntaxKind::EMPHASIS => {
                    // *text* or _text_ - skip marker on each side
                    let content = (range.start + 1)..(range.end - 1);
                    Some(InlineKind::Emphasis { content })
                }
                SyntaxKind::STRONG => {
                    // **text** or __text__ - skip 2 markers on each side
                    let content = (range.start + 2)..(range.end - 2);
                    Some(InlineKind::Strong { content })
                }
                SyntaxKind::CODE_SPAN => {
                    // `code` - skip backtick on each side
                    let content = (range.start + 1)..(range.end - 1);
                    Some(InlineKind::Code { content })
                }
                SyntaxKind::LINK => {
                    // [text](url) - find the ] and ( positions
                    parse_link_ranges(text, range.start)
                }
                SyntaxKind::WIKILINK => {
                    // [[target]] or [[target|alias]]
                    parse_wikilink_ranges(text, range.start)
                }
                SyntaxKind::IMAGE => {
                    // ![alt](url) - similar to link but starts with !
                    parse_image_ranges(text, range.start)
                }
                SyntaxKind::STRIKETHROUGH => {
                    // ~~text~~ - skip 2 markers on each side
                    let content = (range.start + 2)..(range.end - 2);
                    Some(InlineKind::Strikethrough { content })
                }
                _ => None,
            },
        };

        if let Some(kind) = kind {
            inlines.push(InlineElement { kind, range });
        }
    }

    inlines
}

/// Parse [text](url) into separate ranges
fn parse_link_ranges(text: &str, offset: usize) -> Option<InlineKind> {
    // Format: [text](url)
    let close_bracket = text.find(']')?;
    let open_paren = text[close_bracket..].find('(')? + close_bracket;
    let close_paren = text.rfind(')')?;

    let text_range = (offset + 1)..(offset + close_bracket);
    let url_range = (offset + open_paren + 1)..(offset + close_paren);

    Some(InlineKind::Link {
        text: text_range,
        url: url_range,
    })
}

/// Parse [[target]] or [[target|alias]] into separate ranges
fn parse_wikilink_ranges(text: &str, offset: usize) -> Option<InlineKind> {
    // Format: [[target]] or [[target|alias]]
    // Skip opening [[ and closing ]]
    let inner_start = 2;
    let inner_end = text.len() - 2;
    let inner = &text[inner_start..inner_end];

    if let Some(pipe_pos) = inner.find('|') {
        // [[target|alias]]
        let target = (offset + inner_start)..(offset + inner_start + pipe_pos);
        let alias = (offset + inner_start + pipe_pos + 1)..(offset + inner_end);
        Some(InlineKind::WikiLink {
            target,
            alias: Some(alias),
        })
    } else {
        // [[target]]
        let target = (offset + inner_start)..(offset + inner_end);
        Some(InlineKind::WikiLink {
            target,
            alias: None,
        })
    }
}

/// Parse ![alt](url) into separate ranges
fn parse_image_ranges(text: &str, offset: usize) -> Option<InlineKind> {
    // Format: ![alt](url)
    let close_bracket = text.find(']')?;
    let open_paren = text[close_bracket..].find('(')? + close_bracket;
    let close_paren = text.rfind(')')?;

    // Skip the leading ! for alt text start
    let alt_range = (offset + 2)..(offset + close_bracket);
    let url_range = (offset + open_paren + 1)..(offset + close_paren);

    Some(InlineKind::Image {
        alt: alt_range,
        url: url_range,
    })
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
        assert_eq!(list.kind, BlockKind::List);

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

        // Should have a paragraph with wiki-link inline
        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::Paragraph);

        // Find wiki-link inline
        let wikilinks: Vec<_> = block
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1, "Should have 1 wiki-link");

        if let InlineKind::WikiLink { target, alias } = &wikilinks[0].kind {
            assert_eq!(&text[target.clone()], "Page Name");
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1);

        if let InlineKind::WikiLink { target, alias } = &wikilinks[0].kind {
            assert_eq!(&text[target.clone()], "target");
            assert!(alias.is_some());
            assert_eq!(&text[alias.clone().unwrap()], "display text");
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::WikiLink { .. }))
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::Link { .. }))
            .collect();
        assert_eq!(links.len(), 1, "Should have 1 link");

        if let InlineKind::Link {
            text: text_range,
            url,
        } = &links[0].kind
        {
            assert_eq!(&text[text_range.clone()], "here");
            assert_eq!(&text[url.clone()], "https://example.com");
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::Emphasis { .. }))
            .collect();
        assert_eq!(emphasis.len(), 1, "Should have 1 emphasis");

        if let InlineKind::Emphasis { content } = &emphasis[0].kind {
            assert_eq!(&text[content.clone()], "emphasized");
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::Strong { .. }))
            .collect();
        assert_eq!(strong.len(), 1, "Should have 1 strong");

        if let InlineKind::Strong { content } = &strong[0].kind {
            assert_eq!(&text[content.clone()], "strong");
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
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::Code { .. }))
            .collect();
        assert_eq!(code.len(), 1, "Should have 1 code span");

        if let InlineKind::Code { content } = &code[0].kind {
            assert_eq!(&text[content.clone()], "code");
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
}
