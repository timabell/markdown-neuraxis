use crate::editing::{AnchorId, Document, document::Marker};
use regex::Regex;

/// Content grouping for structured UI rendering (ADR-0004 View Layer)
///
/// ContentGroups implement the hierarchical rendering structure needed by frontend
/// frameworks. They group flat `RenderBlock` sequences into semantically meaningful
/// structures like nested lists, enabling proper HTML `<ul>`/`<ol>` generation.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentGroup {
    /// Single block element (heading, paragraph, code, etc.)
    SingleBlock(RenderBlock),
    /// Consecutive bullet list items grouped for `<ul>` rendering
    BulletListGroup { items: Vec<ListItem> },
    /// Consecutive numbered list items grouped for `<ol>` rendering  
    NumberedListGroup { items: Vec<ListItem> },
}

/// Hierarchical list item with nested children (ADR-0004)
///
/// ListItems represent the nested structure of Markdown lists, enabling proper
/// recursive rendering in UI frameworks. Each item contains its own content block
/// plus any child items at deeper indentation levels.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// The render block for this list item's content
    pub block: RenderBlock,
    /// Child list items at deeper indentation (recursive structure)
    pub children: Vec<ListItem>,
}

/// Immutable document snapshot for UI rendering (ADR-0004 Read API)
///
/// Snapshots implement the "Read API" described in ADR-4. They provide an **immutable view**
/// of the document structure without exposing the underlying xi-rope buffer. This enables:
///
/// - **Clean UI separation**: UI renders from snapshots, never mutates rope directly
/// - **Stable references**: Blocks have persistent AnchorIds across document edits
/// - **Version tracking**: UI can detect when document changes require re-rendering
/// - **Multiple frontends**: Same snapshot structure works for Dioxus, TUI, etc.
///
/// ## Rendering Pattern
///
/// ```rust
/// # use markdown_neuraxis_engine::editing::{Document, ContentGroup};
/// # let mut doc = Document::from_bytes(b"# Hello\n\n- Item 1\n- Item 2").unwrap();
/// # doc.create_anchors_from_tree();
/// let snapshot = doc.snapshot();
/// for group in &snapshot.content_groups {
///     match group {
///         ContentGroup::BulletListGroup { items } => {
///             println!("Bullet list with {} items", items.len());
///         },
///         ContentGroup::NumberedListGroup { items } => {
///             println!("Numbered list with {} items", items.len());
///         },
///         ContentGroup::SingleBlock(block) => {
///             println!("Single block: {}", block.content);
///         },
///     }
/// }
/// ```
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    /// Document version for change detection
    pub version: u64,
    /// Flat list of blocks (backward compatibility, will be deprecated)
    pub blocks: Vec<RenderBlock>,
    /// Hierarchical content structure for modern UI rendering
    pub content_groups: Vec<ContentGroup>,
}

/// UI-ready block with stable identity and content metadata (ADR-0004)
///
/// RenderBlocks represent individual document elements prepared for UI consumption.
/// They contain all information needed for rendering without requiring access to
/// the underlying document buffer.
///
/// ## Key Fields
///
/// - **`id`**: Stable AnchorId that persists across edits
/// - **`kind`**: Block type determining rendering strategy  
/// - **`byte_range`**: Full block range in original document
/// - **`content_range`**: Editable text range (excludes Markdown syntax)
/// - **`content`**: Extracted text ready for display/editing
/// - **`segments`**: Parsed wiki-links and URLs for interactive rendering
///
/// ## UI Integration
///
/// The block provides both "pretty" rendering content and precise editing ranges:
///
/// ```rust
/// # use markdown_neuraxis_engine::editing::{Document, BlockKind};
/// # let mut doc = Document::from_bytes(b"# Hello World\n\n- Item 1").unwrap();
/// # doc.create_anchors_from_tree();
/// # let snapshot = doc.snapshot();
/// # let block = &snapshot.blocks[0]; // Get first block (heading)
/// match block.kind {
///     BlockKind::Heading { level } => {
///         // Pretty rendering: show content without # markers  
///         println!("Heading level {}: {}", level, block.content);
///         // Edit mode: focus on content_range for raw Markdown editing
///         println!("Edit range: {:?}", block.content_range);
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RenderBlock {
    /// Stable identifier that persists across document edits
    pub id: AnchorId,
    /// Block type with associated metadata
    pub kind: BlockKind,
    /// Full byte range of this block in the document
    pub byte_range: std::ops::Range<usize>,
    /// Editable content range (excludes Markdown prefixes/suffixes)
    pub content_range: std::ops::Range<usize>,
    /// Indentation depth for hierarchical display
    pub depth: usize,
    /// Extracted content text ready for rendering
    pub content: String,
    /// Parsed inline elements (wiki-links, URLs) for interactive rendering
    pub segments: Option<Vec<TextSegment>>,
}

/// Markdown block type classification for rendering (ADR-0004)
///
/// BlockKind determines how UI frameworks should render each block. The enum
/// provides sufficient metadata for both visual styling and editing behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    /// Regular paragraph text
    Paragraph,
    /// ATX heading (# to ######)
    Heading { level: u8 },
    /// List item with marker type and nesting depth
    ListItem { marker: Marker, depth: usize },
    /// Fenced code block with optional language
    CodeFence { lang: Option<String> },
    /// Horizontal rule (---, ***, ___)
    ThematicBreak,
    /// Block quote (> quoted text)
    BlockQuote,
    /// Fallback for unrecognized Markdown elements
    UnhandledMarkdown,
}

/// Parsed inline content for interactive rendering (ADR-0004)
///
/// TextSegments enable rich inline rendering within blocks. They parse wiki-links
/// and URLs from block content, enabling click navigation and hover previews.
///
/// ## Parsing Strategy
///
/// - **Wiki-links**: `[[Page Name]]` syntax for internal navigation
/// - **URLs**: `http://` and `https://` links with punctuation cleanup
/// - **Overlap resolution**: Wiki-links take precedence over URLs
/// - **Plain text**: Everything else renders as regular text
#[derive(Debug, Clone, PartialEq)]
pub enum TextSegment {
    /// Regular text content
    Text(String),
    /// Wiki-style link [[target]] for internal navigation
    WikiLink { target: String },
    /// HTTP/HTTPS URL for external links
    Url { href: String },
}

/// Create immutable snapshot from document state (ADR-0004 Read API)
///
/// This function implements the core snapshot generation described in ADR-4. It
/// traverses the Tree-sitter CST and produces UI-ready rendering blocks without
/// exposing the internal xi-rope buffer structure.
///
/// ## Snapshot Generation Process
///
/// 1. **CST Traversal**: Recursively walks Tree-sitter parse tree
/// 2. **Block Extraction**: Converts nodes to RenderBlocks with content metadata  
/// 3. **Range Calculation**: Separates structural (byte_range) from editable (content_range)
/// 4. **Anchor Association**: Links blocks to stable AnchorIds for UI consistency
/// 5. **Content Grouping**: Organizes flat blocks into hierarchical ContentGroups
/// 6. **Inline Parsing**: Extracts wiki-links and URLs for interactive rendering
///
/// ## Block Range Types
///
/// - **`byte_range`**: Full structural range including Markdown syntax
/// - **`content_range`**: Editable text range excluding markers/prefixes
///
/// Example: `"# Heading"` → byte_range: 0..9, content_range: 2..9
///
/// ## UI Framework Integration
///
/// The snapshot structure enables both:
/// - **Pretty rendering**: Display content without Markdown syntax
/// - **Raw editing**: Focus content_range for textarea-based editing
///
/// This follows ADR-4's "pretty everywhere, raw in focused block" pattern.
pub(crate) fn create_snapshot(doc: &Document) -> Snapshot {
    let mut blocks = Vec::new();

    if let Some(ref tree) = doc.tree {
        let root_node = tree.root_node();
        collect_render_blocks_recursive(doc, root_node, &mut blocks, 0);
    }

    let content_groups = group_blocks_for_rendering(&blocks);

    Snapshot {
        version: doc.version,
        blocks,
        content_groups,
    }
}

