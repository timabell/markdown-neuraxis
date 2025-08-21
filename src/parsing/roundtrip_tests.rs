//! Roundtrip tests for markdown parsing and generation.
//!
//! These tests ensure that content can be converted to markdown and back
//! without losing information.

use crate::models::ContentBlock;
use crate::parsing::from_markdown;

#[test]
fn test_roundtrip_conversion() {
    let original = ContentBlock::Heading {
        level: 1,
        text: "Main Title".to_string(),
    };
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_heading_levels() {
    for level in 1..=6 {
        let original = ContentBlock::Heading {
            level,
            text: format!("Heading Level {}", level),
        };
        let markdown = original.to_markdown();
        let converted = from_markdown(&markdown).unwrap();
        assert_eq!(original, converted);
    }
}

#[test]
fn test_roundtrip_bullet_list() {
    let original = ContentBlock::BulletList {
        items: vec![
            crate::models::ListItem::new("First item".to_string(), 0),
            crate::models::ListItem::new("Second item".to_string(), 0),
            crate::models::ListItem::new("Third item".to_string(), 0),
        ],
    };
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_numbered_list() {
    let original = ContentBlock::NumberedList {
        items: vec![
            crate::models::ListItem::new("First item".to_string(), 0),
            crate::models::ListItem::new("Second item".to_string(), 0),
            crate::models::ListItem::new("Third item".to_string(), 0),
        ],
    };
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_code_block() {
    let original = ContentBlock::CodeBlock {
        language: Some("rust".to_string()),
        code: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
    };
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_quote() {
    let original = ContentBlock::Quote("This is a quote".to_string());
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_rule() {
    let original = ContentBlock::Rule;
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_roundtrip_paragraph_with_wiki_links() {
    let original = ContentBlock::Paragraph {
        segments: vec![
            crate::models::TextSegment::Text("This is a ".to_string()),
            crate::models::TextSegment::WikiLink {
                target: "test-link".to_string(),
            },
            crate::models::TextSegment::Text(" paragraph.".to_string()),
        ],
    };
    let markdown = original.to_markdown();
    let converted = from_markdown(&markdown).unwrap();
    assert_eq!(original, converted);
}
