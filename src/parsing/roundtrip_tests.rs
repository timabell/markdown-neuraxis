//! Roundtrip tests for markdown parsing and generation.
//!
//! These tests ensure that markdown can be parsed and converted back to
//! markdown without losing information or introducing formatting changes.

use crate::models::Document;
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

            use relative_path::RelativePathBuf;
            let blocks = crate::parsing::parse_multiple_blocks(&original_markdown);
            let parsed_doc = Document::with_content(RelativePathBuf::from(file_name), blocks);

            let regenerated_markdown = parsed_doc
                .content
                .iter()
                .map(|block| block.to_markdown())
                .collect::<Vec<_>>()
                .join("\n");

            assert_eq!(
                original_markdown.trim(),
                regenerated_markdown.trim(),
                "Roundtrip failed for {file_name}"
            );
        }
    }
}
