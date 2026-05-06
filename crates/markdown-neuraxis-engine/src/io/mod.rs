use crate::models::FileTree;
use relative_path::{RelativePath, RelativePathBuf};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

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

/// Metadata of an open file used to detect external changes (ADR-0014).
///
/// The `(mtime, size)` pair is the canonical change signal; if either
/// differs from the value captured at load time, the file has been
/// modified by another process (or another device's Syncthing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFileMeta {
    pub mtime: SystemTime,
    pub size: u64,
}

impl OpenFileMeta {
    fn from_metadata(meta: &fs::Metadata) -> std::io::Result<Self> {
        Ok(Self {
            mtime: meta.modified()?,
            size: meta.len(),
        })
    }
}

/// Outcome of a guarded write that aborted because the file changed on
/// disk between load and save.
#[derive(Debug, thiserror::Error)]
pub enum GuardedWriteError {
    /// The on-disk file no longer matches the meta the caller loaded with.
    /// The current disk content and its meta are returned so the caller
    /// can present the user with the same banner the watcher would.
    #[error("File changed externally before save")]
    ExternalChange { bytes: Vec<u8>, meta: OpenFileMeta },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File not found: {0}")]
    NotFound(PathBuf),
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

/// Read a file and return both its content and `OpenFileMeta`.
pub fn read_file_with_meta(
    relative_path: &RelativePath,
    notes_root: &Path,
) -> Result<(String, OpenFileMeta), IoError> {
    let absolute_path = relative_path.to_path(notes_root);
    if !absolute_path.exists() {
        return Err(IoError::NotFound(absolute_path));
    }
    let content = fs::read_to_string(&absolute_path).map_err(IoError::Io)?;
    let meta = fs::metadata(&absolute_path).map_err(IoError::Io)?;
    let open_meta = OpenFileMeta::from_metadata(&meta).map_err(IoError::Io)?;
    Ok((content, open_meta))
}

/// Write `content` to `relative_path` only if the file's current
/// `(mtime, size)` still matches `expected_meta` (ADR-0014 §5).
///
/// On success returns the post-write `OpenFileMeta`. On detected
/// external change returns `GuardedWriteError::ExternalChange` carrying
/// the on-disk bytes so the caller can route them into the same banner
/// state the watcher uses.
pub fn write_file_guarded(
    relative_path: &RelativePath,
    notes_root: &Path,
    content: &str,
    expected_meta: OpenFileMeta,
) -> Result<OpenFileMeta, GuardedWriteError> {
    let absolute_path = relative_path.to_path(notes_root);

    // If the file no longer exists or its meta drifted, treat as external
    // change and surface the disk version (or empty bytes if absent).
    match fs::metadata(&absolute_path) {
        Ok(disk_meta) => {
            let current = OpenFileMeta::from_metadata(&disk_meta)?;
            if current != expected_meta {
                let bytes = fs::read(&absolute_path).unwrap_or_default();
                return Err(GuardedWriteError::ExternalChange {
                    bytes,
                    meta: current,
                });
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(GuardedWriteError::NotFound(absolute_path));
        }
        Err(e) => return Err(GuardedWriteError::Io(e)),
    }

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&absolute_path, content)?;
    let written_meta = fs::metadata(&absolute_path)?;
    Ok(OpenFileMeta::from_metadata(&written_meta)?)
}

const CONFLICT_TIMESTAMP_FMT: &[FormatItem<'static>] =
    format_description!("[year][month][day]-[hour][minute][second]");

/// Build a conflict filename for `original` (ADR-0014 §6).
///
/// Format: `{stem}.mdnx-conflict-YYYYMMDD-HHMMSS-{device}.{ext}` (UTC).
/// The `mdnx-conflict-` prefix is deliberately distinct from Syncthing's
/// own `.sync-conflict-` pattern so a markdown-neuraxis save-race
/// conflict cannot be confused with a Syncthing device-level conflict.
pub fn conflict_path(original: &RelativePath, when: SystemTime, device: &str) -> RelativePathBuf {
    let parent = original.parent();
    let stem = original.file_stem().unwrap_or("");
    let extension = original.extension();

    let datetime: OffsetDateTime = when.into();
    let timestamp = datetime
        .format(&CONFLICT_TIMESTAMP_FMT)
        .expect("static format description always succeeds for OffsetDateTime");

    let safe_device = sanitize_device(device);
    let new_stem = format!("{stem}.mdnx-conflict-{timestamp}-{safe_device}");

    let new_name = match extension {
        Some(ext) => format!("{new_stem}.{ext}"),
        None => new_stem,
    };

    // RelativePath::parent() returns Some("") for top-level files; joining
    // an empty parent would yield a leading `/`, so fall back to the bare
    // name in that case.
    match parent {
        Some(p) if !p.as_str().is_empty() => p.join(new_name),
        _ => RelativePathBuf::from(new_name),
    }
}

fn sanitize_device(device: &str) -> String {
    let cleaned: String = device
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        String::from("device")
    } else {
        cleaned
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

    // --- conflict_path ---------------------------------------------------

    fn fixed_time() -> SystemTime {
        // 2026-05-06 14:30:15 UTC
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_777_811_415)
    }

    #[test]
    fn test_conflict_path_simple_file() {
        let original = RelativePath::new("notes.md");
        let result = conflict_path(original, fixed_time(), "laptop");
        assert_eq!(
            result.as_str(),
            "notes.mdnx-conflict-20260506-143015-laptop.md"
        );
    }

    #[test]
    fn test_conflict_path_nested() {
        let original = RelativePath::new("1_Projects/sub/notes.md");
        let result = conflict_path(original, fixed_time(), "laptop");
        assert_eq!(
            result.as_str(),
            "1_Projects/sub/notes.mdnx-conflict-20260506-143015-laptop.md"
        );
    }

    #[test]
    fn test_conflict_path_no_extension() {
        let original = RelativePath::new("notes");
        let result = conflict_path(original, fixed_time(), "laptop");
        assert_eq!(
            result.as_str(),
            "notes.mdnx-conflict-20260506-143015-laptop"
        );
    }

    #[test]
    fn test_conflict_path_multiple_dots() {
        // Only the last extension component is split off.
        let original = RelativePath::new("notes.backup.md");
        let result = conflict_path(original, fixed_time(), "laptop");
        assert_eq!(
            result.as_str(),
            "notes.backup.mdnx-conflict-20260506-143015-laptop.md"
        );
    }

    #[test]
    fn test_conflict_path_empty_device_falls_back() {
        let original = RelativePath::new("notes.md");
        let result = conflict_path(original, fixed_time(), "");
        assert_eq!(
            result.as_str(),
            "notes.mdnx-conflict-20260506-143015-device.md"
        );
    }

    #[test]
    fn test_conflict_path_non_ascii_device() {
        // Non-ASCII chars get replaced so the filename stays portable
        // across filesystems (FAT/SD-card on Android in particular).
        let original = RelativePath::new("notes.md");
        let result = conflict_path(original, fixed_time(), "ümlaut");
        // 'ü' is one Unicode scalar (U+00FC); it is replaced with '-'.
        assert_eq!(
            result.as_str(),
            "notes.mdnx-conflict-20260506-143015--mlaut.md"
        );
    }

    #[test]
    fn test_conflict_path_sanitizes_device() {
        let original = RelativePath::new("notes.md");
        let result = conflict_path(original, fixed_time(), "Tim's Laptop / 2");
        // spaces, apostrophes, slashes → '-'
        assert_eq!(
            result.as_str(),
            "notes.mdnx-conflict-20260506-143015-Tim-s-Laptop---2.md"
        );
    }

    // --- read_file_with_meta / write_file_guarded ------------------------

    #[test]
    fn test_read_file_with_meta_returns_content_and_meta() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "test.md", "hello");

