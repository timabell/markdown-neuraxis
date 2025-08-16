pub mod integration;

use crate::{io, parsing};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary notes directory with test files
pub fn create_test_notes_dir() -> TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    let pages_dir = temp_dir.path().join("pages");
    fs::create_dir(&pages_dir).unwrap();
    temp_dir
}

/// Create a test markdown file with content
pub fn create_test_file(notes_dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let file_path = notes_dir.path().join("pages").join(filename);
    fs::write(&file_path, content).unwrap();
    file_path
}
