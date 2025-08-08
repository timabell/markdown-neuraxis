use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlineItem {
    pub content: String,
    pub level: usize,
    pub children: Vec<OutlineItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub outline: Vec<OutlineItem>,
}

pub fn parse_markdown_outline(markdown: &str) -> Document {
    let parser = Parser::new(markdown);
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
                        let item = OutlineItem {
                            content: text.trim().to_string(),
                            level,
                            children: Vec::new(),
                        };
                        items.push(item);
                    }
                }
            }
            _ => {}
        }
    }

    let outline = build_hierarchy(items);
    Document { outline }
}

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
    use insta::assert_yaml_snapshot;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("- First item\n- Second item\n- Third item", "simple_bullet_list")]
    #[case(
        "- Parent item\n  - Child item\n  - Another child\n- Second parent",
        "nested_bullet_list"
    )]
    #[case("- Single item", "single_item")]
    #[case("", "empty_markdown")]
    fn test_outline_parsing_snapshots(#[case] markdown: &str, #[case] name: &str) {
        let doc = parse_markdown_outline(markdown);
        assert_yaml_snapshot!(name, doc.outline);
    }

    #[test]
    fn test_simple_bullet_list_properties() {
        let markdown = "- First item\n- Second item\n- Third item";
        let doc = parse_markdown_outline(markdown);

        assert_eq!(doc.outline.len(), 3);
        assert_eq!(doc.outline[0].content, "First item");
        assert_eq!(doc.outline[0].level, 0);
        assert_eq!(doc.outline[1].content, "Second item");
        assert_eq!(doc.outline[2].content, "Third item");
    }

    #[test]
    fn test_nested_bullet_list_properties() {
        let markdown = "- Parent item\n  - Child item\n  - Another child\n- Second parent";
        let doc = parse_markdown_outline(markdown);

        assert_eq!(doc.outline.len(), 2);
        assert_eq!(doc.outline[0].content, "Parent item");
        assert_eq!(doc.outline[0].level, 0);
        assert_eq!(doc.outline[0].children.len(), 2);
        assert_eq!(doc.outline[0].children[0].content, "Child item");
        assert_eq!(doc.outline[0].children[0].level, 1);
        assert_eq!(doc.outline[1].content, "Second parent");
        assert_eq!(doc.outline[1].level, 0);
    }
}
