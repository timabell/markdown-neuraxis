//! Snapshot tests for markdown parsing.
//!
//! These tests use insta to verify that parsing produces the expected output
//! for various markdown inputs.

use super::parse_markdown;
use insta::assert_yaml_snapshot;
use regex::Regex;
use relative_path::RelativePathBuf;
use rstest::rstest;

/// Create a normalized snapshot value with UUIDs replaced by a placeholder
fn create_normalized_snapshot(content: &[crate::models::ContentBlock]) -> serde_yaml::Value {
    // Serialize to YAML string first
    let yaml_str = serde_yaml::to_string(content).expect("Failed to serialize to YAML");

    // Replace all UUIDs with a simple placeholder
    let uuid_regex =
        Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let normalized_yaml = uuid_regex.replace_all(&yaml_str, "<some-uuid>");

    // Parse back to serde_yaml::Value for snapshot comparison
    serde_yaml::from_str(&normalized_yaml).expect("Failed to parse normalized YAML")
}

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
    let doc = parse_markdown(markdown, RelativePathBuf::from("test.md"));
    let normalized_content = create_normalized_snapshot(&doc.content);
    assert_yaml_snapshot!(name, normalized_content);
}