/// Recursively collect render blocks from the tree-sitter CST
fn collect_render_blocks_recursive(
    doc: &Document,
    node: tree_sitter::Node,
    blocks: &mut Vec<RenderBlock>,
    current_depth: usize,
) {
    let node_kind = node.kind();
    let byte_range = node.byte_range();

    // Skip empty nodes
    if byte_range.is_empty() {
        return;
    }

    match node_kind {
        "atx_heading" => {
            let level = extract_heading_level(doc, &node);
            let content_range = extract_heading_content_range(doc, &node);
            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let content = doc.slice_to_cow(content_range.clone()).trim().to_string();
            let segments = parse_text_segments(&content);
            let segments = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. } | TextSegment::Url { .. }))
            {
                Some(segments)
            } else {
                None
            };

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::Heading { level },
                byte_range,
                content_range,
                depth: current_depth,
                content,
                segments,
            });
        }
        "list_item" => {
            let marker = extract_list_marker(doc, &node);
            let list_depth = calculate_list_depth(doc, &node);
            let content_range = extract_list_item_content_range(doc, &node);
            // Find existing anchor for this list item using the same buggy range calculation as anchor creation
            let anchor_range = calculate_list_item_anchor_range(&node);
            let anchor_id = find_existing_anchor_for_node(doc, &node, &anchor_range);
            let own_byte_range = extract_list_item_own_range(doc, &node);

            let content = doc.slice_to_cow(content_range.clone()).trim().to_string();
            let segments = parse_text_segments(&content);
            let segments = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. } | TextSegment::Url { .. }))
            {
                Some(segments)
            } else {
                None
            };

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::ListItem {
                    marker,
                    depth: list_depth,
                },
                byte_range: own_byte_range,
                content_range,
                depth: list_depth,
                content,
                segments,
            });

            // Only process nested list children, not the list item's own content
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "list" {
                    collect_render_blocks_recursive(doc, child, blocks, list_depth);
                }
            }
        }
        "paragraph" => {
            // Only create paragraph render blocks if they're not inside list items
            // Check if the parent is a list_item
            let is_inside_list_item = node.parent().map(|p| p.kind()) == Some("list_item");

            if !is_inside_list_item {
                // Top-level paragraph
                let content_range = extract_paragraph_content_range(doc, &node);
                let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
                let content = doc.slice_to_cow(content_range.clone()).trim().to_string();
                let segments = parse_text_segments(&content);
                let segments = if segments
                    .iter()
                    .any(|s| matches!(s, TextSegment::WikiLink { .. } | TextSegment::Url { .. }))
                {
                    Some(segments)
                } else {
                    None
                };

                blocks.push(RenderBlock {
                    id: anchor_id,
                    kind: BlockKind::Paragraph,
                    byte_range: content_range.clone(), // Use content_range for byte_range to exclude trailing newlines
                    content_range,
                    depth: current_depth,
                    content,
                    segments,
                });
            }
            // If inside a list item, skip the paragraph block entirely
            // The list item will handle its own content
        }
        "fenced_code_block" => {
            let lang = extract_code_fence_language(doc, &node);
            let content_range = extract_code_fence_content_range(doc, &node);
            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let content = doc.slice_to_cow(content_range.clone()).to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::CodeFence { lang },
                byte_range,
                content_range,
                depth: current_depth,
                content,
                segments: None, // Code blocks should not parse wikilinks
            });
        }
        "indented_code_block" => {
            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let content = doc.slice_to_cow(byte_range.clone()).to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::CodeFence { lang: None },
                byte_range: byte_range.clone(),
                content_range: byte_range.clone(),
                depth: current_depth,
                content,
                segments: None, // Code blocks should not parse wikilinks
            });
        }
        "thematic_break" => {
            // Horizontal rule (---, ***, ___)
            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let content = doc.slice_to_cow(byte_range.clone()).to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::ThematicBreak,
                byte_range: byte_range.clone(),
                content_range: byte_range,
                depth: current_depth,
                content,
                segments: None, // Thematic breaks should not parse wikilinks
            });
        }
        "block_quote" => {
            // > Quoted text
            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let content = doc.slice_to_cow(byte_range.clone()).to_string();
            let segments = parse_text_segments(&content);
            let segments = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. } | TextSegment::Url { .. }))
            {
                Some(segments)
            } else {
                None
            };

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::BlockQuote,
                byte_range: byte_range.clone(),
                content_range: byte_range,
                depth: current_depth,
                content,
                segments,
            });
        }
        "document" | "section" => {
            // Container nodes - just process children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_render_blocks_recursive(doc, child, blocks, current_depth);
            }
        }
        "list" => {
            // List container - process list_item children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_render_blocks_recursive(doc, child, blocks, current_depth);
            }
        }
        _ => {
            // Skip empty ranges (these are often parser artifacts)
            if byte_range.is_empty() {
                return;
            }

            // Unknown/unhandled node type - render as UnhandledMarkdown
            let start_point = node.start_position();
            let content = doc.slice_to_cow(byte_range.clone()).to_string();

            eprintln!(
                "Warning: Unknown markdown element '{}' at line {} (will render as-is): {}",
                node.kind(),
                start_point.row + 1, // Convert 0-based to 1-based line numbers
                content.lines().next().unwrap_or(&content).trim()
            );

            let anchor_id = find_existing_anchor_for_node(doc, &node, &byte_range);
            let segments = parse_text_segments(&content);
            let segments = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. } | TextSegment::Url { .. }))
            {
                Some(segments)
            } else {
                None
            };

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::UnhandledMarkdown,
                byte_range: byte_range.clone(),
                content_range: byte_range,
                depth: current_depth,
                content,
                segments,
            });
        }
    }
}

/// Extract heading level from an ATX heading node
fn extract_heading_level(doc: &Document, node: &tree_sitter::Node) -> u8 {
    let text = doc.slice_to_cow(node.byte_range());
    // Count the number of # characters at the start
    let level = text.chars().take_while(|&c| c == '#').count() as u8;
    level.clamp(1, 6) // ATX headings are level 1-6
}

/// Extract content range for a heading (after the # markers and space)
fn extract_heading_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find where the content starts (after # and space)
    let mut content_start = byte_range.start;
    let chars = text.char_indices();

    // Skip the # characters
    for (i, ch) in chars {
        if ch == '#' {
            content_start = byte_range.start + i + 1;
        } else {
            break;
        }
    }

    // Skip exactly one space after the #'s
    if text.as_bytes().get(content_start - byte_range.start) == Some(&b' ') {
        content_start += 1;
    }

    // Content ends at the end of the heading line, but exclude any trailing newline
    let mut content_end = byte_range.end;
    if text.ends_with('\n') {
        content_end -= 1;
    }

    content_start..content_end
}

