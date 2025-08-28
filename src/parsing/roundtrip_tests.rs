//! Roundtrip tests for markdown parsing and generation.
//!
//! These tests ensure that markdown can be parsed and converted back to
//! markdown without losing information or introducing formatting changes.

use crate::parsing::from_markdown;
use std::fs;
use std::path::Path;

#[test]
fn test_roundtrip_all_test_files() {
    let test_data_dir = Path::new("src/parsing/test_data");

    let entries = fs::read_dir(test_data_dir).expect("Failed to read test data directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            println!("Testing roundtrip for: {file_name}");

            let original_markdown =
                fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {file_name}"));

            let parsed = from_markdown(&original_markdown)
                .unwrap_or_else(|_| panic!("Failed to parse {file_name}"));

            let regenerated_markdown = parsed.to_markdown();

            assert_eq!(
                original_markdown.trim(),
                regenerated_markdown.trim(),
                "Roundtrip failed for {file_name}"
            );
        }
    }
}
