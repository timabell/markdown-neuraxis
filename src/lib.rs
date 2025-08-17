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
    #[case(r#"- First item
- Second item
- Third item"#, "simple_bullet_list")]
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
    fn test_document_parsing_snapshots(#[case] markdown: &str, #[case] name: &str) {
        use std::path::PathBuf;
        let doc = parsing::parse_markdown(markdown, PathBuf::from("test.md"));
        assert_yaml_snapshot!(name, doc.content);
    }
}
