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
    /// For blocks with children, this is the range before children begin -
    /// includes the marker and any continuation lines, but excludes nested blocks.
    pub fn content_range(&self) -> Range<usize> {
        match &self.content {
            BlockContent::Leaf => self.node_range.clone(),
            BlockContent::Children(children) => {
                if let Some(first_child) = children.first() {
                    self.node_range.start..first_child.node_range.start
                } else {
                    // Children vector is empty - treat as leaf
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
        segments: vec![],
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

    // Content starts after marker, ends before newline
    let content_start = node_range.start + marker_len;
    let content_end = node_range.start + first_line_content_end;

    // Process children (nested content)
    // Skip PARAGRAPH children - segments are extracted separately below.
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
    let segments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::PARAGRAPH)
        .map(|para| {
            // Use paragraph's range, but start after the list marker
            // and exclude trailing newline
            let para_range = para.text_range();
            let para_start: usize = para_range.start().into();
            let mut para_end: usize = para_range.end().into();
            // Strip trailing newline from content range
            if para_end > para_start && source.as_bytes().get(para_end - 1) == Some(&b'\n') {
                para_end -= 1;
            }
            let content_range = content_start.max(para_start)..para_end;
            extract_segments(&para, source, content_range)
        })
        .unwrap_or_else(|| {
            // No paragraph child - use first line content range as fallback
            let fallback_range = content_start..content_end;
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
        root_range,
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
        root_range,
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

    // Check for nested blockquotes in children
    let mut children = Vec::new();
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

    let id = find_anchor_for_range(anchors, &node_range);

    // Extract segments from blockquote content (after "> " prefix)
    let prefix_len = text.find(|c: char| c != '>' && c != ' ').unwrap_or(0);
    let content_start = node_range.start + prefix_len;
    let content_end = if text.ends_with('\n') {
        node_range.end - 1
    } else {
        node_range.end
    };
    let segments = extract_segments(&node, source, content_start..content_end);

    Some(Block {
        id,
        kind: BlockKind::BlockQuote,
        node_range,
        root_range,
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
        root_range,
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
        root_range,
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

        // Add Text segment for gap before this inline
        if inline.range.start > cursor {
            let text_end = inline.range.start.min(content_range.end);
            let text = &source[cursor..text_end];
            if !text.is_empty() {
                segments.push(InlineSegment {
                    kind: InlineNode::Text(text.to_string()),
                    range: cursor..text_end,
                });
            }
        }

        // Add the inline segment
        segments.push(InlineSegment {
            kind: inline.node.clone(),
            range: inline.range.clone(),
        });

        cursor = inline.range.end.max(cursor);
    }

    // Add trailing Text segment
    if cursor < content_range.end {
        let text = &source[cursor..content_range.end];
        if !text.is_empty() {
            segments.push(InlineSegment {
                kind: InlineNode::Text(text.to_string()),
                range: cursor..content_range.end,
            });
        }
    }

    // If no inlines found, entire content is plain text
    if segments.is_empty() && !content_range.is_empty() {
        let text = &source[content_range.clone()];
        if !text.is_empty() {
            segments.push(InlineSegment {
                kind: InlineNode::Text(text.to_string()),
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

        // Root range (if different)
        if block.root_range != block.node_range {
            writeln!(out, "{}  root_range: {:?}", prefix, block.root_range).unwrap();
        }

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
        }
    }

    // ============ Snapshot tests ============

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
        let formatted = insta_format_snapshot(&snapshot);

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
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            InlineNode::Text("Hello World".to_string())
        );
        assert_eq!(block.segments[0].range, 2..13);
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
    fn test_code_fence_content_excludes_closing_fence() {
        // The code fence content should NOT include the closing ``` markers
        // Bug: when there's a trailing newline after closing fence, rfind('\n')
        // finds that trailing newline instead of the one before the closing fence
        let text = "```rust\nfn main() {}\n```\n";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];

        // Verify the segment content does not include the closing fence
        assert_eq!(block.segments.len(), 1);
        if let InlineNode::Text(content) = &block.segments[0].kind {
            assert_eq!(content, "fn main() {}");
            assert!(
                !content.contains("```"),
                "Content should not include closing fence, got: {content:?}"
            );
        } else {
            panic!("Expected Text segment");
        }
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
                for segment in &block.segments {
                    assert!(
                        segment.range.end <= doc_len,
                        "segment.range {:?} exceeds document length {}",
                        segment.range,
                        doc_len
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
            .filter(|s| matches!(s.kind, InlineNode::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1, "Should have 1 wiki-link");

        if let InlineNode::WikiLink { target, alias } = &wikilinks[0].kind {
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
            .filter(|s| matches!(s.kind, InlineNode::WikiLink { .. }))
            .collect();
        assert_eq!(wikilinks.len(), 1);

        if let InlineNode::WikiLink { target, alias } = &wikilinks[0].kind {
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
            .filter(|s| matches!(s.kind, InlineNode::WikiLink { .. }))
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
            .filter(|s| matches!(s.kind, InlineNode::Link { .. }))
            .collect();
        assert_eq!(links.len(), 1, "Should have 1 link");

        if let InlineNode::Link {
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
            .filter(|s| matches!(s.kind, InlineNode::Emphasis(_)))
            .collect();
        assert_eq!(emphasis.len(), 1, "Should have 1 emphasis");

        if let InlineNode::Emphasis(children) = &emphasis[0].kind {
            assert_eq!(children.len(), 1, "Should have 1 child");
            if let InlineNode::Text(text) = &children[0] {
                assert_eq!(text, "emphasized");
            } else {
                panic!("Expected Text child");
            }
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
            .filter(|s| matches!(s.kind, InlineNode::Strong(_)))
            .collect();
        assert_eq!(strong.len(), 1, "Should have 1 strong");

        if let InlineNode::Strong(children) = &strong[0].kind {
            assert_eq!(children.len(), 1, "Should have 1 child");
            if let InlineNode::Text(text) = &children[0] {
                assert_eq!(text, "strong");
            } else {
                panic!("Expected Text child");
            }
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
            .filter(|s| matches!(s.kind, InlineNode::Code(_)))
            .collect();
        assert_eq!(code.len(), 1, "Should have 1 code span");

        if let InlineNode::Code(content) = &code[0].kind {
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
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            InlineNode::Text("This is a quote".to_string())
        );
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
            for segment in &block.segments {
                assert!(
                    segment.range.start <= segment.range.end,
                    "Invalid segment range: {:?}",
                    segment.range
                );
                // Must not panic when slicing
                let _content = &source[segment.range.clone()];
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
            for segment in &block.segments {
                assert!(
                    segment.range.start <= segment.range.end,
                    "Invalid segment range: {:?}",
                    segment.range
                );
                let _content = &source[segment.range.clone()];
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
            for segment in &block.segments {
                assert!(
                    segment.range.start <= segment.range.end,
                    "Invalid segment range: {:?}",
                    segment.range
                );
                let _content = &source[segment.range.clone()];
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
            InlineNode::Text("Hello world".to_string())
        );
    }

    #[test]
    fn test_segments_strong_emphasis() {
        // **bold** -> Strong segment with children [Text("bold")]
        let text = "**bold**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            InlineNode::Strong(vec![InlineNode::Text("bold".to_string())])
        );
    }

    #[test]
    fn test_segments_emphasis() {
        // *italic* -> Emphasis segment with children [Text("italic")]
        let text = "*italic*";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);
        assert_eq!(
            block.segments[0].kind,
            InlineNode::Emphasis(vec![InlineNode::Text("italic".to_string())])
        );
    }

    #[test]
    fn test_segments_mixed_text_and_strong() {
        // Hello **world** -> [Text("Hello "), Strong([Text("world")])]
        let text = "Hello **world**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 2);
        assert_eq!(
            block.segments[0].kind,
            InlineNode::Text("Hello ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            InlineNode::Strong(vec![InlineNode::Text("world".to_string())])
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
            InlineNode::WikiLink {
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
            InlineNode::WikiLink {
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
        // Should be: [Text("See "), WikiLink, Text(" and "), Strong([Text("bold")]), Text(" text")]
        assert_eq!(block.segments.len(), 5);
        assert_eq!(block.segments[0].kind, InlineNode::Text("See ".to_string()));
        assert_eq!(
            block.segments[1].kind,
            InlineNode::WikiLink {
                target: "link".to_string(),
                alias: None
            }
        );
        assert_eq!(
            block.segments[2].kind,
            InlineNode::Text(" and ".to_string())
        );
        assert_eq!(
            block.segments[3].kind,
            InlineNode::Strong(vec![InlineNode::Text("bold".to_string())])
        );
        assert_eq!(
            block.segments[4].kind,
            InlineNode::Text(" text".to_string())
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
        assert_eq!(block.segments[0].kind, InlineNode::Text("Use ".to_string()));
        assert_eq!(block.segments[1].kind, InlineNode::Code("code".to_string()));
        assert_eq!(
            block.segments[2].kind,
            InlineNode::Text(" inline".to_string())
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
            InlineNode::Link {
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
            InlineNode::Image {
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
            InlineNode::Text("Item with ".to_string())
        );
        assert_eq!(
            item.segments[1].kind,
            InlineNode::Strong(vec![InlineNode::Text("bold".to_string())])
        );
    }

    #[test]
    fn test_list_item_hanging_indent() {
        let text = "- First\n  second\n";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let list = &snapshot.blocks[0];
        let item = match &list.content {
            BlockContent::Children(c) => &c[0],
            _ => panic!("expected children"),
        };

        // Check that "second" appears in the segments
        let has_second = item.segments.iter().any(|s| match &s.kind {
            InlineNode::Text(t) => t.contains("second"),
            _ => false,
        });
        assert!(has_second, "segments: {:?}", item.segments);
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
            InlineNode::Text("Heading with ".to_string())
        );
        assert_eq!(
            block.segments[1].kind,
            InlineNode::Emphasis(vec![InlineNode::Text("emphasis".to_string())])
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
        assert_eq!(block.segments[0].kind, InlineNode::Text("code".to_string()));
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
            InlineNode::Strikethrough("struck".to_string())
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
                .any(|s| matches!(s.kind, InlineNode::HardBreak))
        });
        assert!(has_hard_break, "Should have HardBreak segment");
    }

    // ============ content_range() tests ============

    #[test]
    fn test_content_range_leaf_returns_node_range() {
        // Leaf blocks return their full node_range as content_range
        let text = "# Hello World";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        let heading = &snapshot.blocks[0];
        assert_eq!(heading.kind, BlockKind::Heading { level: 1 });
        assert_eq!(
            heading.content_range(),
            heading.node_range.clone(),
            "Leaf block content_range should equal node_range"
        );
    }

    #[test]
    fn test_content_range_with_children_excludes_nested() {
        // Block with children: content_range is from node_range.start to first child's start
        let text = "- parent\n  - child\n";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Find the parent list item
        fn find_parent_item(blocks: &[Block]) -> Option<&Block> {
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. })
                    && matches!(block.content, BlockContent::Children(_))
                {
                    return Some(block);
                }
                if let BlockContent::Children(children) = &block.content
                    && let Some(found) = find_parent_item(children)
                {
                    return Some(found);
                }
            }
            None
        }

        let parent = find_parent_item(&snapshot.blocks).expect("Should have parent list item");
        let content_range = parent.content_range();

        // Content range should start at block start
        assert_eq!(content_range.start, parent.node_range.start);

        // Content range should end before first child begins
        if let BlockContent::Children(children) = &parent.content {
            let first_child = &children[0];
            assert_eq!(
                content_range.end, first_child.node_range.start,
                "content_range should end where first child starts"
            );
        }

        // Verify we can slice the content (should be "- parent\n")
        let content = &text[content_range.clone()];
        assert!(
            content.contains("parent"),
            "Content should include 'parent'"
        );
        assert!(
            !content.contains("child"),
            "Content should NOT include 'child'"
        );
    }

    #[test]
    fn test_content_range_multiline_before_children() {
        // List item with continuation line before nested child
        let text = "- first line\n  second line\n  - nested\n";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        fn find_parent_with_nested(blocks: &[Block]) -> Option<&Block> {
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. })
                    && let BlockContent::Children(children) = &block.content
                    && children
                        .iter()
                        .any(|c| matches!(c.kind, BlockKind::List { .. }))
                {
                    return Some(block);
                }
                if let BlockContent::Children(children) = &block.content
                    && let Some(found) = find_parent_with_nested(children)
                {
                    return Some(found);
                }
            }
            None
        }

        let parent =
            find_parent_with_nested(&snapshot.blocks).expect("Should find parent with nested list");
        let content_range = parent.content_range();
        let content = &text[content_range.clone()];

        // Both continuation lines should be in content, but not the nested item
        assert!(
            content.contains("first line"),
            "Content should include first line"
        );
        assert!(
            content.contains("second line"),
            "Content should include continuation line"
        );
        assert!(
            !content.contains("nested"),
            "Content should NOT include nested child"
        );
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
                .any(|s| matches!(s.kind, InlineNode::Strong(_)))
        );
        assert!(
            block
                .segments
                .iter()
                .any(|s| matches!(s.kind, InlineNode::Emphasis(_)))
        );
    }

    #[test]
    fn test_segments_triple_delimiter_nesting() {
        // ***text*** -> Strong containing Emphasis containing Text
        let text = "***nested***";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);

        // Should be Strong([Emphasis([Text("nested")])])
        if let InlineNode::Strong(strong_children) = &block.segments[0].kind {
            assert_eq!(strong_children.len(), 1, "Strong should have 1 child");
            if let InlineNode::Emphasis(em_children) = &strong_children[0] {
                assert_eq!(em_children.len(), 1, "Emphasis should have 1 child");
                assert_eq!(em_children[0], InlineNode::Text("nested".to_string()));
            } else {
                panic!(
                    "Expected Emphasis inside Strong, got {:?}",
                    strong_children[0]
                );
            }
        } else {
            panic!("Expected Strong, got {:?}", block.segments[0].kind);
        }
    }

    #[test]
    fn test_wikilink_inside_strong() {
        // **bold [[link]]** -> Strong containing [Text("bold "), WikiLink]
        let text = "**bold [[link]]**";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let block = &snapshot.blocks[0];
        assert_eq!(block.segments.len(), 1);

        // Should be Strong([Text("bold "), WikiLink])
        if let InlineNode::Strong(children) = &block.segments[0].kind {
            assert_eq!(
                children.len(),
                2,
                "Strong should have 2 children: {:?}",
                children
            );
            assert_eq!(children[0], InlineNode::Text("bold ".to_string()));
            assert_eq!(
                children[1],
                InlineNode::WikiLink {
                    target: "link".to_string(),
                    alias: None
                }
            );
        } else {
            panic!("Expected Strong, got {:?}", block.segments[0].kind);
        }
    }

    #[test]
    fn test_list_item_segments_exclude_marker_and_newline() {
        // Verifies that segments contain content WITHOUT marker or trailing newline,
        // making them suitable for textarea display. The node_range still includes
        // the full block with marker and newline for structural operations.
        let text = "- Item 1\n- Item 2\n";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Find the first list item
        fn find_first_list_item(blocks: &[Block]) -> Option<&Block> {
            for block in blocks {
                if matches!(block.kind, BlockKind::ListItem { .. }) {
                    return Some(block);
                }
                if let BlockContent::Children(children) = &block.content
                    && let Some(found) = find_first_list_item(children)
                {
                    return Some(found);
                }
            }
            None
        }

        let item = find_first_list_item(&snapshot.blocks).expect("Should have list item");

        // Segments should have content without marker or newline
        assert_eq!(item.segments.len(), 1);
        assert_eq!(
            item.segments[0].kind,
            InlineNode::Text("Item 1".to_string())
        );
        // Segment range should be after marker (2) and before newline (8)
        assert_eq!(item.segments[0].range, 2..8);

        // node_range includes the full block with marker and newline
        assert_eq!(
            &text[item.node_range.clone()],
            "- Item 1\n",
            "node_range should include marker and trailing newline"
        );
    }
}
