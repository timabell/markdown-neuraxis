//! Markdown parsing module that converts markdown text into structured documents.
//!
//! This module handles the transformation of raw markdown content into a hierarchical
//! document structure that can be rendered and manipulated by the application.

use crate::models::{BlockId, BulletMarker, ContentBlock, Document, ListItem, TextSegment};

/// Type alias for the complex item stack tuple to improve readability
type ItemStackEntry = (
    String,
    Vec<ListItem>,
    Vec<ContentBlock>,
    Option<BulletMarker>,
);
use pulldown_cmark::{Event, Tag, TagEnd};
use relative_path::RelativePathBuf;

/// Parse text content and extract wiki-links, returning segments
fn parse_wiki_links(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current_pos = 0;

    // Find all [[...]] patterns
    while let Some(start) = text[current_pos..].find("[[") {
        let absolute_start = current_pos + start;

        // Add any text before the wiki-link
        if start > 0 {
            let text_segment = text[current_pos..absolute_start].to_string();
            if !text_segment.is_empty() {
                segments.push(TextSegment::Text(text_segment));
            }
        }

        // Find the end of the wiki-link
        if let Some(end) = text[absolute_start + 2..].find("]]") {
            let absolute_end = absolute_start + 2 + end;
            let link_content = &text[absolute_start + 2..absolute_end];

            // Parse link content
            let target = link_content.trim().to_string();

            segments.push(TextSegment::WikiLink { target });
            current_pos = absolute_end + 2;
        } else {
            // No closing ]], treat [[ as regular text
            segments.push(TextSegment::Text("[[".to_string()));
            current_pos = absolute_start + 2;
        }
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

/// Detect the bullet marker type at a given position in the content
fn detect_bullet_marker(content: &str, pos: usize) -> Option<BulletMarker> {
    // Look backwards from the position to find the line start
    let line_start = content[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
    // Look at the whole line to find the marker
    let line_end = content[pos..]
        .find('\n')
        .map(|p| p + pos)
        .unwrap_or(content.len());
    let full_line = &content[line_start..line_end];

    // Look for the first non-whitespace character in the full line
    for ch in full_line.chars() {
        match ch {
            '-' => return Some(BulletMarker::Dash),
            '*' => return Some(BulletMarker::Star),
            ' ' | '\t' => continue, // Skip whitespace
            _ => break,             // Stop at any other character
        }
    }
    None
}

/// Parse markdown content into a complete Document structure.
///
/// This function processes markdown text and converts it into a structured document
/// with properly hierarchical content blocks including headings, paragraphs, lists,
/// code blocks, quotes, and horizontal rules.
///
/// # Arguments
/// * `content` - The raw markdown text to parse
/// * `path` - The file path associated with this document
///
/// # Returns
/// A `Document` containing structured content blocks
pub fn parse_markdown(content: &str, path: RelativePathBuf) -> Document {
    use pulldown_cmark::{Options, Parser};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(content, options).into_offset_iter();
    let mut processor = MarkdownProcessor::new();

    for (event, range) in parser {
        processor.process_event_with_offset(event, range, content);
    }

    let blocks = processor.finalize();
    Document::with_content(path, blocks)
}

/// Handles the complex state management required for parsing markdown events.
///
/// # Pulldown-cmark Event Flow for Lists
///
/// Understanding how pulldown-cmark emits events for lists is crucial for correct parsing.
/// The library emits events in a specific order that represents the structure of nested lists.
///
/// ## Simple List Example
/// ```markdown
/// - Item 1
/// - Item 2
/// ```
/// Events:
/// 1. `Start(List)` - Begin the list container
/// 2. `Start(Item)` - Begin first item
/// 3. `Text("Item 1")` - Content of first item
/// 4. `End(Item)` - End first item
/// 5. `Start(Item)` - Begin second item
/// 6. `Text("Item 2")` - Content of second item
/// 7. `End(Item)` - End second item
/// 8. `End(List)` - End the list container
///
/// ## Nested List Example
/// ```markdown
/// - Parent
///   - Child
/// ```
/// Events:
/// 1. `Start(List)` - Begin outer list
/// 2. `Start(Item)` - Begin parent item
/// 3. `Text("Parent")` - Parent item content
/// 4. `Start(List)` - Begin nested list (INSIDE parent item)
/// 5. `Start(Item)` - Begin child item
/// 6. `Text("Child")` - Child item content
/// 7. `End(Item)` - End child item
/// 8. `End(List)` - End nested list
/// 9. `End(Item)` - End parent item (AFTER nested list)
/// 10. `End(List)` - End outer list
///
/// **Key insight**: Nested lists appear INSIDE their parent item, between the parent's
/// text content and the parent's `End(Item)` event.
///
/// The events follow the same pattern, with each nested list appearing inside its
/// parent item. The processor uses two stacks to track this state:
/// - `list_stack`: Tracks list contexts and their items
/// - `item_stack`: Tracks item content and children being built
struct MarkdownProcessor {
    /// Accumulates content blocks as they are completed
    blocks: Vec<ContentBlock>,

    /// Accumulates text content for the current block (paragraph, heading, etc.)
    current_text: String,

    /// Manages nested list parsing state
    list_parser: ListParser,

    /// Tracks whether we're inside a blockquote
    in_blockquote: bool,

    /// Tracks whether we're inside a code block
    in_code_block: bool,

    /// Language specified for the current code block
    code_language: Option<String>,

    /// Accumulates content for the current code block
    code_content: String,
}

impl MarkdownProcessor {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_text: String::new(),
            list_parser: ListParser::new(),
            in_blockquote: false,
            in_code_block: false,
            code_language: None,
            code_content: String::new(),
        }
    }

    /// Process a markdown event with offset information for marker detection
    fn process_event_with_offset(
        &mut self,
        event: Event,
        range: std::ops::Range<usize>,
        content: &str,
    ) {
        match event {
            Event::Start(Tag::Paragraph) => {
                // New paragraph - flush any existing one
                self.flush_paragraph();
            }
            Event::End(TagEnd::Paragraph) => {
                // End paragraph - flush it unless we're in a blockquote
                if !self.in_blockquote {
                    self.flush_paragraph();
                }
            }
            Event::Start(Tag::Heading { level: _, .. }) => {
                self.flush_paragraph();
            }
            Event::End(TagEnd::Heading(level)) => {
                let text = self.current_text.trim().to_string();
                if !text.is_empty() {
                    self.blocks.push(ContentBlock::Heading {
                        level: level as u8,
                        text,
                    });
                }
                self.current_text.clear();
            }
            Event::Start(Tag::List(first_item)) => {
                // Only flush text if this is a top-level list
                if !self.list_parser.is_parsing() {
                    self.flush_paragraph();
                }
                self.list_parser.start_list(first_item.is_some());
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(block) = self.list_parser.end_list() {
                    self.blocks.push(block);
                }
            }
            Event::Start(Tag::Item) => {
                // Detect bullet marker from the content at this position
                let marker = detect_bullet_marker(content, range.start);
                self.list_parser.start_item_with_marker(marker);
            }
            Event::End(TagEnd::Item) => {
                self.list_parser.end_item();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                // Only flush paragraph if this is a top-level code block
                if !self.list_parser.is_in_item() {
                    self.flush_paragraph();
                }
                self.in_code_block = true;
                self.code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    _ => None,
                };
                self.code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                let code_block = ContentBlock::CodeBlock {
                    language: self.code_language.take(),
                    code: self.code_content.clone(),
                };

                if self.list_parser.is_in_item() {
                    // This code block is inside a list item, add it to the current item
                    self.list_parser.add_nested_content(code_block);
                } else {
                    // This is a top-level code block
                    self.blocks.push(code_block);
                }
                self.code_content.clear();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                self.flush_paragraph();
                self.in_blockquote = true;
            }
            Event::End(TagEnd::BlockQuote) => {
                let text = self.current_text.trim().to_string();
                if !text.is_empty() {
                    self.blocks.push(ContentBlock::Quote(text));
                }
                self.current_text.clear();
                self.in_blockquote = false;
            }
            Event::Rule => {
                self.flush_paragraph();
                self.blocks.push(ContentBlock::Rule);
            }
            Event::Text(text) => {
                if self.in_code_block {
                    self.code_content.push_str(&text);
                } else if self.list_parser.is_in_item() {
                    self.list_parser.add_text(&text);
                } else {
                    self.current_text.push_str(&text);
                }
            }
            Event::Code(code) => {
                // Handle inline code
                let code_text = format!("`{code}`");
                if self.list_parser.is_in_item() {
                    self.list_parser.add_text(&code_text);
                } else {
                    self.current_text.push_str(&code_text);
                }
            }
            Event::SoftBreak => {
                // Soft breaks (regular newlines) are rendered as spaces in HTML
                if self.in_code_block {
                    self.code_content.push('\n');
                } else if self.list_parser.is_in_item() {
                    self.list_parser.add_text(" ");
                } else {
                    self.current_text.push(' ');
                }
            }
            Event::HardBreak => {
                // Hard breaks (trailing spaces + newline) - preserve the original pattern
                if self.in_code_block {
                    self.code_content.push('\n');
                } else if self.list_parser.is_in_item() {
                    self.list_parser.add_text("  \n");
                } else {
                    self.current_text.push_str("  \n");
                }
            }
            _ => {}
        }
    }

    /// Flush any pending paragraph text to blocks
    fn flush_paragraph(&mut self) {
        let text = self.current_text.trim().to_string();
        if !text.is_empty() {
            let segments = parse_wiki_links(&text);
            self.blocks.push(ContentBlock::Paragraph { segments });
        }
        self.current_text.clear();
    }

    /// Finalize processing and return all content blocks
    fn finalize(mut self) -> Vec<ContentBlock> {
        self.flush_paragraph();
        self.blocks
    }
}

