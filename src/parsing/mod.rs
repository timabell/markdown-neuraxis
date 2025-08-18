use crate::models::{ContentBlock, Document, ListItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

/// Parse markdown content into a complete Document structure
pub fn parse_markdown(content: &str, path: PathBuf) -> Document {
    let parser = Parser::new(content);
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut current_text = String::new();
    let mut list_stack: Vec<(Vec<ListItem>, bool)> = Vec::new(); // (items, is_ordered)
    let mut item_stack: Vec<(String, Vec<ListItem>)> = Vec::new(); // (text, children)
    let mut in_item = false;
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
                // Starting a new list
                if list_stack.is_empty() {
                    // Top-level list
                    flush_text(&mut current_text, &mut blocks);
                }
                list_stack.push((Vec::new(), first_item.is_some()));
            }
            Event::End(TagEnd::List(_)) => {
                // Ending a list
                if let Some((items, is_ordered)) = list_stack.pop() {
                    if list_stack.is_empty() {
                        // This is a top-level list, add it to blocks
                        if !items.is_empty() {
                            if is_ordered {
                                blocks.push(ContentBlock::NumberedList { items });
                            } else {
                                blocks.push(ContentBlock::BulletList { items });
                            }
                        }
                    } else {
                        // This is a nested list, save it as children for the parent item
                        if let Some((_, children)) = item_stack.last_mut() {
                            *children = items;
                        }
                    }
                }
            }
            Event::Start(Tag::Item) => {
                // Starting a new item
                item_stack.push((String::new(), Vec::new()));
                in_item = true;
            }
            Event::End(TagEnd::Item) => {
                // Ending an item
                in_item = false;
                if let Some((text, children)) = item_stack.pop() {
                    let level = list_stack.len().saturating_sub(1);
                    let mut item = ListItem::new(text.trim().to_string(), level);
                    item.children = children;

                    // Add this item to the current list
                    if let Some((items, _)) = list_stack.last_mut() {
                        items.push(item);
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
                    // Add text to the current item
                    if let Some((item_text, _)) = item_stack.last_mut() {
                        item_text.push_str(&text);
                    }
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if in_item {
                    if let Some((item_text, _)) = item_stack.last_mut() {
                        item_text.push('\n');
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
