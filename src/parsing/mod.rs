//! Markdown parsing module that converts markdown text into structured documents.
//!
//! This module handles the transformation of raw markdown content into a hierarchical
//! document structure that can be rendered and manipulated by the application.

use crate::models::{ContentBlock, Document, ListItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

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
pub fn parse_markdown(content: &str, path: PathBuf) -> Document {
    let parser = Parser::new(content);
    let mut processor = MarkdownProcessor::new();

    for event in parser {
        processor.process_event(event);
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
/// ## Complex Nested Example
/// ```markdown
/// - Item 1
///   - Item 1.1
///     - Item 1.1.1
///   - Item 1.2
/// - Item 2
/// ```
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
            in_code_block: false,
            code_language: None,
            code_content: String::new(),
        }
    }

    /// Process a single markdown event
    fn process_event(&mut self, event: Event) {
        match event {
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
                self.list_parser.start_item();
            }
            Event::End(TagEnd::Item) => {
                self.list_parser.end_item();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                self.flush_paragraph();
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
                self.blocks.push(ContentBlock::CodeBlock {
                    language: self.code_language.take(),
                    code: self.code_content.clone(),
                });
                self.code_content.clear();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                self.flush_paragraph();
            }
            Event::End(TagEnd::BlockQuote) => {
                let text = self.current_text.trim().to_string();
                if !text.is_empty() {
                    self.blocks.push(ContentBlock::Quote(text));
                }
                self.current_text.clear();
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
            Event::SoftBreak | Event::HardBreak => {
                if self.in_code_block {
                    self.code_content.push('\n');
                } else if self.list_parser.is_in_item() {
                    self.list_parser.add_text("\n");
                } else {
                    self.current_text.push('\n');
                }
            }
            _ => {}
        }
    }

    /// Flush any pending paragraph text to blocks
    fn flush_paragraph(&mut self) {
        let text = self.current_text.trim().to_string();
        if !text.is_empty() {
            self.blocks.push(ContentBlock::Paragraph(text));
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

    /// Stack of (text, children) pairs for items being constructed
    /// - Text accumulates while parsing item content
    /// - Children collect nested lists that appear within the item
    item_stack: Vec<(String, Vec<ListItem>)>,

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
                    return Some(if is_ordered {
                        ContentBlock::NumberedList { items }
                    } else {
                        ContentBlock::BulletList { items }
                    });
                }
            } else {
                // This is a nested list, save it as children for the parent item
                if let Some((_, children)) = self.item_stack.last_mut() {
                    *children = items;
                }
            }
        }
        None
    }

    /// Start a new list item
    fn start_item(&mut self) {
        self.item_stack.push((String::new(), Vec::new()));
        self.in_item = true;
    }

    /// End the current list item and add it to the current list
    fn end_item(&mut self) {
        self.in_item = false;
        debug_assert!(
            !self.item_stack.is_empty(),
            "Item end without corresponding start"
        );

        if let Some((text, children)) = self.item_stack.pop() {
            // Calculate nesting level: subtract 1 because list_stack includes the current list
            let level = self.list_stack.len().saturating_sub(1);
            let mut item = ListItem::new(text.trim().to_string(), level);
            item.children = children;

            // Add this item to the current list
            if let Some((items, _)) = self.list_stack.last_mut() {
                items.push(item);
            }
        }
    }

    /// Add text content to the current item
    fn add_text(&mut self, text: &str) {
        if let Some((item_text, _)) = self.item_stack.last_mut() {
            item_text.push_str(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_list() {
        let content = "- First item\n- Second item";
        let doc = parse_markdown(content, PathBuf::from("/test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].content, "First item");
            assert_eq!(items[1].content, "Second item");
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let content = "- Parent\n  - Child";
        let doc = parse_markdown(content, PathBuf::from("/test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].content, "Parent");
            assert_eq!(items[0].children.len(), 1);
            assert_eq!(items[0].children[0].content, "Child");
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_parse_mixed_content() {
        let content = "# Title\n\nSome text\n\n- List item\n\n```rust\ncode\n```";
        let doc = parse_markdown(content, PathBuf::from("/test.md"));

        assert_eq!(doc.content.len(), 4);
        assert!(matches!(
            doc.content[0],
            ContentBlock::Heading { level: 1, .. }
        ));
        assert!(matches!(doc.content[1], ContentBlock::Paragraph(_)));
        assert!(matches!(doc.content[2], ContentBlock::BulletList { .. }));
        assert!(matches!(doc.content[3], ContentBlock::CodeBlock { .. }));
    }
}
