pub mod io;
pub mod models;
pub mod parsing;
pub mod ui;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use models::{Document, OutlineItem};

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
    fn test_outline_parsing_snapshots(#[case] markdown: &str, #[case] name: &str) {
        use std::path::PathBuf;
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));
        assert_yaml_snapshot!(name, doc.outline);
    }

    #[test]
    fn test_simple_bullet_list_properties() {
        use std::path::PathBuf;
        let markdown = "- First item\n- Second item\n- Third item";
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));

        assert_eq!(doc.outline.len(), 3);
        assert_eq!(doc.outline[0].content, "First item");
        assert_eq!(doc.outline[0].level, 0);
        assert_eq!(doc.outline[1].content, "Second item");
        assert_eq!(doc.outline[2].content, "Third item");
    }

    #[test]
    fn test_nested_bullet_list_properties() {
        use std::path::PathBuf;
        let markdown = "- Parent item\n  - Child item\n  - Another child\n- Second parent";
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));

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