/// Dedicated parser for handling the complex state of nested list parsing.
///
/// This struct encapsulates all the complexity of tracking nested lists and items,
/// making the main parsing logic cleaner and more maintainable.
struct ListParser {
    /// Stack of (items, is_ordered) pairs tracking nested list contexts
    /// - Each entry represents one list level in the document
    /// - Stack depth indicates current nesting level (0 = top-level)
    list_stack: Vec<(Vec<ListItem>, bool)>,

    /// Stack of (text, children, nested_content, marker) tuples for items being constructed
    /// - Text accumulates while parsing item content
    /// - Children collect nested lists that appear within the item
    /// - Nested content collects code blocks and other content that appear within the item
    /// - Marker stores the original bullet marker type
    item_stack: Vec<ItemStackEntry>,

    /// Whether we're currently parsing inside a list item
    in_item: bool,
}

impl ListParser {
    fn new() -> Self {
        Self {
            list_stack: Vec::new(),
            item_stack: Vec::new(),
            in_item: false,
        }
    }

    /// Check if we're currently parsing any list
    fn is_parsing(&self) -> bool {
        !self.list_stack.is_empty()
    }

    /// Check if we're currently inside a list item
    fn is_in_item(&self) -> bool {
        self.in_item
    }

    /// Start a new list (either top-level or nested)
    fn start_list(&mut self, is_ordered: bool) {
        self.list_stack.push((Vec::new(), is_ordered));
    }

