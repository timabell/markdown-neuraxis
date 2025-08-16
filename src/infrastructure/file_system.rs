use crate::domain::models::{FileEntry, NotesStructure};
use crate::domain::services::{FileService, FileServiceError};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct RealFileService;

impl RealFileService {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FileService for RealFileService {
    fn read_file(&self, path: &Path) -> Result<String, FileServiceError> {
        if !path.exists() {
            return Err(FileServiceError::NotFound(path.to_path_buf()));
        }
        fs::read_to_string(path).map_err(FileServiceError::Io)
    }

    fn scan_markdown_files(&self, root: &Path) -> Result<Vec<FileEntry>, FileServiceError> {
        let pages_path = root.join("pages");
        if !pages_path.exists() {
            return Err(FileServiceError::InvalidPath(
                "pages directory not found".to_string(),
            ));
        }

        let mut files = Vec::new();
        scan_directory_recursive(&pages_path, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn validate_notes_structure(&self, root: &Path) -> Result<NotesStructure, FileServiceError> {
        if !root.exists() || !root.is_dir() {
            return Err(FileServiceError::NotFound(root.to_path_buf()));
        }

        let structure = NotesStructure::new(root.to_path_buf());
        if !structure.is_valid() {
            return Err(FileServiceError::InvalidPath(
                "Invalid notes structure: pages directory not found".to_string(),
            ));
        }

        Ok(structure)
    }
}

fn scan_directory_recursive(
    dir: &Path,
    files: &mut Vec<FileEntry>,
) -> Result<(), FileServiceError> {
    let entries = fs::read_dir(dir).map_err(FileServiceError::Io)?;

    for entry in entries {
        let entry = entry.map_err(FileServiceError::Io)?;
        let path = entry.path();

        if path.is_dir() {
            scan_directory_recursive(&path, files)?;
        } else if let Some(ext) = path.extension() {
            if ext == "md" {
                files.push(FileEntry::new(path, false));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub struct MockFileService {
    pub files: std::collections::HashMap<PathBuf, String>,
}

#[cfg(test)]
impl MockFileService {
    pub fn new() -> Self {
        Self {
            files: std::collections::HashMap::new(),
        }
    }

    pub fn add_file<P: Into<PathBuf>>(&mut self, path: P, content: &str) {
        self.files.insert(path.into(), content.to_string());
    }

    pub fn with_files(files: Vec<(PathBuf, String)>) -> Self {
        let mut service = Self::new();
        for (path, content) in files {
            service.files.insert(path, content);
        }
        service
    }
}

#[cfg(test)]
impl FileService for MockFileService {
    fn read_file(&self, path: &Path) -> Result<String, FileServiceError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| FileServiceError::NotFound(path.to_path_buf()))
    }

    fn scan_markdown_files(&self, _root: &Path) -> Result<Vec<FileEntry>, FileServiceError> {
        Ok(self
            .files
            .keys()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext == "md")
            })
            .map(|path| FileEntry::new(path.clone(), false))
            .collect())
    }

    fn file_exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn validate_notes_structure(&self, root: &Path) -> Result<NotesStructure, FileServiceError> {
        Ok(NotesStructure::new(root.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_file_service() {
        let mut mock = MockFileService::new();
        mock.add_file("/test.md", "# Test content");

        let content = mock.read_file(Path::new("/test.md")).unwrap();
        assert_eq!(content, "# Test content");

        let files = mock.scan_markdown_files(Path::new("/")).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("/test.md"));
    }
}