/// Extract content range for a paragraph (excluding trailing newlines)
fn extract_paragraph_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Content starts at the beginning of the paragraph
    let content_start = byte_range.start;

    // Content ends at the end, but exclude any trailing newlines
    let mut content_end = byte_range.end;
    if text.ends_with('\n') {
        content_end -= 1;
        // Also check for \r\n (though we're focusing on LF for now)
        if text.len() > 1 && text.as_bytes()[text.len() - 2] == b'\r' {
            content_end -= 1;
        }
    }

    content_start..content_end
}

/// Extract list marker from a list item node
fn extract_list_marker(doc: &Document, node: &tree_sitter::Node) -> Marker {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range);

    // Find the marker in the text
    let trimmed = text.trim_start();

    if trimmed.starts_with("- ") {
        Marker::Dash
    } else if trimmed.starts_with("* ") {
        Marker::Asterisk
    } else if trimmed.starts_with("+ ") {
        Marker::Plus
    } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        // Numbered list (1., 2., etc.)
        Marker::Numbered
    } else {
        // Default to dash if we can't determine
        Marker::Dash
    }
}

/// Calculate the depth of a list item based on indentation
fn calculate_list_depth(doc: &Document, node: &tree_sitter::Node) -> usize {
    // Get the start of the line this list item is on
    let start_byte = node.start_byte();

    // Find the beginning of the line
    let full_text = doc.slice_to_cow(0..doc.len());
    let line_start = full_text[..start_byte]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let line_text = &full_text[line_start..];

    // Extract just the indentation part
    let indent_str = line_text
        .chars()
        .take_while(|&c| c == ' ' || c == '\t')
        .collect::<String>();

    // Use the document's indent style to calculate depth
    doc.indent_style.calculate_depth(&indent_str)
}

/// Extract the byte range for just the list item's own line (excluding children)
fn extract_list_item_own_range(doc: &Document, node: &tree_sitter::Node) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find the end of the first line (list item's own content)
    let line_end = if let Some(newline_pos) = text.find('\n') {
        byte_range.start + newline_pos
    } else {
        byte_range.end
    };

    byte_range.start..line_end
}

/// Calculate the same range as the existing buggy anchor creation logic
/// IMPORTANT: This replicates the bug in anchors.rs::calculate_list_item_own_range
/// where it returns full_range instead of properly handling newlines as the comment describes
fn calculate_list_item_anchor_range(node: &tree_sitter::Node) -> std::ops::Range<usize> {
    let full_range = node.byte_range();
    // For a list_item, find the first child list (if any) and stop there
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "list" {
            // The list item's own content ends where the child list begins
            return full_range.start..child.byte_range().start;
        }
    }
    // When no child list is found, return the full range of the list item node
    // Both anchor generation and snapshot creation use this consistent approach
    full_range
}

/// Extract content range for a list item (after the marker and space)
fn extract_list_item_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find the start of content (after indentation and marker)
    let trimmed = text.trim_start();
    let indent_len = text.len() - trimmed.len();

    let mut marker_len = 0;
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        marker_len = 2; // "- " or "* " or "+ "
    } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        // Find the numbered marker like "1. "
        if let Some(dot_pos) = trimmed.find(". ") {
            marker_len = dot_pos + 2; // "N. "
        }
    }

    let content_start = byte_range.start + indent_len + marker_len;

    // For list items, the content should only be the text on the first line,
    // not including nested content
    let first_line_text = &text[indent_len + marker_len..];
    let content_end = if let Some(newline_pos) = first_line_text.find('\n') {
        content_start + newline_pos
    } else {
        byte_range.end
    };

    content_start..content_end
}

/// Extract language from a fenced code block
fn extract_code_fence_language(doc: &Document, node: &tree_sitter::Node) -> Option<String> {
    // Look for the info string on the first line
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range);

    if let Some(first_line_end) = text.find('\n') {
        let first_line = &text[..first_line_end];

        // Remove the fence markers (``` or ~~~) and get the language
        let lang_part = first_line
            .trim_start_matches('`')
            .trim_start_matches('~')
            .trim();

        if lang_part.is_empty() {
            None
        } else {
            Some(lang_part.to_string())
        }
    } else {
        None
    }
}

/// Extract content range for a fenced code block (the code inside)
fn extract_code_fence_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find the end of the first line (opening fence)
    let content_start = if let Some(first_newline) = text.find('\n') {
        byte_range.start + first_newline + 1
    } else {
        byte_range.start
    };

    // Find the start of the last line (closing fence)
    let content_end = if let Some(last_newline) = text.rfind('\n') {
        // Check if there's a closing fence
        let potential_close = &text[last_newline + 1..];
        if potential_close.trim_start().starts_with("```")
            || potential_close.trim_start().starts_with("~~~")
        {
            byte_range.start + last_newline
        } else {
            byte_range.end
        }
    } else {
        byte_range.end
    };

    content_start..content_end
}

/// Parse text content and extract wiki-links and URLs, returning segments
fn parse_text_segments(text: &str) -> Vec<TextSegment> {
    use std::sync::OnceLock;

    // Regex pattern for HTTP/HTTPS URLs
    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    let url_regex =
        URL_REGEX.get_or_init(|| Regex::new(r"https?://[^\s<>\[\]]+").expect("Invalid URL regex"));

    let mut segments = Vec::new();
    let mut current_pos = 0;

    // Find all patterns (wiki-links and URLs) and their positions
    let mut patterns: Vec<(usize, usize, PatternType)> = Vec::new();

    // Find wiki-links [[...]]
    let mut wiki_start = 0;
    while let Some(start) = text[wiki_start..].find("[[") {
        let absolute_start = wiki_start + start;
        if let Some(end) = text[absolute_start + 2..].find("]]") {
            let absolute_end = absolute_start + 2 + end + 2; // Include the closing ]]
            patterns.push((absolute_start, absolute_end, PatternType::WikiLink));
            wiki_start = absolute_end;
        } else {
            // No closing ]], treat [[ as regular text (will be processed separately)
            patterns.push((
                absolute_start,
                absolute_start + 2,
                PatternType::UnmatchedWikiStart,
            ));
            wiki_start = absolute_start + 2;
        }
    }

    // Find URLs and clean up trailing punctuation
    for url_match in url_regex.find_iter(text) {
        let start = url_match.start();
        let mut end = url_match.end();

        // Remove trailing punctuation that's typically not part of URLs
        let url_text = &text[start..end];
        while let Some(last_char) = url_text[..(end - start)].chars().last() {
            if matches!(
                last_char,
                '.' | ',' | ':' | ';' | '!' | '?' | ')' | ']' | '}'
            ) {
                end -= last_char.len_utf8();
            } else {
                break;
            }
        }

        if end > start {
            patterns.push((start, end, PatternType::Url));
        }
    }

    // Sort patterns by starting position
    patterns.sort_by_key(|&(start, _, _)| start);

    // Remove overlapping patterns (prefer wiki-links over URLs when they overlap)
    let mut filtered_patterns = Vec::new();
    for &(start, end, pattern_type) in &patterns {
        let overlaps = filtered_patterns.iter().any(|&(prev_start, prev_end, _)| {
            // Check if current pattern overlaps with any previous pattern
            start < prev_end && end > prev_start
        });

        if !overlaps {
            filtered_patterns.push((start, end, pattern_type));
        }
    }

    // Process the text with patterns
    for &(start, end, pattern_type) in &filtered_patterns {
        // Add text before the pattern
        if current_pos < start {
            let text_segment = text[current_pos..start].to_string();
            if !text_segment.is_empty() {
                segments.push(TextSegment::Text(text_segment));
            }
        }

        // Add the pattern segment
        match pattern_type {
            PatternType::WikiLink => {
                let link_content = &text[start + 2..end - 2]; // Remove [[ and ]]
                let target = link_content.trim().to_string();
                segments.push(TextSegment::WikiLink { target });
            }
            PatternType::Url => {
                let href = text[start..end].to_string();
                segments.push(TextSegment::Url { href });
            }
            PatternType::UnmatchedWikiStart => {
                // Add [[ as regular text
                segments.push(TextSegment::Text("[[".to_string()));
            }
        }

        current_pos = end;
    }

    // Add any remaining text
    if current_pos < text.len() {
        let remaining = text[current_pos..].to_string();
        if !remaining.is_empty() {
            segments.push(TextSegment::Text(remaining));
        }
    }

    segments
}