    /// End the current list and return a ContentBlock if it's top-level
    fn end_list(&mut self) -> Option<ContentBlock> {
        debug_assert!(
            !self.list_stack.is_empty(),
            "List end without corresponding start"
        );

        if let Some((items, is_ordered)) = self.list_stack.pop() {
            if self.list_stack.is_empty() {
                // This is a top-level list, return it as a content block
                if !items.is_empty() {
                    let items_with_ids: Vec<(BlockId, ListItem)> = items
                        .into_iter()
                        .map(|item| (BlockId::new(), item))
                        .collect();
                    return Some(if is_ordered {
                        ContentBlock::NumberedList {
                            items: items_with_ids,
                        }
                    } else {
                        ContentBlock::BulletList {
                            items: items_with_ids,
                        }
                    });
                }
            } else {
                // This is a nested list, add it as children for the parent item
                if let Some((_, children, _, _)) = self.item_stack.last_mut() {
                    // If this was a numbered list, mark all items as numbered
                    let mut items_to_add = items;
                    if is_ordered {
                        for item in &mut items_to_add {
                            if item.marker.is_none() {
                                item.marker = Some(BulletMarker::Numbered);
                            }
                        }
                    }
                    children.extend(items_to_add);
                }
            }
        }
        None
    }

    /// Start a new list item with a specific bullet marker
    fn start_item_with_marker(&mut self, marker: Option<BulletMarker>) {
        self.item_stack
            .push((String::new(), Vec::new(), Vec::new(), marker));
        self.in_item = true;
    }

    /// End the current list item and add it to the current list
    fn end_item(&mut self) {
        self.in_item = false;
        debug_assert!(
            !self.item_stack.is_empty(),
            "Item end without corresponding start"
        );

        if let Some((text, children, nested_content, marker)) = self.item_stack.pop() {
            // Calculate nesting level: subtract 1 because list_stack includes the current list
            let level = self.list_stack.len().saturating_sub(1);
            let trimmed_text = text.trim().to_string();
            let segments = parse_wiki_links(&trimmed_text);

            let mut item = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. }))
            {
                ListItem::with_segments(trimmed_text, segments, level)
            } else {
                ListItem::new(trimmed_text, level)
            };

            item.children = children;
            item.nested_content = nested_content;
            item.marker = marker;

            // Add this item to the current list
            if let Some((items, _)) = self.list_stack.last_mut() {
                items.push(item);
            }
        }
    }

    /// Add text content to the current item
    fn add_text(&mut self, text: &str) {
        if let Some((item_text, _, _, _)) = self.item_stack.last_mut() {
            item_text.push_str(text);
        }
    }

    /// Add nested content (like code blocks) to the current item
    fn add_nested_content(&mut self, content: ContentBlock) {
        if let Some((_, _, nested_content, _)) = self.item_stack.last_mut() {
            nested_content.push(content);
        }
    }
}

