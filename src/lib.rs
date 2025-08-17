pub mod io;
pub mod models;
pub mod parsing;
pub mod ui;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use models::{ContentBlock, Document, ListItem};

#[cfg(test)]
mod unit_tests {
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
    fn test_document_parsing_snapshots(#[case] markdown: &str, #[case] name: &str) {
        use std::path::PathBuf;
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));
        assert_yaml_snapshot!(name, doc.content);
    }

    #[test]
    fn test_simple_bullet_list_properties() {
        use std::path::PathBuf;
        let markdown = "- First item\n- Second item\n- Third item";
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 3);
            // Note: pulldown-cmark processes items in reverse document order
            assert_eq!(items[0].content, "Third item");
            assert_eq!(items[0].level, 0);
            assert_eq!(items[1].content, "Second item");
            assert_eq!(items[2].content, "First item");
        } else {
            panic!("Expected BulletList block");
        }
    }

    #[test]
    fn test_nested_bullet_list_properties() {
        use std::path::PathBuf;
        let markdown = "- Parent item\n  - Child item\n  - Another child\n- Second parent";
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));

        assert_eq!(doc.content.len(), 1);
        if let ContentBlock::BulletList { items } = &doc.content[0] {
            assert_eq!(items.len(), 2);
            // Note: pulldown-cmark processes items in reverse document order
            // Second parent comes first
            assert_eq!(items[0].content, "Second parent");
            assert_eq!(items[0].level, 0);
            assert_eq!(items[0].children.len(), 0);

            // First parent has children
            assert_eq!(items[1].content, "Parent item");
            assert_eq!(items[1].level, 0);
            assert_eq!(items[1].children.len(), 2);
            assert_eq!(items[1].children[0].content, "Another child");
            assert_eq!(items[1].children[0].level, 1);
            assert_eq!(items[1].children[1].content, "Child item");
        } else {
            panic!("Expected BulletList block");
        }
    }
}
