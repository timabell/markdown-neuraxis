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
    #[error("File already exists: {0}")]
    FileExists(PathBuf),
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

    // Create parent directories if they don't exist
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(IoError::Io)?;
    }

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

/// Rename/move a file to a new path, creating directories as needed
pub fn rename_file(
    old_relative_path: &RelativePath,
    new_relative_path: &RelativePath,
    notes_root: &Path,
) -> Result<(), IoError> {
    let old_abs_path = old_relative_path.to_path(notes_root);
    let new_abs_path = new_relative_path.to_path(notes_root);

    // Check if target already exists
    if new_abs_path.exists() {
        return Err(IoError::FileExists(new_abs_path));
    }

    // Create parent directories if needed
    if let Some(parent) = new_abs_path.parent() {
        fs::create_dir_all(parent).map_err(IoError::Io)?;
    }

    // Only rename if old file exists (for new unsaved files, this is a no-op)
    if old_abs_path.exists() {
        fs::rename(&old_abs_path, &new_abs_path).map_err(IoError::Io)?;

        // Clean up empty parent directories (like rmdir - fails safely if not empty)
        cleanup_empty_parents(&old_abs_path, notes_root);
    }

    Ok(())
}

/// Remove empty parent directories up to (but not including) notes_root.
fn cleanup_empty_parents(path: &Path, notes_root: &Path) {
    let mut current = path.parent();
    while let Some(parent) = current {
        // Stop at notes root
        if parent == notes_root {
            break;
        }
        // Check if directory is empty before attempting removal
        let is_empty = fs::read_dir(parent)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            break;
        }
        // Directory is empty, remove it
        let _ = fs::remove_dir(parent);
        current = parent.parent();
    }
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

    #[test]
    fn test_write_file_success() {
        let notes_dir = create_test_notes_dir();
        let relative_path = RelativePath::new("new_file.md");
        let content = "# New File\n\nThis is new content";

        // Write the file
        let result = write_file(relative_path, notes_dir.path(), content);
        assert!(result.is_ok());

        // Verify file exists and has correct content
        let written_content = read_file(relative_path, notes_dir.path()).unwrap();
        assert_eq!(written_content, content);
    }

    #[test]
    fn test_write_file_creates_parent_directories() {
        let notes_dir = create_test_notes_dir();
        let relative_path = RelativePath::new("folder/subfolder/new_file.md");
        let content = "# New File in Nested Folder";

        // Write the file - this should create the parent directories
        let result = write_file(relative_path, notes_dir.path(), content);
        assert!(result.is_ok());

        // Verify file exists and has correct content
        let written_content = read_file(relative_path, notes_dir.path()).unwrap();
        assert_eq!(written_content, content);

        // Verify parent directories were created
        let parent_dir = notes_dir.path().join("folder").join("subfolder");
        assert!(parent_dir.exists());
        assert!(parent_dir.is_dir());
    }

    #[test]
    fn test_write_file_overwrites_existing() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "existing.md", "# Original Content");

        let relative_path = RelativePath::new("existing.md");
        let new_content = "# Updated Content\n\nThis is new";

        // Overwrite the existing file
        let result = write_file(relative_path, notes_dir.path(), new_content);
        assert!(result.is_ok());

        // Verify content was updated
        let written_content = read_file(relative_path, notes_dir.path()).unwrap();
        assert_eq!(written_content, new_content);
    }

    #[test]
    fn test_rename_file_same_directory() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "old.md", "# Content");

        let old_path = RelativePath::new("old.md");
        let new_path = RelativePath::new("new.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_ok());

        // Old file should not exist
        assert!(!old_path.to_path(notes_dir.path()).exists());
        // New file should exist with same content
        let content = read_file(new_path, notes_dir.path()).unwrap();
        assert_eq!(content, "# Content");
    }

    #[test]
    fn test_rename_file_to_new_directory() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "root.md", "# Root Content");

        let old_path = RelativePath::new("root.md");
        let new_path = RelativePath::new("subfolder/moved.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_ok());

        // Old file should not exist
        assert!(!old_path.to_path(notes_dir.path()).exists());
        // New file should exist in new directory
        let content = read_file(new_path, notes_dir.path()).unwrap();
        assert_eq!(content, "# Root Content");
        // Directory should have been created
        assert!(notes_dir.path().join("subfolder").is_dir());
    }

    #[test]
    fn test_rename_file_nonexistent_is_ok() {
        // For new unsaved files, rename should succeed even if source doesn't exist
        let notes_dir = create_test_notes_dir();

        let old_path = RelativePath::new("nonexistent.md");
        let new_path = RelativePath::new("new.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_rename_file_fails_if_target_exists() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "source.md", "# Source");
        create_test_file(&notes_dir, "target.md", "# Target exists");

        let old_path = RelativePath::new("source.md");
        let new_path = RelativePath::new("target.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_err());
        assert!(matches!(result, Err(IoError::FileExists(_))));
    }

    #[test]
    fn test_rename_file_removes_empty_parent_folders() {
        let notes_dir = create_test_notes_dir();
        // Create a file nested in folders
        let sub_dir = notes_dir.path().join("folder").join("subfolder");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("file.md"), "# Content").unwrap();

        let old_path = RelativePath::new("folder/subfolder/file.md");
        let new_path = RelativePath::new("moved.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_ok());

        // Old file should be gone
        assert!(!sub_dir.join("file.md").exists());
        // Empty folders should be cleaned up
        assert!(!notes_dir.path().join("folder/subfolder").exists());
        assert!(!notes_dir.path().join("folder").exists());
    }

    #[test]
    fn test_rename_file_keeps_non_empty_parent_folders() {
        let notes_dir = create_test_notes_dir();
        // Create nested structure with another file
        let folder = notes_dir.path().join("folder");
        let sub_dir = folder.join("subfolder");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("file.md"), "# Content").unwrap();
        std::fs::write(folder.join("other.md"), "# Other").unwrap();

        let old_path = RelativePath::new("folder/subfolder/file.md");
        let new_path = RelativePath::new("moved.md");

        let result = rename_file(old_path, new_path, notes_dir.path());
        assert!(result.is_ok());

        // Empty subfolder should be removed
        assert!(!sub_dir.exists());
        // Parent folder with other file should remain
        assert!(folder.exists());
        assert!(folder.join("other.md").exists());
    }
}
