use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary notes directory with test files
pub fn create_test_notes_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Create a test markdown file with content
pub fn create_test_file(notes_dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let file_path = notes_dir.path().join(filename);
    fs::write(&file_path, content).unwrap();
    file_path
}
