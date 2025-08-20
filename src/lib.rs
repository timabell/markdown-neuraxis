pub mod io;
pub mod models;
pub mod parsing;
pub mod ui;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use models::{BlockId, ContentBlock, Document, DocumentState, ListItem};

#[cfg(test)]
mod unit_tests {
    use super::*;
    use insta::assert_yaml_snapshot;
    use rstest::rstest;

    #[rstest]
    #[case(
        r#"- First item
- Second item
- Third item"#,
        "simple_bullet_list"
    )]
    #[case(
        r#"- Parent item
  - Child item
  - Another child
- Second parent"#,
        "nested_bullet_list"
    )]
    #[case("- Single item", "single_item")]
    #[case("", "empty_markdown")]
    #[case(
        r#"- Item 1
  - Item 1.1
    - Item 1.1.1
      - Item 1.1.1.1
      - Item 1.1.1.2
    - Item 1.1.2
  - Item 1.2
    - Item 1.2.1
- Item 2
  - Item 2.1
- Item 3"#,
        "deep_nested_list"
    )]
    #[case(
        "This paragraph has [[Simple-Link]] and [[Folder/Page]] references.",
        "wiki_links_paragraph"
    )]
    #[case(
        r#"# Heading

Paragraph with [[Getting-Started]] link.

- List item with [[journal/2024-01-15]] reference
- Another item with [[1_Projects/Website]] link"#,
        "wiki_links_mixed"
    )]
    fn test_document_parsing_snapshots(#[case] markdown: &str, #[case] name: &str) {
        use std::path::PathBuf;
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));
        assert_yaml_snapshot!(name, doc.content);
    }
}
