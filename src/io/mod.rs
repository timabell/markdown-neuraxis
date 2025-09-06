use crate::models::FileTree;
use relative_path::RelativePath;
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
pub fn read_file(relative_path: &RelativePath, notes_root: &Path) -> Result<String, IoError> {
    let absolute_path = relative_path.to_path(notes_root);
    if !absolute_path.exists() {
        return Err(IoError::NotFound(absolute_path));
    }
    fs::read_to_string(&absolute_path).map_err(IoError::Io)
}

/// Write content to a markdown file
pub fn write_file(
    relative_path: &RelativePath,
    notes_root: &Path,
    content: &str,
) -> Result<(), IoError> {
    let absolute_path = relative_path.to_path(notes_root);
    fs::write(&absolute_path, content).map_err(IoError::Io)
}

/// Scan for markdown files in the notes directory
pub fn scan_markdown_files(notes_root: &Path) -> Result<Vec<PathBuf>, IoError> {
    if !notes_root.exists() {
        return Err(IoError::InvalidNotesDir(
            "notes directory not found".to_string(),
        ));
    }

    let mut files = Vec::new();
    scan_directory_recursive(notes_root, &mut files)?;
    files.sort();
    Ok(files)
}

/// Build a file tree from markdown files in the notes directory
pub fn build_file_tree(notes_root: &Path) -> Result<FileTree, IoError> {
    if !notes_root.exists() {
        return Err(IoError::InvalidNotesDir(
            "notes directory not found".to_string(),
        ));
    }

    let files = scan_markdown_files(notes_root)?;
    Ok(FileTree::build_from_files(notes_root.to_path_buf(), &files))
}

fn scan_directory_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), IoError> {
    let entries = fs::read_dir(dir).map_err(IoError::Io)?;

    for entry in entries {
        let entry = entry.map_err(IoError::Io)?;
        let path = entry.path();

        if path.is_dir() {
            scan_directory_recursive(&path, files)?;
        } else if let Some(ext) = path.extension()
            && ext == "md"
        {
            files.push(path);
        }
    }

    Ok(())
}

pub fn validate_notes_dir(path: &Path) -> Result<(), IoError> {
    if !path.exists() || !path.is_dir() {
        return Err(IoError::InvalidNotesDir(
            "Directory does not exist".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{create_test_file, create_test_notes_dir};

    #[test]
    fn test_scan_and_load_files() {
        // Given a notes directory with markdown files
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "test1.md", "- First item\n- Second item");
        create_test_file(&notes_dir, "test2.md", "- Parent\n  - Child");

        // When scanning for files
        let files = scan_markdown_files(notes_dir.path()).unwrap();

        // Then we find the expected files
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "test1.md"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "test2.md"));
    }

    #[test]
    fn test_handle_invalid_notes_directory() {
        let nonexistent_path = PathBuf::from("/this/path/does/not/exist");

        let result = scan_markdown_files(&nonexistent_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("notes directory"));
    }

    #[test]
    fn test_scan_nested_directories() {
        // Given a notes directory with nested structure
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "root.md", "# Root file");

        // Create nested directory structure
        let sub_dir = notes_dir.path().join("subfolder");
        std::fs::create_dir(&sub_dir).unwrap();
        let nested_file = sub_dir.join("nested.md");
        std::fs::write(&nested_file, "# Nested file").unwrap();

        // When scanning for files
        let files = scan_markdown_files(notes_dir.path()).unwrap();

        // Then we find both root and nested files
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "root.md"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "nested.md"));
    }

    #[test]
    fn test_ignore_non_markdown_files() {
        // Given a notes directory with mixed file types
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "document.md", "# Markdown");
        create_test_file(&notes_dir, "image.png", "fake image data");
        create_test_file(&notes_dir, "config.json", "{}");

        // When scanning for files
        let files = scan_markdown_files(notes_dir.path()).unwrap();

        // Then we only find markdown files
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "document.md");
    }

    #[test]
    fn test_validate_notes_dir_exists() {
        let notes_dir = create_test_notes_dir();
        let result = validate_notes_dir(notes_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_notes_dir_not_exists() {
        let result = validate_notes_dir(Path::new("/nonexistent/path"));
        assert!(result.is_err());
        assert!(matches!(result, Err(IoError::InvalidNotesDir(_))));
    }

    #[test]
    fn test_read_file_success() {
        let notes_dir = create_test_notes_dir();
        let _file_path = create_test_file(&notes_dir, "test.md", "# Test Content\n\nParagraph");

        let relative_path = RelativePath::new("test.md");
        let content = read_file(relative_path, notes_dir.path()).unwrap();
        assert_eq!(content, "# Test Content\n\nParagraph");
    }

    #[test]
    fn test_read_file_not_found() {
        let notes_dir = create_test_notes_dir();
        let relative_path = RelativePath::new("nonexistent.md");
        let result = read_file(relative_path, notes_dir.path());
        assert!(result.is_err());
        assert!(matches!(result, Err(IoError::NotFound(_))));
    }
}