/// Parse a single markdown block from text.
///
/// This function attempts to parse a single block of markdown content,
/// determining its type based on content patterns (headings, lists, code blocks, etc.).
///
/// # Arguments
/// * `markdown` - Raw markdown text representing a single block
///
/// # Returns
/// A `Result` containing the parsed `ContentBlock` or an error message
pub fn from_markdown(markdown: &str) -> Result<ContentBlock, String> {
    let trimmed = markdown.trim();

    // Try to parse as heading
    if trimmed.starts_with('#') {
        let level_end = trimmed.chars().take_while(|c| *c == '#').count();
        if level_end > 0 && level_end <= 6 {
            let text = trimmed[level_end..].trim().to_string();
            return Ok(ContentBlock::Heading {
                level: level_end as u8,
                text,
            });
        }
    }

    // Try to parse as code block
    if trimmed.starts_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 && lines.last() == Some(&"```") {
            let first_line = lines[0];
            let language = if first_line.len() > 3 {
                Some(first_line[3..].to_string())
            } else {
                None
            };
            let code = lines[1..lines.len() - 1].join("\n");
            return Ok(ContentBlock::CodeBlock { language, code });
        }
    }

    // Try to parse as quote
    if let Some(stripped) = trimmed.strip_prefix('>') {
        let text = stripped.trim().to_string();
        return Ok(ContentBlock::Quote(text));
    }

    // Try to parse as rule
    if trimmed == "---" || trimmed == "***" {
        return Ok(ContentBlock::Rule);
    }

    // Try to parse as list
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        // Parse multiple bullet points separated by newlines
        let lines: Vec<&str> = trimmed.lines().collect();
        let mut items = Vec::new();

        for line in lines {
            let line = line.trim();
            if line.starts_with("- ") || line.starts_with("* ") {
                let content = line[2..].trim().to_string();
                let segments = parse_wiki_links(&content);
                let marker = if line.starts_with("- ") {
                    Some(BulletMarker::Dash)
                } else {
                    Some(BulletMarker::Star)
                };
                let mut item = if segments
                    .iter()
                    .any(|s| matches!(s, TextSegment::WikiLink { .. }))
                {
                    ListItem::with_segments(content, segments, 0)
                } else {
                    ListItem::new(content, 0)
                };
                item.marker = marker;
                items.push(item);
            }
        }

        if !items.is_empty() {
            let items_with_ids: Vec<(BlockId, ListItem)> = items
                .into_iter()
                .map(|item| (BlockId::new(), item))
                .collect();
            return Ok(ContentBlock::BulletList {
                items: items_with_ids,
            });
        }
    }

    // Try to parse as numbered list
    if trimmed
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
        && trimmed.contains(". ")
    {
        // Parse multiple numbered items separated by newlines
        let lines: Vec<&str> = trimmed.lines().collect();
        let mut items = Vec::new();

        for line in lines {
            let line = line.trim();
            // Check if line starts with number followed by ". "
            if let Some(dot_pos) = line.find(". ") {
                if line[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                    let content = line[dot_pos + 2..].trim().to_string();
                    let segments = parse_wiki_links(&content);
                    let item = if segments
                        .iter()
                        .any(|s| matches!(s, TextSegment::WikiLink { .. }))
                    {
                        ListItem::with_segments(content, segments, 0)
                    } else {
                        ListItem::new(content, 0)
                    };
                    items.push(item);
                }
            }
        }

        if !items.is_empty() {
            let items_with_ids: Vec<(BlockId, ListItem)> = items
                .into_iter()
                .map(|item| (BlockId::new(), item))
                .collect();
            return Ok(ContentBlock::NumberedList {
                items: items_with_ids,
            });
        }
    }

    // Default to paragraph
    let segments = parse_wiki_links(trimmed);
    Ok(ContentBlock::Paragraph { segments })
}

/// Parse markdown content that may contain multiple blocks separated by double newlines.
///
/// This function splits markdown content on double newlines and parses each chunk
/// as a separate block. Empty chunks are filtered out.
///
/// # Arguments
/// * `markdown` - Raw markdown text potentially containing multiple blocks
///
/// # Returns
/// A vector of parsed `ContentBlock`s
pub fn parse_multiple_blocks(markdown: &str) -> Vec<ContentBlock> {
    if markdown.trim().is_empty() {
        return vec![];
    }

    // Split on double newlines (handles \n\n, \r\n\r\n, etc.)
    let chunks: Vec<&str> = markdown
        .split("\n\n")
        .map(|chunk| chunk.trim())
        .filter(|chunk| !chunk.is_empty())
        .collect();

    if chunks.is_empty() {
        return vec![];
    }

    // If there's only one chunk, use the original single-block parsing
    if chunks.len() == 1 {
        if let Ok(block) = from_markdown(chunks[0]) {
            return vec![block];
        }
    }

    // Parse each chunk as a separate block
    chunks
        .into_iter()
        .filter_map(|chunk| from_markdown(chunk).ok())
        .collect()
}

