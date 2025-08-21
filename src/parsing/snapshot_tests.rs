//! Snapshot tests for markdown parsing.
//!
//! These tests use insta to verify that parsing produces the expected output
//! for various markdown inputs.

use super::parse_markdown;
use insta::assert_yaml_snapshot;
use rstest::rstest;
use std::path::PathBuf;

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
    let doc = parse_markdown(markdown, PathBuf::from("test.md"));
    assert_yaml_snapshot!(name, doc.content);
}
