use crate::models::FileTree;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid notes directory: {0}")]
    InvalidNotesDir(String),
}

/// Read a markdown file and return its content
pub fn read_file(path: &Path) -> Result<String, IoError> {
    if !path.exists() {
        return Err(IoError::NotFound(path.to_path_buf()));
    }
    fs::read_to_string(path).map_err(IoError::Io)
}

/// Write content to a markdown file
pub fn write_file(path: &Path, content: &str) -> Result<(), IoError> {
    fs::write(path, content).map_err(IoError::Io)
}

/// Scan for markdown files in the notes directory
pub fn scan_markdown_files(notes_root: &Path) -> Result<Vec<PathBuf>, IoError> {
    let pages_dir = notes_root.join("pages");
    if !pages_dir.exists() {
        return Err(IoError::InvalidNotesDir(
            "pages directory not found".to_string(),
        ));
    }

    let mut files = Vec::new();
    scan_directory_recursive(&pages_dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Build a file tree from markdown files in the notes directory
pub fn build_file_tree(notes_root: &Path) -> Result<FileTree, IoError> {
    let pages_dir = notes_root.join("pages");
    if !pages_dir.exists() {
        return Err(IoError::InvalidNotesDir(
            "pages directory not found".to_string(),
        ));
    }

    let files = scan_markdown_files(notes_root)?;
    Ok(FileTree::build_from_files(pages_dir, &files))
}

fn scan_directory_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), IoError> {
    let entries = fs::read_dir(dir).map_err(IoError::Io)?;

    for entry in entries {
        let entry = entry.map_err(IoError::Io)?;
        let path = entry.path();

        if path.is_dir() {
            scan_directory_recursive(&path, files)?;
        } else if let Some(ext) = path.extension() {
            if ext == "md" {
                files.push(path);
            }
        }
    }

    Ok(())
}

/// Validate that a directory has the expected notes structure
pub fn validate_notes_dir(path: &Path) -> Result<(), IoError> {
    if !path.exists() || !path.is_dir() {
        return Err(IoError::InvalidNotesDir(
            "Directory does not exist".to_string(),
        ));
    }

    let pages_dir = path.join("pages");
    if !pages_dir.exists() {
        return Err(IoError::InvalidNotesDir(
            "pages directory not found".to_string(),
        ));
    }

    Ok(())
}