        let (content, meta) =
            read_file_with_meta(RelativePath::new("test.md"), notes_dir.path()).unwrap();
        assert_eq!(content, "hello");
        assert_eq!(meta.size, 5);
    }

    #[test]
    fn test_write_file_guarded_happy_path() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "test.md", "first");

        let (_content, meta) =
            read_file_with_meta(RelativePath::new("test.md"), notes_dir.path()).unwrap();

        let new_meta = write_file_guarded(
            RelativePath::new("test.md"),
            notes_dir.path(),
            "second",
            meta,
        )
        .expect("guarded write should succeed when meta matches");

        assert_eq!(new_meta.size, 6);
        let on_disk = std::fs::read_to_string(notes_dir.path().join("test.md")).unwrap();
        assert_eq!(on_disk, "second");
    }

    #[test]
    fn test_write_file_guarded_detects_external_change() {
        let notes_dir = create_test_notes_dir();
        create_test_file(&notes_dir, "test.md", "original");

        let (_content, captured_meta) =
            read_file_with_meta(RelativePath::new("test.md"), notes_dir.path()).unwrap();

        // Simulate external write that bumps mtime/size enough that the
        // captured meta becomes stale. Sleep briefly to guarantee mtime
        // changes on filesystems with second-resolution timestamps.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(notes_dir.path().join("test.md"), "external write").unwrap();

        let result = write_file_guarded(
            RelativePath::new("test.md"),
            notes_dir.path(),
            "our pending edit",
            captured_meta,
        );

        match result {
            Err(GuardedWriteError::ExternalChange { bytes, meta: _ }) => {
                assert_eq!(bytes, b"external write");
            }
            other => panic!("expected ExternalChange, got {other:?}"),
        }

        // Crucially, the file on disk is the external version, not ours.
        let on_disk = std::fs::read_to_string(notes_dir.path().join("test.md")).unwrap();
        assert_eq!(on_disk, "external write");
    }

    #[test]
    fn test_write_file_guarded_missing_file_returns_not_found() {
        let notes_dir = create_test_notes_dir();
        let bogus_meta = OpenFileMeta {
            mtime: SystemTime::UNIX_EPOCH,
            size: 0,
        };
        let result = write_file_guarded(
            RelativePath::new("absent.md"),
            notes_dir.path(),
            "anything",
            bogus_meta,
        );
        assert!(matches!(result, Err(GuardedWriteError::NotFound(_))));
    }
}
