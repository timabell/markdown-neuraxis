use crate::models::{ContentBlock, Document, ListItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

/// Parse markdown content into a complete Document structure
pub fn parse_markdown(content: &str, path: PathBuf) -> Document {
    let parser = Parser::new(content);
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut current_text = String::new();
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut text_stack: Vec<String> = Vec::new();
    let mut list_stack: Vec<usize> = Vec::new();
    let mut in_item = false;
    let mut _in_list = false;
    let mut is_ordered_list = false;
    let mut in_code_block = false;
    let mut code_language: Option<String> = None;
    let mut code_content = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level: _, .. }) => {
                flush_text(&mut current_text, &mut blocks);
                current_text.clear();
            }
            Event::End(TagEnd::Heading(level)) => {
                let text = current_text.trim().to_string();
                if !text.is_empty() {
                    blocks.push(ContentBlock::Heading {
                        level: level as u8,
                        text,
                    });
                }
                current_text.clear();
            }
            Event::Start(Tag::List(first_item)) => {
                flush_text(&mut current_text, &mut blocks);
                list_stack.push(0);
                _in_list = true;
                is_ordered_list = first_item.is_some();
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    _in_list = false;
                    let hierarchy = build_hierarchy(list_items.clone());
                    if !hierarchy.is_empty() {
                        if is_ordered_list {
                            blocks.push(ContentBlock::NumberedList { items: hierarchy });
                        } else {
                            blocks.push(ContentBlock::BulletList { items: hierarchy });
                        }
                    }
                    list_items.clear();
                }
            }
            Event::Start(Tag::Item) => {
                text_stack.push(String::new());
                in_item = true;
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                if let Some(text) = text_stack.pop() {
                    if !text.trim().is_empty() {
                        let level = list_stack.len().saturating_sub(1);
                        let item = ListItem::new(text.trim().to_string(), level);
                        list_items.push(item);
                    }
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_text(&mut current_text, &mut blocks);
                in_code_block = true;
                code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    _ => None,
                };
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                blocks.push(ContentBlock::CodeBlock {
                    language: code_language.take(),
                    code: code_content.clone(),
                });
                code_content.clear();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_text(&mut current_text, &mut blocks);
            }
            Event::End(TagEnd::BlockQuote) => {
                let text = current_text.trim().to_string();
                if !text.is_empty() {
                    blocks.push(ContentBlock::Quote(text));
                }
                current_text.clear();
            }
            Event::Rule => {
                flush_text(&mut current_text, &mut blocks);
                blocks.push(ContentBlock::Rule);
            }
            Event::Text(text) => {
                if in_code_block {
                    code_content.push_str(&text);
                } else if in_item {
                    if let Some(current_item_text) = text_stack.last_mut() {
                        current_item_text.push_str(&text);
                    }
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if in_item {
                    if let Some(current_item_text) = text_stack.last_mut() {
                        current_item_text.push('\n');
                    }
                } else {
                    current_text.push('\n');
                }
            }
            _ => {}
        }
    }

    // Flush any remaining text as a paragraph
    flush_text(&mut current_text, &mut blocks);

    Document::with_content(path, blocks)
}

fn flush_text(current_text: &mut String, blocks: &mut Vec<ContentBlock>) {
    let text = current_text.trim().to_string();
    if !text.is_empty() {
        blocks.push(ContentBlock::Paragraph(text));
    }
    current_text.clear();
}

/// Build hierarchical outline from flat list of items
/// Note: pulldown-cmark gives us items in reverse document order
fn build_hierarchy(mut items: Vec<ListItem>) -> Vec<ListItem> {
    if items.is_empty() {
        return Vec::new();
    }

    // Reverse to get document order
    items.reverse();

    let mut result = Vec::new();
    let mut i = 0;

    while i < items.len() {
        if items[i].level == 0 {
            let (item, consumed) = build_item_with_children(&items, i);
            result.push(item);
            i += consumed;
        } else {
            i += 1; // Skip orphaned child items
        }
    }

    result
}

/// Build a single item with all its children recursively
fn build_item_with_children(items: &[ListItem], start_idx: usize) -> (ListItem, usize) {
    let mut item = items[start_idx].clone();
    let mut i = start_idx + 1;
    let target_child_level = item.level + 1;

    // Collect all immediate children
    while i < items.len() && items[i].level >= target_child_level {
        if items[i].level == target_child_level {
            let (child, consumed) = build_item_with_children(items, i);
            item.children.push(child);
            i += consumed;
        } else {
            i += 1; // Skip items at wrong nesting level
        }
    }

    (item, i - start_idx)
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
            // Note: pulldown-cmark processes items in reverse document order
            assert_eq!(items[0].content, "Second item");
            assert_eq!(items[1].content, "First item");
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
