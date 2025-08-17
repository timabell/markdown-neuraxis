use crate::models::{ContentBlock, Document, ListItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

/// Parse markdown content into a complete Document structure
pub fn parse_markdown(content: &str, path: PathBuf) -> Document {
    let parser = Parser::new(content);
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut current_text = String::new();

    // Track text for the current item being built
    let mut current_item_text = String::new();
    let mut in_item = false;

    // Track code block state
    let mut in_code_block = false;
    let mut code_language: Option<String> = None;
    let mut code_content = String::new();

    // Use flat collection with post-processing approach
    let mut flat_items: Vec<(String, usize)> = Vec::new(); // (content, depth)
    let mut current_depth = 0;
    let mut is_numbered = false;

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
                
                if current_depth == 0 {
                    is_numbered = first_item.is_some();
                }
                current_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                current_depth -= 1;
                
                if current_depth == 0 {
                    // Build hierarchy and add to blocks
                    let hierarchy = build_hierarchy(&flat_items);
                    if !hierarchy.is_empty() {
                        if is_numbered {
                            blocks.push(ContentBlock::NumberedList { items: hierarchy });
                        } else {
                            blocks.push(ContentBlock::BulletList { items: hierarchy });
                        }
                    }
                    flat_items.clear();
                }
            }
            Event::Start(Tag::Item) => {
                current_item_text.clear();
                in_item = true;
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                if !current_item_text.trim().is_empty() {
                    flat_items.push((current_item_text.trim().to_string(), current_depth - 1));
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
                    current_item_text.push_str(&text);
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_content.push('\n');
                } else if in_item {
                    current_item_text.push('\n');
                } else {
                    current_text.push('\n');
                }
            }
            _ => {}
        }
    }

    // Handle any remaining flat items
    if !flat_items.is_empty() {
        let hierarchy = build_hierarchy(&flat_items);
        if !hierarchy.is_empty() {
            blocks.push(ContentBlock::BulletList { items: hierarchy });
        }
    }

    // Flush any remaining text as a paragraph
    flush_text(&mut current_text, &mut blocks);

    Document::with_content(path, blocks)
}

fn build_hierarchy(flat_items: &[(String, usize)]) -> Vec<ListItem> {
    let mut result = Vec::new();
    let mut stack: Vec<&mut ListItem> = Vec::new();

    for (content, depth) in flat_items {
        let item = ListItem::new(content.clone(), *depth);

        // Adjust stack to current depth
        while stack.len() > *depth {
            stack.pop();
        }

        if *depth == 0 {
            // Top-level item
            result.push(item);
            // We can't keep a mutable reference to the item we just moved
            // Use a different approach: recursive building
        } else {
            // Need a completely different approach
            // For now, return a simple working version
        }
    }

    // Simple recursive approach that actually works
    build_recursive(flat_items, 0).0
}

fn build_recursive(items: &[(String, usize)], start_idx: usize) -> (Vec<ListItem>, usize) {
    let mut result = Vec::new();
    let mut i = start_idx;
    let expected_depth = if start_idx < items.len() { items[start_idx].1 } else { 0 };

    while i < items.len() {
        let (content, depth) = &items[i];
        
        if *depth == expected_depth {
            // This item belongs at the current level
            let mut item = ListItem::new(content.clone(), *depth);
            i += 1;

            // Check if next items are children (deeper depth)
            if i < items.len() && items[i].1 > *depth {
                let (children, next_i) = build_recursive(items, i);
                item.children = children;
                i = next_i;
            }

            result.push(item);
        } else if *depth < expected_depth {
            // This item belongs to a parent level, return control
            break;
        } else {
            // Deeper than expected, shouldn't happen
            i += 1;
        }
    }

    (result, i)
}

fn add_child_to_item(items: &mut Vec<ListItem>, path: &[usize], child: ListItem) {
    if path.is_empty() {
        return;
    }

    let mut current = &mut items[path[0]];
    for &idx in &path[1..] {
        current = &mut current.children[idx];
    }
    current.children.push(child);
}

fn get_last_child_index(items: &[ListItem], path: &[usize]) -> usize {
    if path.is_empty() {
        return 0;
    }

    let mut current = &items[path[0]];
    for &idx in &path[1..] {
        current = &current.children[idx];
    }
    current.children.len().saturating_sub(1)
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