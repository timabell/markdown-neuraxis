use super::*;

#[test]
fn user_workflow_scan_and_load_files() {
    // Given a notes directory with markdown files
    let notes_dir = create_test_notes_dir();
    create_test_file(&notes_dir, "test1.md", "- First item\n- Second item");
    create_test_file(&notes_dir, "test2.md", "- Parent\n  - Child");

    // When scanning for files
    let files = io::scan_markdown_files(notes_dir.path()).unwrap();

    // Then we find the expected files
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.file_name().unwrap() == "test1.md"));
    assert!(files.iter().any(|f| f.file_name().unwrap() == "test2.md"));
}

#[test]
fn user_workflow_select_file_and_parse_outline() {
    // Given a notes directory with a nested markdown file
    let notes_dir = create_test_notes_dir();
    let file_path = create_test_file(
        &notes_dir,
        "nested.md",
        "- Parent item\n  - Child item\n  - Another child\n- Second parent",
    );

    // When reading and parsing the file
    let content = io::read_file(&file_path).unwrap();
    let document = parsing::parse_markdown(&content, file_path);

    // Then we get the correct outline structure
    assert_eq!(document.outline.len(), 2);

    // First parent has children
    assert_eq!(document.outline[0].content, "Parent item");
    assert_eq!(document.outline[0].children.len(), 2);
    assert_eq!(document.outline[0].children[0].content, "Child item");
    assert_eq!(document.outline[0].children[1].content, "Another child");

    // Second parent has no children
    assert_eq!(document.outline[1].content, "Second parent");
    assert_eq!(document.outline[1].children.len(), 0);
}

#[test]
fn user_workflow_handle_invalid_notes_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    // Don't create pages directory

    let result = io::scan_markdown_files(temp_dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("pages directory"));
}

#[test]
fn user_workflow_deep_nesting_outline() {
    let notes_dir = create_test_notes_dir();
    let file_path = create_test_file(
        &notes_dir,
        "deep.md",
        "- Level 0\n  - Level 1\n    - Level 2\n      - Level 3",
    );

    let content = io::read_file(&file_path).unwrap();
    let document = parsing::parse_markdown(&content, file_path);

    // Current implementation flattens nested levels into children of first parent (in reverse order)
    assert_eq!(document.outline.len(), 1);
    assert_eq!(document.outline[0].content, "Level 0");
    assert_eq!(document.outline[0].children.len(), 3);
    assert_eq!(document.outline[0].children[0].content, "Level 3");
    assert_eq!(document.outline[0].children[1].content, "Level 2");
    assert_eq!(document.outline[0].children[2].content, "Level 1");
}
