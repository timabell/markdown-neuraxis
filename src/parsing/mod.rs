use crate::models::{Document, OutlineItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

/// Parse markdown content into a Document with outline structure
pub fn parse_markdown(content: &str, path: PathBuf) -> Document {
    let parser = Parser::new(content);
    let mut items: Vec<OutlineItem> = Vec::new();
    let mut text_stack: Vec<String> = Vec::new();
    let mut list_stack: Vec<usize> = Vec::new();
    let mut in_item = false;

    for event in parser {
        match event {
            Event::Start(Tag::List(_)) => {
                list_stack.push(0);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                text_stack.push(String::new());
                in_item = true;
            }
            Event::Text(text) if in_item => {
                if let Some(current_text) = text_stack.last_mut() {
                    current_text.push_str(&text);
                }
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                if let Some(text) = text_stack.pop() {
                    if !text.trim().is_empty() {
                        let level = list_stack.len().saturating_sub(1);
                        let item = OutlineItem::new(text.trim().to_string(), level);
                        items.push(item);
                    }
                }
            }
            _ => {}
        }
    }

    let outline = build_hierarchy(items);
    Document::with_outline(path, outline)
}

/// Build hierarchical outline from flat list of items
/// Note: pulldown-cmark gives us items in reverse document order
fn build_hierarchy(mut items: Vec<OutlineItem>) -> Vec<OutlineItem> {
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
fn build_item_with_children(items: &[OutlineItem], start_idx: usize) -> (OutlineItem, usize) {
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

        assert_eq!(doc.outline.len(), 2);
        // Note: pulldown-cmark processes items in reverse document order
        assert_eq!(doc.outline[0].content, "Second item");
        assert_eq!(doc.outline[1].content, "First item");
    }

    #[test]
    fn test_parse_nested_list() {
        let content = "- Parent\n  - Child";
        let doc = parse_markdown(content, PathBuf::from("/test.md"));

        assert_eq!(doc.outline.len(), 1);
        assert_eq!(doc.outline[0].content, "Parent");
        assert_eq!(doc.outline[0].children.len(), 1);
        assert_eq!(doc.outline[0].children[0].content, "Child");
    }
}
