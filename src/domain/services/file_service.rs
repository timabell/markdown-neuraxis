use crate::domain::models::{FileEntry, NotesStructure};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FileServiceError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub trait FileService: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<String, FileServiceError>;
    fn scan_markdown_files(&self, root: &Path) -> Result<Vec<FileEntry>, FileServiceError>;
    fn file_exists(&self, path: &Path) -> bool;
    fn validate_notes_structure(&self, root: &Path) -> Result<NotesStructure, FileServiceError>;
}