#[cfg(test)]
mod roundtrip_tests;

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContentBlock, TextSegment};

    #[test]
    fn test_heading_from_markdown() {
        let result = from_markdown("### Test Heading").unwrap();
        assert_eq!(
            result,
            ContentBlock::Heading {
                level: 3,
                text: "Test Heading".to_string()
            }
        );
    }

    #[test]
    fn test_paragraph_from_markdown() {
        let result = from_markdown("This is a [[wiki-link]] test.").unwrap();
        if let ContentBlock::Paragraph { segments } = result {
            assert_eq!(segments.len(), 3);
            assert_eq!(segments[0], TextSegment::Text("This is a ".to_string()));
            assert_eq!(
                segments[1],
                TextSegment::WikiLink {
                    target: "wiki-link".to_string(),
                }
            );
            assert_eq!(segments[2], TextSegment::Text(" test.".to_string()));
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_multiple_blocks_single_paragraph() {
        let markdown = "This is a single paragraph.";
        let blocks = parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::Paragraph { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_split_paragraphs() {
        let markdown = "First paragraph.\n\nSecond paragraph.";
        let blocks = parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], ContentBlock::Paragraph { .. }));
        assert!(matches!(blocks[1], ContentBlock::Paragraph { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_mixed_content() {
        let markdown = "# Heading\n\nThis is a paragraph.\n\n- List item";
        let blocks = parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], ContentBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], ContentBlock::Paragraph { .. }));
        assert!(matches!(blocks[2], ContentBlock::BulletList { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_empty_input() {
        let blocks = parse_multiple_blocks("");
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_numbered_list_parsing_from_editor() {
        // Test parsing numbered list that would happen when user types in the editor
        let markdown = "1. first item\n2. second item\n3. third item";

        let result = from_markdown(markdown).unwrap();

        if let ContentBlock::NumberedList { items } = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].1.content, "first item");
            assert_eq!(items[1].1.content, "second item");
            assert_eq!(items[2].1.content, "third item");
        } else {
            panic!("Expected numbered list, got: {result:?}");
        }
    }

    #[test]
    fn test_bullet_list_parsing_from_editor() {
        // Test parsing that would happen when user types a bullet list in the editor
        let markdown = "- bullet one\n- bullet two\n- bullet three";

        // This simulates what happens when from_markdown is called
        let result = from_markdown(markdown).unwrap();

        if let ContentBlock::BulletList { items } = result {
            // Should have 3 separate items
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].1.content, "bullet one");
            assert_eq!(items[1].1.content, "bullet two");
            assert_eq!(items[2].1.content, "bullet three");
        } else {
            panic!("Expected bullet list, got: {result:?}");
        }
    }

    #[test]
    fn test_parse_simple_list() {
        let content = "- First item\n- Second item";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].1.content, "First item");
            assert_eq!(items[1].1.content, "Second item");
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let content = "- Parent\n  - Child";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1.content, "Parent");
            assert_eq!(items[0].1.children.len(), 1);
            assert_eq!(items[0].1.children[0].content, "Child");
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_mixed_content() {
        let content = "# Title\n\nSome text\n\n- List item\n\n```rust\ncode\n```";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 4);
        assert!(matches!(
            doc.content[0],
            ContentBlock::Heading { level: 1, .. }
        ));
        assert!(matches!(doc.content[1], ContentBlock::Paragraph { .. }));
        assert!(matches!(doc.content[2], ContentBlock::BulletList { .. }));
        assert!(matches!(doc.content[3], ContentBlock::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_inline_code_in_list() {
        let content = "- This is a bullet point with inline code: `let x = 5;`";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(
                items[0].1.content,
                "This is a bullet point with inline code: `let x = 5;`"
            );
            assert!(
                items[0].1.nested_content.is_empty(),
                "Inline code should not create nested content"
            );
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_fenced_code_block_in_list() {
        let content = r#"- This bullet has a fenced code block:
  ```rust
  fn example() {
      println!("Hello");
  }
  ```"#;
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1.content, "This bullet has a fenced code block:");
            assert_eq!(items[0].1.nested_content.len(), 1);

            if let ContentBlock::CodeBlock { language, code } = &items[0].1.nested_content[0] {
                assert_eq!(language.as_ref().unwrap(), "rust");
                assert!(code.contains("fn example()"));
                assert!(code.contains("println!"));
            } else {
                panic!("Expected CodeBlock in nested_content");
            }
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_multiple_code_blocks_in_list() {
        let content = r#"- First item with code:
  ```rust
  fn first() { }
  ```
- Second item with inline: `x = 1`
- Third item with multiple blocks:
  ```python
  def hello():
      pass
  ```
  ```javascript
  console.log("test");
  ```"#;
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 3);

            // First item - one code block
            assert_eq!(items[0].1.content, "First item with code:");
            assert_eq!(items[0].1.nested_content.len(), 1);

            // Second item - inline code only
            assert_eq!(items[1].1.content, "Second item with inline: `x = 1`");
            assert_eq!(items[1].1.nested_content.len(), 0);

            // Third item - two code blocks
            assert_eq!(items[2].1.content, "Third item with multiple blocks:");
            assert_eq!(items[2].1.nested_content.len(), 2);

            if let ContentBlock::CodeBlock { language, .. } = &items[2].1.nested_content[0] {
                assert_eq!(language.as_ref().unwrap(), "python");
            } else {
                panic!("Expected first CodeBlock to be Python");
            }

            if let ContentBlock::CodeBlock { language, .. } = &items[2].1.nested_content[1] {
                assert_eq!(language.as_ref().unwrap(), "javascript");
            } else {
                panic!("Expected second CodeBlock to be JavaScript");
            }
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_nested_lists_with_code_blocks() {
        let content = r#"- Parent item
  - Nested item with code:
    ```rust
    fn nested() { }
    ```
  - Another nested item"#;
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1.content, "Parent item");
            assert_eq!(items[0].1.children.len(), 2);

            // First nested item should have code block
            assert_eq!(items[0].1.children[0].content, "Nested item with code:");
            assert_eq!(items[0].1.children[0].nested_content.len(), 1);

            // Second nested item should not have code block
            assert_eq!(items[0].1.children[1].content, "Another nested item");
            assert_eq!(items[0].1.children[1].nested_content.len(), 0);
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_wiki_links() {
        let content = "This is a paragraph with [[Simple-Link]] and [[Complex-Link]].";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::Paragraph { segments } = &doc.content[0] {
            assert_eq!(segments.len(), 5);

            // Check the segments
            assert_eq!(
                segments[0],
                TextSegment::Text("This is a paragraph with ".to_string())
            );
            assert_eq!(
                segments[1],
                TextSegment::WikiLink {
                    target: "Simple-Link".to_string(),
                }
            );
            assert_eq!(segments[2], TextSegment::Text(" and ".to_string()));
            assert_eq!(
                segments[3],
                TextSegment::WikiLink {
                    target: "Complex-Link".to_string(),
                }
            );
            assert_eq!(segments[4], TextSegment::Text(".".to_string()));
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_parse_wiki_links_in_list() {
        let content = "- List item with [[Page-Link]] reference";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);

            // Check that the item has segments
            if let Some(ref segments) = items[0].1.segments {
                assert_eq!(segments.len(), 3);
                assert_eq!(
                    segments[0],
                    TextSegment::Text("List item with ".to_string())
                );
                assert_eq!(
                    segments[1],
                    TextSegment::WikiLink {
                        target: "Page-Link".to_string(),
                    }
                );
                assert_eq!(segments[2], TextSegment::Text(" reference".to_string()));
            } else {
                panic!("Expected list item to have segments");
            }
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_soft_breaks_vs_hard_breaks() {
        // Test soft break (regular newline without trailing spaces)
        let soft_break_content = "First line\nSecond line in same paragraph";
        let soft_doc = parse_markdown(soft_break_content, RelativePathBuf::from("test.md"));

        // Test hard break (trailing spaces + newline)
        let hard_break_content = "First line  \nSecond line in same paragraph";
        let hard_doc = parse_markdown(hard_break_content, RelativePathBuf::from("test.md"));

        // Both should produce 1 paragraph
        assert_eq!(soft_doc.content.len(), 1);
        assert_eq!(hard_doc.content.len(), 1);

        // Check soft break behavior - should have space instead of newline
        if let ContentBlock::Paragraph {
            segments: soft_segments,
        } = &soft_doc.content[0]
        {
            if let TextSegment::Text(soft_text) = &soft_segments[0] {
                assert_eq!(soft_text, "First line Second line in same paragraph");
                assert!(!soft_text.contains("  \n"));
            } else {
                panic!("Expected text segment for soft break");
            }
        } else {
            panic!("Expected paragraph for soft break");
        }

        // Check hard break behavior - should have original pattern preserved
        if let ContentBlock::Paragraph {
            segments: hard_segments,
        } = &hard_doc.content[0]
        {
            if let TextSegment::Text(hard_text) = &hard_segments[0] {
                assert!(hard_text.contains("  \n"));
                assert_eq!(hard_text, "First line  \nSecond line in same paragraph");
            } else {
                panic!("Expected text segment for hard break");
            }
        } else {
            panic!("Expected paragraph for hard break");
        }
    }

    #[test]
    fn test_bullet_list_with_soft_breaks() {
        // Test bullet list where items are separated by single newlines (soft breaks)
        let content = "- bullet one\n- bullet two\n- bullet three";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        // Should have 1 bullet list block
        assert_eq!(doc.content.len(), 1);

        if let ContentBlock::BulletList { items } = &doc.content[0] {
            // Should have 3 separate items, not combined
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].1.content, "bullet one");
            assert_eq!(items[1].1.content, "bullet two");
            assert_eq!(items[2].1.content, "bullet three");
        } else {
            panic!("Expected bullet list block");
        }
    }

    #[test]
    fn test_consecutive_paragraphs_are_separate_blocks() {
        let content = "First paragraph content here.\n\nSecond paragraph content here.\n\nThird paragraph content here.";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        // Should have 3 separate paragraph blocks
        assert_eq!(doc.content.len(), 3);

        // All should be paragraphs
        assert!(matches!(doc.content[0], ContentBlock::Paragraph { .. }));
        assert!(matches!(doc.content[1], ContentBlock::Paragraph { .. }));
        assert!(matches!(doc.content[2], ContentBlock::Paragraph { .. }));

        // Check content
        if let ContentBlock::Paragraph { segments } = &doc.content[0] {
            if let TextSegment::Text(text) = &segments[0] {
                assert_eq!(text, "First paragraph content here.");
            } else {
                panic!("Expected first segment to be text");
            }
        } else {
            panic!("Expected first block to be paragraph");
        }

        if let ContentBlock::Paragraph { segments } = &doc.content[1] {
            if let TextSegment::Text(text) = &segments[0] {
                assert_eq!(text, "Second paragraph content here.");
            } else {
                panic!("Expected first segment to be text");
            }
        } else {
            panic!("Expected second block to be paragraph");
        }

        if let ContentBlock::Paragraph { segments } = &doc.content[2] {
            if let TextSegment::Text(text) = &segments[0] {
                assert_eq!(text, "Third paragraph content here.");
            } else {
                panic!("Expected first segment to be text");
            }
        } else {
            panic!("Expected third block to be paragraph");
        }
    }

    #[test]
    fn test_bullet_marker_detection() {
        let content = "- First dash item\n* First star item";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        // Different bullet types create separate lists
        assert_eq!(doc.content.len(), 2);

        // First list with dash marker
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1.marker, Some(BulletMarker::Dash));
        } else {
            panic!("Expected first BulletList block");
        }

        // Second list with star marker
        if let ContentBlock::BulletList { items } = &doc.content[1] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1.marker, Some(BulletMarker::Star));
        } else {
            panic!("Expected second BulletList block");
        }

        // Test roundtrip behavior
        use crate::models::DocumentState;
        let doc_state = DocumentState::from_document(doc);
        let roundtrip_doc = doc_state.to_document();
        let roundtrip = roundtrip_doc
            .content
            .iter()
            .map(|block| block.to_markdown())
            .collect::<Vec<_>>()
            .join("\n");
        println!("Original: {content:?}");
        println!("Roundtrip: {roundtrip:?}");
        assert_eq!(content, roundtrip);
    }

    #[test]
    fn test_parsing_numbered_list_with_nested_bullets() {
        let content = "1. First item\n\t- Nested dash\n\t* Nested star\n2. Second item";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);

        if let ContentBlock::NumberedList { items } = &doc.content[0] {
            assert_eq!(items.len(), 2);

            // First item should have both dash and star nested items
            assert_eq!(items[0].1.content, "First item");
            assert_eq!(items[0].1.children.len(), 2);
            assert_eq!(items[0].1.children[0].content, "Nested dash");
            assert_eq!(items[0].1.children[0].marker, Some(BulletMarker::Dash));
            assert_eq!(items[0].1.children[1].content, "Nested star");
            assert_eq!(items[0].1.children[1].marker, Some(BulletMarker::Star));

            // Second item should have no children
            assert_eq!(items[1].1.content, "Second item");
            assert_eq!(items[1].1.children.len(), 0);
        } else {
            panic!("Expected NumberedList block");
        }
    }

    #[test]
    fn test_parsing_bullet_list_with_nested_numbered() {
        let content = "- Bullet item\n\t1. First numbered\n\t2. Second numbered";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);

        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);

            // Bullet item should have nested numbered items
            assert_eq!(items[0].1.content, "Bullet item");
            assert_eq!(items[0].1.children.len(), 2);
            assert_eq!(items[0].1.children[0].content, "First numbered");
            assert_eq!(items[0].1.children[1].content, "Second numbered");

            // These should now have Numbered marker
            assert_eq!(items[0].1.children[0].marker, Some(BulletMarker::Numbered));
            assert_eq!(items[0].1.children[1].marker, Some(BulletMarker::Numbered));

            // Test that it roundtrips correctly
            let regenerated = doc
                .content
                .iter()
                .map(|block| block.to_markdown())
                .collect::<Vec<_>>()
                .join("\n");
            println!("Original: {content}");
            println!("Generated: {regenerated}");

            // Should contain numbered items
            assert!(regenerated.contains("1. First numbered"));
            assert!(regenerated.contains("2. Second numbered"));
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_rendering_nested_mixed_lists() {
        use crate::models::{BlockId, ListItem};

        // Test rendering numbered list with nested bullet items
        let dash_child = ListItem {
            content: "Nested dash".to_string(),
            segments: None,
            level: 1,
            children: vec![],
            nested_content: vec![],
            marker: Some(BulletMarker::Dash),
        };

        let star_child = ListItem {
            content: "Nested star".to_string(),
            segments: None,
            level: 1,
            children: vec![],
            nested_content: vec![],
            marker: Some(BulletMarker::Star),
        };

        let parent_item = ListItem {
            content: "Parent item".to_string(),
            segments: None,
            level: 0,
            children: vec![dash_child, star_child],
            nested_content: vec![],
            marker: None,
        };

        let numbered_list = ContentBlock::NumberedList {
            items: vec![(BlockId::new(), parent_item)],
        };

        let result = numbered_list.to_markdown();
        println!("Rendered numbered with nested bullets: {result}");

        // Should contain both dash and star markers
        assert!(result.contains("- Nested dash"));
        assert!(result.contains("* Nested star"));
    }

    #[test]
    fn test_rendering_bullet_with_nested_numbered_items() {
        use crate::models::{BlockId, ListItem};

        // Test rendering bullet list with children that should be numbered
        let first_numbered = ListItem {
            content: "First numbered".to_string(),
            segments: None,
            level: 1,
            children: vec![],
            nested_content: vec![],
            marker: None, // This is the issue - no marker to indicate it should be numbered
        };

        let second_numbered = ListItem {
            content: "Second numbered".to_string(),
            segments: None,
            level: 1,
            children: vec![],
            nested_content: vec![],
            marker: None,
        };

        let parent_item = ListItem {
            content: "Bullet item".to_string(),
            segments: None,
            level: 0,
            children: vec![first_numbered, second_numbered],
            nested_content: vec![],
            marker: Some(BulletMarker::Dash),
        };

        let bullet_list = ContentBlock::BulletList {
            items: vec![(BlockId::new(), parent_item)],
        };

        let result = bullet_list.to_markdown();
        println!("Rendered bullet with nested items: {result}");

        // Currently this will render as "- First numbered" and "- Second numbered"
        // But it should render as "1. First numbered" and "2. Second numbered"
    }

    #[test]
    fn test_quote_parsing_debug() {
        let content = "> This is a quote";
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        println!("Parsed {} blocks:", doc.content.len());
        for (i, block) in doc.content.iter().enumerate() {
            println!("Block {i}: {block:?}");
        }

        let regenerated = doc
            .content
            .iter()
            .map(|block| block.to_markdown())
            .collect::<Vec<_>>()
            .join("\n");

        println!("Original: '{content}'");
        println!("Generated: '{regenerated}'");

        // Should be a quote
        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::Quote(text) = &doc.content[0] {
            assert_eq!(text, "This is a quote");
            assert_eq!(regenerated, "> This is a quote");
        } else {
            panic!("Expected Quote block, got: {:?}", doc.content[0]);
        }
    }

    #[test]
    fn test_standalone_code_blocks_still_work() {
        let content = r#"# Title

Paragraph text.

```rust
fn standalone() {
    println!("This should not be in a list");
}
```

- List item after code block"#;
        let doc = parse_markdown(content, RelativePathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 4);
        assert!(matches!(doc.content[0], ContentBlock::Heading { .. }));
        assert!(matches!(doc.content[1], ContentBlock::Paragraph { .. }));
        assert!(matches!(doc.content[2], ContentBlock::CodeBlock { .. }));
        assert!(matches!(doc.content[3], ContentBlock::BulletList { .. }));

        // Verify the standalone code block
        if let ContentBlock::CodeBlock { language, code } = &doc.content[2] {
            assert_eq!(language.as_ref().unwrap(), "rust");
            assert!(code.contains("standalone"));
        } else {
            panic!("Expected standalone CodeBlock");
        }
    }
}