#[derive(Copy, Clone, Debug)]
enum PatternType {
    WikiLink,
    Url,
    UnmatchedWikiStart,
}

/// Groups consecutive list items into proper nested HTML structure
/// This is the core data layer function that should handle grouping, not the UI
fn group_blocks_for_rendering(blocks: &[RenderBlock]) -> Vec<ContentGroup> {
    let mut groups = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        let block = &blocks[i];

        match &block.kind {
            BlockKind::ListItem { marker, .. } => {
                // Start a new list group - collect all consecutive list items
                let list_start = i;
                let first_marker = marker.clone();
                while i < blocks.len() {
                    if let BlockKind::ListItem { marker, .. } = &blocks[i].kind {
                        // Only group items with the same marker type (numbered vs bullet)
                        if is_same_list_type(&first_marker, marker) {
                            i += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Group the list items into a nested structure
                let list_blocks = &blocks[list_start..i];
                let list_items = build_nested_list_structure(list_blocks);

                // Create the appropriate list group based on marker type
                let group = match first_marker {
                    Marker::Numbered => ContentGroup::NumberedListGroup { items: list_items },
                    _ => ContentGroup::BulletListGroup { items: list_items },
                };

                groups.push(group);
            }
            _ => {
                // Single non-list block (including UnhandledMarkdown)
                groups.push(ContentGroup::SingleBlock(block.clone()));
                i += 1;
            }
        }
    }

    groups
}

/// Check if two markers represent the same list type (numbered vs bullet)
fn is_same_list_type(marker1: &Marker, marker2: &Marker) -> bool {
    matches!(
        (marker1, marker2),
        (Marker::Numbered, Marker::Numbered)
            | (
                Marker::Dash | Marker::Asterisk | Marker::Plus,
                Marker::Dash | Marker::Asterisk | Marker::Plus,
            )
    )
}

/// Builds a nested list structure from flat list blocks
fn build_nested_list_structure(blocks: &[RenderBlock]) -> Vec<ListItem> {
    let mut result = Vec::new();

    for block in blocks {
        if let BlockKind::ListItem { .. } = &block.kind {
            let item_depth = block.depth; // Use RenderBlock.depth, not BlockKind depth

            let new_item = ListItem {
                block: block.clone(),
                children: Vec::new(),
            };

            // Insert at the appropriate nesting level
            insert_list_item_at_depth(&mut result, new_item, item_depth);
        }
    }

    result
}

/// Helper function to insert a list item at the correct depth
fn insert_list_item_at_depth(items: &mut Vec<ListItem>, new_item: ListItem, target_depth: usize) {
    if target_depth == 0 {
        // Insert at root level
        items.push(new_item);
    } else if let Some(last_item) = items.last_mut() {
        // Try to insert as a child of the last item at the previous depth
        insert_list_item_at_depth(&mut last_item.children, new_item, target_depth - 1);
    } else {
        // No parent exists, insert at root level anyway (fallback)
        items.push(new_item);
    }
}

/// Find existing anchor for a tree-sitter node
/// Uses node ID first for identity preservation, then falls back to range matching
fn find_existing_anchor_for_node(
    doc: &Document,
    node: &tree_sitter::Node,
    range: &std::ops::Range<usize>,
) -> AnchorId {
    let node_id = node.id();

    // First try: Find anchor by node ID (preserves identity across edits)
    for anchor in &doc.anchors {
        if let Some(anchor_node_id) = anchor.node_id
            && anchor_node_id == node_id
        {
            // Found anchor by node ID - return the same ID even if range changed
            return anchor.id;
        }
    }

    // Second try: Find anchor by exact range (for newly created nodes)
    for anchor in &doc.anchors {
        if anchor.range == *range {
            return anchor.id;
        }
    }

    // Third try: Position matching with validation to catch range drift bugs
    for anchor in &doc.anchors {
        if anchor.range.start == range.start {
            if anchor.range.end != range.end {
                // Range drift detected - this indicates a bug in range calculation consistency
                // ARCHITECTURAL ISSUE: Tree-sitter node ranges are not stable across incremental parsing.
                // Even when logical content is unchanged, node boundaries can shift when document structure changes.
                // This is expected behavior, not a bug. The position-based fallback handles this correctly.
                // Long-term solution: Don't rely on tree-sitter ranges for anchor identity at all.
            }
            return anchor.id;
        }
    }

    // No existing anchor found - create a stable ID based on node characteristics
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    node_id.hash(&mut hasher);
    range.start.hash(&mut hasher);
    range.end.hash(&mut hasher);
    AnchorId(hasher.finish() as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{Document, commands::Cmd};

    #[test]
    fn test_calculate_list_depth_with_detected_indent() {
        let mut doc =
            Document::from_bytes(b"- item 1\n  - 2-space nested\n    - 4-space nested\n").unwrap();
        doc.create_anchors_from_tree();

        let tree = doc.tree.as_ref().unwrap();
        let root = tree.root_node();

        // Find the list items manually by iterating through children
        let mut list_items = Vec::new();

        fn find_list_items<'a>(
            node: tree_sitter::Node<'a>,
            items: &mut Vec<tree_sitter::Node<'a>>,
        ) {
            if node.kind() == "list_item" {
                items.push(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_list_items(child, items);
            }
        }

        find_list_items(root, &mut list_items);

        assert!(list_items.len() >= 3, "Should have at least 3 list items");

        // With 2-space detection, depths should be 0, 1, 2
        assert_eq!(
            calculate_list_depth(&doc, &list_items[0]),
            0,
            "First item should be depth 0"
        );
        assert_eq!(
            calculate_list_depth(&doc, &list_items[1]),
            1,
            "Second item should be depth 1 (2 spaces)"
        );
        assert_eq!(
            calculate_list_depth(&doc, &list_items[2]),
            2,
            "Third item should be depth 2 (4 spaces)"
        );
    }

    #[test]
    fn test_nested_list_items_get_correct_anchor_ids() {
        // Test that nested list items get their own anchor IDs, not their parent's
        let mut doc = Document::from_bytes(
            b"- parent item\n  - child item 1\n  - child item 2\n    - grandchild item\n",
        )
        .unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Verify blocks have correct depth values
        assert_eq!(snapshot.blocks.len(), 4);
        assert_eq!(snapshot.blocks[0].depth, 0, "parent item should be depth 0");
        assert_eq!(
            snapshot.blocks[1].depth, 1,
            "child item 1 should be depth 1"
        );
        assert_eq!(
            snapshot.blocks[2].depth, 1,
            "child item 2 should be depth 1"
        );
        assert_eq!(
            snapshot.blocks[3].depth, 2,
            "grandchild item should be depth 2"
        );

        // Find the list group
        let list_group = snapshot
            .content_groups
            .iter()
            .find_map(|g| {
                if let ContentGroup::BulletListGroup { items } = g {
                    Some(items)
                } else {
                    None
                }
            })
            .expect("Should have a bullet list group");

        // The parent item should have its own unique anchor ID
        assert_eq!(list_group.len(), 1, "Should have one top-level item");
        let parent_item = &list_group[0];
        let parent_id = parent_item.block.id;

        // Verify nesting structure
        assert_eq!(
            parent_item.children.len(),
            2,
            "Parent should have 2 children"
        );
        assert_eq!(
            parent_item.children[1].children.len(),
            1,
            "Second child should have 1 grandchild"
        );

        // Collect all anchor IDs to check uniqueness
        let mut all_ids = vec![parent_id];

        // Child items should have different anchor IDs from parent
        for child in &parent_item.children {
            assert_ne!(
                child.block.id, parent_id,
                "Child '{}' should not have same anchor ID as parent '{}'",
                child.block.content, parent_item.block.content
            );
            all_ids.push(child.block.id);

            // Grandchildren should also have unique IDs
            for grandchild in &child.children {
                assert_ne!(
                    grandchild.block.id, parent_id,
                    "Grandchild '{}' should not have parent's anchor ID",
                    grandchild.block.content
                );
                assert_ne!(
                    grandchild.block.id, child.block.id,
                    "Grandchild '{}' should not have its parent's anchor ID",
                    grandchild.block.content
                );
                all_ids.push(grandchild.block.id);
            }
        }

        // Verify all IDs are unique
        let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
        assert_eq!(
            unique_ids.len(),
            all_ids.len(),
            "All anchor IDs should be unique"
        );
    }

    #[test]
    fn test_nested_list_anchor_uniqueness() {
        let mut doc = Document::from_bytes(
            b"- asdf\n  - asdf\n  - indented 1\n    - indented 1.1 hoooooray\n    - indented 1.2\n      - indented 1.2.1\n        - indented 1.2.1.1 yay fixed\n    - indented 1.3\n- indented 2 well\n  - indented 2.1\n  - indented 2.2\n"
        ).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Collect all list item blocks (including nested ones)
        let mut all_list_blocks = Vec::new();
        fn collect_list_blocks<'a>(items: &'a [ListItem], blocks: &mut Vec<&'a RenderBlock>) {
            for item in items {
                blocks.push(&item.block);
                collect_list_blocks(&item.children, blocks);
            }
        }

        for group in &snapshot.content_groups {
            if let ContentGroup::BulletListGroup { items } = group {
                collect_list_blocks(items, &mut all_list_blocks);
            }
        }

        // Check that all anchor IDs are unique
        let anchor_ids: Vec<_> = all_list_blocks.iter().map(|block| block.id).collect();
        let unique_ids: std::collections::HashSet<_> = anchor_ids.iter().collect();

        assert_eq!(
            anchor_ids.len(),
            unique_ids.len(),
            "All list items should have unique anchor IDs. Found {} total, {} unique",
            anchor_ids.len(),
            unique_ids.len()
        );
    }

    // ============ Snapshot API tests ============

    #[test]
    fn test_snapshot_empty_document() {
        let doc = Document::from_bytes(b"").unwrap();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.version, 0);
        assert_eq!(snapshot.blocks.len(), 0);
        assert_eq!(snapshot.content_groups.len(), 0);
    }

    #[test]
    fn test_snapshot_simple_heading() {
        let mut doc = Document::from_bytes(b"# Hello World").unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.version, 0);
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::Heading { level: 1 });
        assert_eq!(block.byte_range, 0..13);
        assert_eq!(block.content_range, 2..13); // After "# " prefix
        assert_eq!(block.depth, 0);
    }

    #[test]
    fn test_snapshot_multiple_headings() {
        let text = "# Heading 1\n\n## Heading 2\n\n### Heading 3";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 3);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::Heading { level: 1 }
        ));
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::Heading { level: 2 }
        ));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::Heading { level: 3 }
        ));

        // Check content ranges exclude the markdown prefixes
        assert_eq!(snapshot.blocks[0].content_range, 2..11); // After "# "
        assert_eq!(snapshot.blocks[1].content_range, 16..25); // After "## "
        assert_eq!(snapshot.blocks[2].content_range, 31..40); // After "### "
    }

    #[test]
    fn test_snapshot_simple_list() {
        let text = "- Item 1\n- Item 2\n- Item 3";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 3);

        for block in &snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0
                }
            ));
            assert_eq!(block.depth, 0);
        }

        // Content ranges should exclude structural elements like newlines
        assert_eq!(snapshot.blocks[0].content_range, 2..8); // "Item 1" (excluding newline)
        assert_eq!(snapshot.blocks[1].content_range, 11..17); // "Item 2" (excluding newline)
        assert_eq!(snapshot.blocks[2].content_range, 20..26); // "Item 3" (no trailing newline)
    }

    #[test]
    fn test_snapshot_nested_list() {
        let text = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // We should have exactly 4 list items now that we filter out paragraph nodes
        assert_eq!(snapshot.blocks.len(), 4);

        // All blocks should be list items with dash markers
        for block in &snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: _
                }
            ));
        }

        // Verify that parent list item's byte_range doesn't include children
        let parent_item = &snapshot.blocks[0];
        assert_eq!(parent_item.content, "Item 1");
        assert_eq!(parent_item.byte_range, 0..8); // Just "- Item 1" without the newline
        assert_eq!(doc.slice_to_cow(parent_item.byte_range.clone()), "- Item 1");

        // Verify nested items have their own ranges
        let nested_item1 = &snapshot.blocks[1];
        assert_eq!(nested_item1.content, "Nested 1");
        assert_eq!(nested_item1.byte_range, 11..21); // "  - Nested 1" line (note: starts at 9 + 2 spaces)
        assert_eq!(
            doc.slice_to_cow(nested_item1.byte_range.clone()),
            "- Nested 1"
        );

        let nested_item2 = &snapshot.blocks[2];
        assert_eq!(nested_item2.content, "Nested 2");
        assert_eq!(nested_item2.byte_range, 24..34); // "  - Nested 2" line
        assert_eq!(
            doc.slice_to_cow(nested_item2.byte_range.clone()),
            "- Nested 2"
        );
    }

    #[test]
    fn test_snapshot_different_list_markers() {
        let text = "- Dash item\n* Star item\n+ Plus item\n1. Numbered item";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 4);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::ListItem {
                marker: Marker::Asterisk,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::ListItem {
                marker: Marker::Plus,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[3].kind,
            BlockKind::ListItem {
                marker: Marker::Numbered,
                depth: 0
            }
        ));
    }

    #[test]
    fn test_snapshot_mixed_content() {
        let text = "# Main Heading\n\nThis is a paragraph.\n\n- List item 1\n- List item 2\n\n## Sub Heading";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Should have: heading, paragraph, 2 list items, heading
        assert_eq!(snapshot.blocks.len(), 5);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::Heading { level: 1 }
        ));
        assert!(matches!(snapshot.blocks[1].kind, BlockKind::Paragraph));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[3].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[4].kind,
            BlockKind::Heading { level: 2 }
        ));
    }

    #[test]
    fn test_snapshot_code_fences() {
        let text = "```rust\nfn main() {}\n```\n\n```\nplain code\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 2);

        // First code fence with language
        assert!(
            matches!(snapshot.blocks[0].kind, BlockKind::CodeFence { lang: Some(ref lang) } if lang == "rust")
        );

        // Second code fence without language
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::CodeFence { lang: None }
        ));
    }

    #[test]
    fn test_snapshot_anchor_association() {
        let text = "# Heading\n\n- Item 1\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Due to granular parsing, we might get more blocks than expected
        assert!(snapshot.blocks.len() >= 3);

        // Each block should have a unique anchor ID
        let mut ids = std::collections::HashSet::new();
        for block in &snapshot.blocks {
            assert!(
                ids.insert(block.id),
                "Each block should have a unique anchor ID"
            );
        }

        // Every document anchor ID should appear in the blocks
        // (though blocks may have additional temporary IDs for paragraphs etc.)
        let doc_anchor_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();
        let block_anchor_ids: std::collections::HashSet<AnchorId> =
            snapshot.blocks.iter().map(|b| b.id).collect();

        for doc_anchor_id in &doc_anchor_ids {
            assert!(
                block_anchor_ids.contains(doc_anchor_id),
                "Document anchor ID {doc_anchor_id:?} should appear in blocks"
            );
        }
    }

    #[test]
    fn test_snapshot_version_tracking() {
        let mut doc = Document::from_bytes(b"# Test").unwrap();
        doc.create_anchors_from_tree();

        let initial_snapshot = doc.snapshot();
        assert_eq!(initial_snapshot.version, 0);

        // Make an edit
        doc.apply(Cmd::InsertText {
            at: 6,
            text: " Document".to_string(),
        });

        let updated_snapshot = doc.snapshot();
        assert_eq!(updated_snapshot.version, 1);
    }

    #[test]
    fn test_snapshot_after_edits() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();
        doc.create_anchors_from_tree();

        // Initial snapshot
        let initial_snapshot = doc.snapshot();
        assert_eq!(initial_snapshot.blocks.len(), 1);

        // Add a new list item
        doc.apply(Cmd::SplitListItem { at: 8 });
        doc.apply(Cmd::InsertText {
            at: 11,
            text: "Item 2".to_string(),
        });

        let updated_snapshot = doc.snapshot();
        assert_eq!(updated_snapshot.blocks.len(), 2);
        assert_eq!(updated_snapshot.version, 2);

        // Both should be list items
        for block in &updated_snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0
                }
            ));
        }
    }

    #[test]
    fn test_snapshot_content_ranges_after_edit() {
        let mut doc = Document::from_bytes(b"# Heading").unwrap();
        doc.create_anchors_from_tree();

        // Add text to the heading
        doc.apply(Cmd::InsertText {
            at: 9,
            text: " Extended".to_string(),
        });

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
        assert_eq!(block.content_range, 2..18); // Should include the extended text
        assert_eq!(&doc.text()[block.content_range.clone()], "Heading Extended");
    }

    // ============ Content Grouping tests ============

    #[test]
    fn test_group_blocks_simple_bullet_list() {
        // Create mock blocks for a simple bullet list
        let blocks = vec![
            RenderBlock {
                id: AnchorId(1),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 0..8,
                content_range: 2..8,
                depth: 0,
                content: "Item 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(2),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 9..17,
                content_range: 11..17,
                depth: 0,
                content: "Item 2".to_string(),
                segments: None,
            },
        ];

        let groups = group_blocks_for_rendering(&blocks);

        assert_eq!(groups.len(), 1);
        match &groups[0] {
            ContentGroup::BulletListGroup { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].block.content, "Item 1");
                assert_eq!(items[1].block.content, "Item 2");
                assert!(items[0].children.is_empty());
                assert!(items[1].children.is_empty());
            }
            _ => panic!("Expected BulletListGroup"),
        }
    }

    #[test]
    fn test_group_blocks_simple_numbered_list() {
        // Create mock blocks for a simple numbered list
        let blocks = vec![
            RenderBlock {
                id: AnchorId(1),
                kind: BlockKind::ListItem {
                    marker: Marker::Numbered,
                    depth: 0,
                },
                byte_range: 0..8,
                content_range: 3..8,
                depth: 0,
                content: "Item 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(2),
                kind: BlockKind::ListItem {
                    marker: Marker::Numbered,
                    depth: 0,
                },
                byte_range: 9..17,
                content_range: 12..17,
                depth: 0,
                content: "Item 2".to_string(),
                segments: None,
            },
        ];

        let groups = group_blocks_for_rendering(&blocks);

        assert_eq!(groups.len(), 1);
        match &groups[0] {
            ContentGroup::NumberedListGroup { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].block.content, "Item 1");
                assert_eq!(items[1].block.content, "Item 2");
                assert!(items[0].children.is_empty());
                assert!(items[1].children.is_empty());
            }
            _ => panic!("Expected NumberedListGroup"),
        }
    }

    #[test]
    fn test_group_blocks_mixed_list_types() {
        // Create mock blocks with bullet list followed by numbered list
        let blocks = vec![
            RenderBlock {
                id: AnchorId(1),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 0..8,
                content_range: 2..8,
                depth: 0,
                content: "Bullet 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(2),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 9..17,
                content_range: 11..17,
                depth: 0,
                content: "Bullet 2".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(3),
                kind: BlockKind::ListItem {
                    marker: Marker::Numbered,
                    depth: 0,
                },
                byte_range: 18..26,
                content_range: 21..26,
                depth: 0,
                content: "Number 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(4),
                kind: BlockKind::ListItem {
                    marker: Marker::Numbered,
                    depth: 0,
                },
                byte_range: 27..35,
                content_range: 30..35,
                depth: 0,
                content: "Number 2".to_string(),
                segments: None,
            },
        ];

        let groups = group_blocks_for_rendering(&blocks);

        assert_eq!(groups.len(), 2);

        // First group: Bullet list
        match &groups[0] {
            ContentGroup::BulletListGroup { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].block.content, "Bullet 1");
                assert_eq!(items[1].block.content, "Bullet 2");
            }
            _ => panic!("Expected BulletListGroup"),
        }

        // Second group: Numbered list
        match &groups[1] {
            ContentGroup::NumberedListGroup { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].block.content, "Number 1");
                assert_eq!(items[1].block.content, "Number 2");
            }
            _ => panic!("Expected NumberedListGroup"),
        }
    }

    #[test]
    fn test_group_blocks_nested_list() {
        // Create mock blocks for nested list
        let blocks = vec![
            RenderBlock {
                id: AnchorId(1),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 0..8,
                content_range: 2..8,
                depth: 0,
                content: "Item 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(2),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 1,
                },
                byte_range: 9..19,
                content_range: 13..19,
                depth: 1,
                content: "Nested 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(3),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 1,
                },
                byte_range: 20..30,
                content_range: 24..30,
                depth: 1,
                content: "Nested 2".to_string(),
                segments: None,
            },
        ];

        let groups = group_blocks_for_rendering(&blocks);

        assert_eq!(groups.len(), 1);
        match &groups[0] {
            ContentGroup::BulletListGroup { items } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].block.content, "Item 1");
                assert_eq!(items[0].children.len(), 2);
                assert_eq!(items[0].children[0].block.content, "Nested 1");
                assert_eq!(items[0].children[1].block.content, "Nested 2");
            }
            _ => panic!("Expected BulletListGroup"),
        }
    }

    #[test]
    fn test_group_blocks_mixed_content() {
        // Create mock blocks with mixed content types
        let blocks = vec![
            RenderBlock {
                id: AnchorId(1),
                kind: BlockKind::Heading { level: 1 },
                byte_range: 0..10,
                content_range: 2..10,
                depth: 0,
                content: "Heading".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(2),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 11..19,
                content_range: 13..19,
                depth: 0,
                content: "Item 1".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(3),
                kind: BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0,
                },
                byte_range: 20..28,
                content_range: 22..28,
                depth: 0,
                content: "Item 2".to_string(),
                segments: None,
            },
            RenderBlock {
                id: AnchorId(4),
                kind: BlockKind::Paragraph,
                byte_range: 29..39,
                content_range: 29..39,
                depth: 0,
                content: "Paragraph".to_string(),
                segments: None,
            },
        ];

        let groups = group_blocks_for_rendering(&blocks);

        assert_eq!(groups.len(), 3);

        // First group: Heading
        match &groups[0] {
            ContentGroup::SingleBlock(block) => {
                assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
                assert_eq!(block.content, "Heading");
            }
            _ => panic!("Expected SingleBlock"),
        }

        // Second group: List with two items
        match &groups[1] {
            ContentGroup::BulletListGroup { items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].block.content, "Item 1");
                assert_eq!(items[1].block.content, "Item 2");
            }
            _ => panic!("Expected BulletListGroup"),
        }

        // Third group: Paragraph
        match &groups[2] {
            ContentGroup::SingleBlock(block) => {
                assert!(matches!(block.kind, BlockKind::Paragraph));
                assert_eq!(block.content, "Paragraph");
            }
            _ => panic!("Expected SingleBlock"),
        }
    }

    #[test]
    fn test_end_to_end_bullet_list_grouping() {
        // Test the full pipeline: markdown text -> Document -> Snapshot -> grouped structure
        let markdown_text = r#"
- indented 1
    - indented 1.1
    - indented 1.2
- indented 2
    - indented 2.1
    - indented 2.2"#;

        // Parse markdown into Document
        let mut doc =
            Document::from_bytes(markdown_text.as_bytes()).expect("Should parse markdown");
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have 1 group (the bullet list group)
        assert_eq!(
            snapshot.content_groups.len(),
            1,
            "Should have 1 content group"
        );

        match &snapshot.content_groups[0] {
            ContentGroup::BulletListGroup { items: list_items } => {
                // Should have 2 top-level items
                assert_eq!(list_items.len(), 2, "Should have exactly 2 top-level items");

                // First item: "indented 1" with 2 children
                assert_eq!(list_items[0].block.content, "indented 1");
                assert_eq!(
                    list_items[0].children.len(),
                    2,
                    "indented 1 should have 2 children"
                );
                assert_eq!(list_items[0].children[0].block.content, "indented 1.1");
                assert_eq!(list_items[0].children[1].block.content, "indented 1.2");

                // Second item: "indented 2" with 2 children
                assert_eq!(list_items[1].block.content, "indented 2");
                assert_eq!(
                    list_items[1].children.len(),
                    2,
                    "indented 2 should have 2 children"
                );
                assert_eq!(list_items[1].children[0].block.content, "indented 2.1");
                assert_eq!(list_items[1].children[1].block.content, "indented 2.2");

                // Verify depths are correct
                assert_eq!(list_items[0].block.depth, 0, "indented 1 should be depth 0");
                assert_eq!(
                    list_items[0].children[0].block.depth, 1,
                    "indented 1.1 should be depth 1"
                );
                assert_eq!(
                    list_items[0].children[1].block.depth, 1,
                    "indented 1.2 should be depth 1"
                );
                assert_eq!(list_items[1].block.depth, 0, "indented 2 should be depth 0");
                assert_eq!(
                    list_items[1].children[0].block.depth, 1,
                    "indented 2.1 should be depth 1"
                );
                assert_eq!(
                    list_items[1].children[1].block.depth, 1,
                    "indented 2.2 should be depth 1"
                );
            }
            _ => panic!(
                "Expected BulletListGroup, got {:?}",
                snapshot.content_groups[0]
            ),
        }
    }

    #[test]
    fn test_end_to_end_numbered_list_grouping() {
        // Test the full pipeline: markdown text -> Document -> Snapshot -> grouped structure for numbered lists
        let markdown_text = r#"
1. First item
    1. Nested first
    2. Nested second  
2. Second item
    1. Another nested first
    2. Another nested second"#;

        // Parse markdown into Document
        let mut doc =
            Document::from_bytes(markdown_text.as_bytes()).expect("Should parse markdown");
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have 1 group (the numbered list group)
        assert_eq!(
            snapshot.content_groups.len(),
            1,
            "Should have 1 content group"
        );

        match &snapshot.content_groups[0] {
            ContentGroup::NumberedListGroup { items: list_items } => {
                // Should have 2 top-level items
                assert_eq!(list_items.len(), 2, "Should have exactly 2 top-level items");

                // First item: "First item" with 2 children
                assert_eq!(list_items[0].block.content, "First item");
                assert_eq!(
                    list_items[0].children.len(),
                    2,
                    "First item should have 2 children"
                );
                assert_eq!(list_items[0].children[0].block.content, "Nested first");
                assert_eq!(list_items[0].children[1].block.content, "Nested second");

                // Second item: "Second item" with 2 children
                assert_eq!(list_items[1].block.content, "Second item");
                assert_eq!(
                    list_items[1].children.len(),
                    2,
                    "Second item should have 2 children"
                );
                assert_eq!(
                    list_items[1].children[0].block.content,
                    "Another nested first"
                );
                assert_eq!(
                    list_items[1].children[1].block.content,
                    "Another nested second"
                );
            }
            _ => panic!(
                "Expected NumberedListGroup, got {:?}",
                snapshot.content_groups[0]
            ),
        }
    }

    #[test]
    fn test_is_same_list_type() {
        // Test numbered lists
        assert!(is_same_list_type(&Marker::Numbered, &Marker::Numbered));

        // Test bullet lists
        assert!(is_same_list_type(&Marker::Dash, &Marker::Dash));
        assert!(is_same_list_type(&Marker::Dash, &Marker::Asterisk));
        assert!(is_same_list_type(&Marker::Asterisk, &Marker::Plus));
        assert!(is_same_list_type(&Marker::Plus, &Marker::Dash));

        // Test different types
        assert!(!is_same_list_type(&Marker::Numbered, &Marker::Dash));
        assert!(!is_same_list_type(&Marker::Dash, &Marker::Numbered));
        assert!(!is_same_list_type(&Marker::Asterisk, &Marker::Numbered));
        assert!(!is_same_list_type(&Marker::Plus, &Marker::Numbered));
    }

    #[test]
    fn test_snapshot_after_replace_range_stale_tree() {
        // Reproduce the xi-rope panic when tree-sitter has stale ranges
        let initial_text = "- item 1\n- item 2";
        let mut doc = Document::from_bytes(initial_text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        // Initial snapshot should work fine
        let snapshot1 = doc.snapshot();
        assert!(!snapshot1.blocks.is_empty());

        // Apply a ReplaceRange command to modify text - make longer replacement
        let _patch = doc.apply(Cmd::ReplaceRange {
            range: 0..8, // Replace "- item 1" with longer text
            text: "- this is a much longer item 1".to_string(),
        });

        // This snapshot creation should trigger the xi-rope panic
        // because tree-sitter nodes have stale byte ranges
        let snapshot2 = doc.snapshot();
        assert!(!snapshot2.blocks.is_empty());
    }

    // ============ Text Segment Parsing tests ============

    #[test]
    fn test_parse_text_segments_plain_text() {
        let text = "This is plain text without links";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0],
            TextSegment::Text("This is plain text without links".to_string())
        );
    }

    #[test]
    fn test_parse_text_segments_single_link() {
        let text = "Check out [[Page Name]] for details";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Check out ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Page Name".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for details".to_string()));
    }

    #[test]
    fn test_parse_text_segments_multiple_links() {
        let text = "See [[First Page]] and [[Second Page]] and [[Third Page]]";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 6);
        assert_eq!(segments[0], TextSegment::Text("See ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "First Page".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::WikiLink {
                target: "Second Page".to_string()
            }
        );
        assert_eq!(segments[4], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[5],
            TextSegment::WikiLink {
                target: "Third Page".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_whitespace_trimming() {
        let text = "Link: [[  Spaced Out  ]] text";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Link: ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Spaced Out".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" text".to_string()));
    }

    #[test]
    fn test_parse_text_segments_unclosed_bracket() {
        let text = "Incomplete [[link without closing";
        let segments = parse_text_segments(text);

        // Should treat [[ as regular text when there's no closing ]]
        // The parse function will add "[[" as text, then continue with the rest
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Incomplete ".to_string()));
        assert_eq!(segments[1], TextSegment::Text("[[".to_string()));
        assert_eq!(
            segments[2],
            TextSegment::Text("link without closing".to_string())
        );
    }

    #[test]
    fn test_parse_text_segments_empty_link() {
        let text = "Empty [[]] link";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Empty ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" link".to_string()));
    }

    #[test]
    fn test_parse_text_segments_adjacent_links() {
        let text = "[[First]][[Second]] adjacent";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(
            segments[0],
            TextSegment::WikiLink {
                target: "First".to_string()
            }
        );
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Second".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" adjacent".to_string()));
    }

    #[test]
    fn test_parse_text_segments_complex_target_names() {
        let text = "See [[Notes/2023/Daily Journal]] and [[Project: New Feature]]";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], TextSegment::Text("See ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Notes/2023/Daily Journal".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::WikiLink {
                target: "Project: New Feature".to_string()
            }
        );
    }

    #[test]
    fn test_snapshot_with_wikilinks_in_heading() {
        let text = "# See [[Other Page]] for more info";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
        assert_eq!(block.content, "See [[Other Page]] for more info");
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("See ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Other Page".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for more info".to_string()));
    }

    #[test]
    fn test_snapshot_with_wikilinks_in_paragraph() {
        let text = "This paragraph links to [[Another Page]] and [[Yet Another]].";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Paragraph));
        assert_eq!(
            block.content,
            "This paragraph links to [[Another Page]] and [[Yet Another]]."
        );
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 5);
        assert_eq!(
            segments[0],
            TextSegment::Text("This paragraph links to ".to_string())
        );
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Another Page".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::WikiLink {
                target: "Yet Another".to_string()
            }
        );
        assert_eq!(segments[4], TextSegment::Text(".".to_string()));
    }

    #[test]
    fn test_snapshot_with_wikilinks_in_list_item() {
        let text = "- Link to [[Referenced Page]] in list item";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(
            block.kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert_eq!(block.content, "Link to [[Referenced Page]] in list item");
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Link to ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Referenced Page".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" in list item".to_string()));
    }

    #[test]
    fn test_snapshot_no_wikilinks_segments_none() {
        let text = "# Plain heading with no links";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
        assert_eq!(block.content, "Plain heading with no links");
        assert!(block.segments.is_none());
    }

    #[test]
    fn test_snapshot_code_blocks_no_wikilink_parsing() {
        let text = "```\nThis [[link]] should not be parsed\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::CodeFence { lang: None }));
        assert!(block.segments.is_none());
    }

    // ============ URL Detection Tests ============

    #[test]
    fn test_parse_text_segments_simple_url() {
        let text = "Check out https://example.com for more info";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Check out ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for more info".to_string()));
    }

    #[test]
    fn test_parse_text_segments_multiple_urls() {
        let text = "Visit https://example.com and http://test.org for details";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0], TextSegment::Text("Visit ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::Url {
                href: "http://test.org".to_string()
            }
        );
        assert_eq!(segments[4], TextSegment::Text(" for details".to_string()));
    }

    #[test]
    fn test_parse_text_segments_mixed_wiki_and_urls() {
        let text = "See [[Internal Page]] and visit https://external.com";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], TextSegment::Text("See ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "Internal Page".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and visit ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::Url {
                href: "https://external.com".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_url_at_start() {
        let text = "https://example.com is a great site";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(
            segments[1],
            TextSegment::Text(" is a great site".to_string())
        );
    }

    #[test]
    fn test_parse_text_segments_url_at_end() {
        let text = "Visit our website at https://example.com";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            TextSegment::Text("Visit our website at ".to_string())
        );
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_url_with_path_and_params() {
        let text = "Go to https://example.com/path?param=value&other=test";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], TextSegment::Text("Go to ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com/path?param=value&other=test".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_no_urls_or_wikilinks() {
        let text = "Just plain text with no special links";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0],
            TextSegment::Text("Just plain text with no special links".to_string())
        );
    }

    #[test]
    fn test_parse_text_segments_adjacent_wiki_and_url() {
        let text = "[[Internal]]https://external.com";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            TextSegment::WikiLink {
                target: "Internal".to_string()
            }
        );
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://external.com".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_http_and_https() {
        let text = "HTTP: http://insecure.com HTTPS: https://secure.com";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], TextSegment::Text("HTTP: ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "http://insecure.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" HTTPS: ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::Url {
                href: "https://secure.com".to_string()
            }
        );
    }

    #[test]
    fn test_parse_text_segments_url_edge_cases() {
        // URL followed by punctuation
        let text = "Visit https://example.com. It's great!";
        let segments = parse_text_segments(text);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Visit ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(". It's great!".to_string()));
    }

    #[test]
    fn test_parse_text_segments_overlapping_patterns() {
        // Wiki link containing what looks like a URL
        let text = "See [[https://example.com documentation]] for details";
        let segments = parse_text_segments(text);

        // Should prefer wiki-link over URL when they overlap
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("See ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::WikiLink {
                target: "https://example.com documentation".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for details".to_string()));
    }

    #[test]
    fn test_snapshot_with_urls_in_heading() {
        let text = "# Check out https://example.com for updates";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
        assert_eq!(block.content, "Check out https://example.com for updates");
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Check out ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for updates".to_string()));
    }

    #[test]
    fn test_snapshot_with_urls_in_paragraph() {
        let text = "This paragraph has https://example.com and https://test.org links.";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Paragraph));
        assert_eq!(
            block.content,
            "This paragraph has https://example.com and https://test.org links."
        );
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 5);
        assert_eq!(
            segments[0],
            TextSegment::Text("This paragraph has ".to_string())
        );
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
        assert_eq!(
            segments[3],
            TextSegment::Url {
                href: "https://test.org".to_string()
            }
        );
        assert_eq!(segments[4], TextSegment::Text(" links.".to_string()));
    }

    #[test]
    fn test_snapshot_with_urls_in_list_item() {
        let text = "- Check https://example.com for updates";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(
            block.kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert_eq!(block.content, "Check https://example.com for updates");
        assert!(block.segments.is_some());

        let segments = block.segments.as_ref().unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], TextSegment::Text("Check ".to_string()));
        assert_eq!(
            segments[1],
            TextSegment::Url {
                href: "https://example.com".to_string()
            }
        );
        assert_eq!(segments[2], TextSegment::Text(" for updates".to_string()));
    }
}
