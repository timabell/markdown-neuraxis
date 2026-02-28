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

use crate::editing::AnchorId;

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
pub enum BlockKindV2 {
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
    pub kind: BlockKindV2,
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
pub struct SnapshotV2 {
    /// Root-level blocks
    pub blocks: Vec<Block>,
}

/// Format a snapshot as a readable string for snapshot testing
pub fn format_snapshot(snapshot: &SnapshotV2, source: &str) -> String {
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
pub fn create_snapshot(doc: &crate::editing::Document) -> SnapshotV2 {
    let source = doc.text();
    if source.is_empty() {
        return SnapshotV2 { blocks: vec![] };
    }

    // Parse using Rowan parser
    let tree = parse(&source);
    let mut blocks = Vec::new();

    // Process top-level children
    for child in tree.children() {
        if let Some(block) = process_node(&source, child, None) {
            blocks.push(block);
        }
    }

    SnapshotV2 { blocks }
}

/// Process a Rowan node into a Block
fn process_node(source: &str, node: SyntaxNode, root_range: Option<Range<usize>>) -> Option<Block> {
    let text_range = node.text_range();
    let node_range = (text_range.start().into())..(text_range.end().into());
    let root_range = root_range.unwrap_or_else(|| node_range.clone());

    match node.kind() {
        SyntaxKind::LIST => process_list(source, node, root_range),
        SyntaxKind::LIST_ITEM => process_list_item(source, node, root_range),
        SyntaxKind::PARAGRAPH => process_paragraph(source, node, root_range),
        SyntaxKind::BLOCK_QUOTE => process_block_quote(source, node, root_range),
        SyntaxKind::HEADING => process_heading(source, node, root_range),
        SyntaxKind::FENCED_CODE => process_fenced_code(source, node, root_range),
        SyntaxKind::THEMATIC_BREAK => process_thematic_break(source, node, root_range),
        _ => None, // Skip unknown node types
    }
}

fn process_list(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
    let text_range = node.text_range();
    let node_range = (text_range.start().into())..(text_range.end().into());
    let mut children = Vec::new();

    for child in node.children() {
        if let Some(block) = process_node(source, child, Some(root_range.clone())) {
            children.push(block);
        }
    }

    Some(Block {
        id: AnchorId(0), // TODO: proper anchor lookup
        kind: BlockKindV2::List,
        node_range: node_range.clone(),
        root_range: node_range,
        lines: vec![], // List container has no lines of its own
        inlines: vec![],
        content: BlockContent::Children(children),
    })
}

fn process_list_item(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
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

    let first_line_info = LineInfo {
        full: line_start..line_end_with_newline,
        prefix: line_start..(line_start + marker_len),
        content: content_start..content_end,
    };

    // Process children (nested content)
    let mut children = Vec::new();
    for child in node.children() {
        // Skip trivia, process actual content
        if let Some(block) = process_node(source, child, Some(root_range.clone())) {
            children.push(block);
        }
    }

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::ListItem { marker },
        node_range,
        root_range,
        lines: vec![first_line_info],
        inlines: vec![],
        content,
    })
}

fn process_paragraph(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
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

        lines.push(LineInfo {
            full: pos..line_end,
            prefix: pos..(pos + trimmed_start),
            content: (pos + trimmed_start)..content_end,
        });

        pos = line_end;
    }

    // Extract inline elements
    let inlines = extract_inlines(&node, source);

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::Paragraph,
        node_range,
        root_range,
        lines,
        inlines,
        content: BlockContent::Leaf,
    })
}

fn process_block_quote(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
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

        lines.push(LineInfo {
            full: actual_line_start..line_end,
            prefix: actual_line_start..(actual_line_start + prefix_end),
            content: (actual_line_start + prefix_end)..content_end,
        });

        pos = line_end;
    }

    // Check for nested blockquotes in children
    for child in node.children() {
        if child.kind() == SyntaxKind::BLOCK_QUOTE
            && let Some(block) = process_block_quote(source, child, root_range.clone())
        {
            children.push(block);
        }
    }

    let content = if children.is_empty() {
        BlockContent::Leaf
    } else {
        BlockContent::Children(children)
    };

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::BlockQuote,
        node_range,
        root_range,
        lines,
        inlines: extract_inlines(&node, source),
        content,
    })
}

fn process_heading(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
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

    let line_info = LineInfo {
        full: node_range.clone(),
        prefix: node_range.start..(node_range.start + prefix_end),
        content: (node_range.start + prefix_end)..content_end,
    };

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::Heading { level },
        node_range,
        root_range,
        lines: vec![line_info],
        inlines: extract_inlines(&node, source),
        content: BlockContent::Leaf,
    })
}

fn process_fenced_code(source: &str, node: SyntaxNode, root_range: Range<usize>) -> Option<Block> {
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

        lines.push(LineInfo {
            full: pos..line_end,
            prefix: pos..pos, // Code blocks have no prefix
            content: pos..content_end,
        });

        pos = line_end;
    }

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::FencedCode { language },
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
) -> Option<Block> {
    let text_range = node.text_range();
    let node_range: Range<usize> = (text_range.start().into())..(text_range.end().into());

    Some(Block {
        id: AnchorId(0),
        kind: BlockKindV2::ThematicBreak,
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
}
