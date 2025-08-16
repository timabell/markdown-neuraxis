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
fn build_hierarchy(items: Vec<OutlineItem>) -> Vec<OutlineItem> {
    let mut result = Vec::new();
    let mut stack: Vec<OutlineItem> = Vec::new();
    let mut pending_children: Vec<OutlineItem> = Vec::new();

    for item in items {
        // If this is a child item, store it for later
        if item.level > 0 {
            pending_children.push(item);
            continue;
        }

        // This is a top-level item - add any pending children to the last parent
        if let Some(mut parent) = stack.pop() {
            parent.children = pending_children;
            pending_children = Vec::new();
            result.push(parent);
        }

        stack.push(item);
    }

    // Handle the last parent with any remaining children
    if let Some(mut parent) = stack.pop() {
        parent.children = pending_children;
        result.push(parent);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_list() {
        let content = "- First item\n- Second item";
        let doc = parse_markdown(content, PathBuf::from("/test.md"));

        assert_eq!(doc.outline.len(), 2);
        assert_eq!(doc.outline[0].content, "First item");
        assert_eq!(doc.outline[1].content, "Second item");
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
