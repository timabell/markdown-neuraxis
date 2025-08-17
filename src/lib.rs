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
    #[case(
        "- Level 0 Item A\n  - Level 1 Item A1\n    - Level 2 Item A1a\n      - Level 3 Item A1a1\n      - Level 3 Item A1a2\n    - Level 2 Item A1b\n  - Level 1 Item A2\n    - Level 2 Item A2a\n- Level 0 Item B\n  - Level 1 Item B1\n- Level 0 Item C",
        "deep_nested_list"
    )]
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
            assert_eq!(items[0].content, "First item");
            assert_eq!(items[0].level, 0);
            assert_eq!(items[1].content, "Second item");
            assert_eq!(items[2].content, "Third item");
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
            // First parent has children (in document order)
            assert_eq!(items[0].content, "Parent item");
            assert_eq!(items[0].level, 0);
            assert_eq!(items[0].children.len(), 2);
            assert_eq!(items[0].children[0].content, "Child item");
            assert_eq!(items[0].children[0].level, 1);
            assert_eq!(items[0].children[1].content, "Another child");

            // Second parent has no children
            assert_eq!(items[1].content, "Second parent");
            assert_eq!(items[1].level, 0);
            assert_eq!(items[1].children.len(), 0);
        } else {
            panic!("Expected BulletList block");
        }
    }
}
