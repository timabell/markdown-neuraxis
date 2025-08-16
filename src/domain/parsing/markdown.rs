use crate::domain::models::{Document, OutlineItem};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

pub trait MarkdownParser: Send + Sync {
    fn parse(&self, content: &str, path: PathBuf) -> Document;
}

#[derive(Debug, Default)]
pub struct PulldownMarkdownParser;

impl PulldownMarkdownParser {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MarkdownParser for PulldownMarkdownParser {
    fn parse(&self, content: &str, path: PathBuf) -> Document {
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

        let outline = super::outline::build_hierarchy(items);
        Document::with_outline(path, outline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_simple_markdown() {
        let parser = PulldownMarkdownParser::new();
        let content = "- First item\n- Second item";
        let doc = parser.parse(content, PathBuf::from("/test.md"));

        assert_eq!(doc.outline.len(), 2);
        assert_eq!(doc.outline[0].content, "First item");
        assert_eq!(doc.outline[1].content, "Second item");
    }

    #[test]
    fn test_parse_nested_markdown() {
        let parser = PulldownMarkdownParser::new();
        let content = "- Parent\n  - Child";
        let doc = parser.parse(content, PathBuf::from("/test.md"));

        assert_eq!(doc.outline.len(), 1);
        assert_eq!(doc.outline[0].content, "Parent");
        assert_eq!(doc.outline[0].children.len(), 1);
        assert_eq!(doc.outline[0].children[0].content, "Child");
    }
}
