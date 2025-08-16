use crate::domain::models::{Document, FileEntry, NotesStructure};
use crate::domain::parsing::MarkdownParser;
use crate::domain::services::{FileService, FileServiceError};
use crate::infrastructure::RealFileService;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum DocumentServiceError {
    #[error("File service error: {0}")]
    FileService(#[from] FileServiceError),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Clone)]
pub struct DocumentService {
    file_service: Arc<dyn FileService>,
    parser: Arc<dyn MarkdownParser>,
}

impl PartialEq for DocumentService {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.file_service, &other.file_service)
            && Arc::ptr_eq(&self.parser, &other.parser)
    }
}

impl DocumentService {
    pub fn new(file_service: Arc<dyn FileService>, parser: Arc<dyn MarkdownParser>) -> Self {
        Self {
            file_service,
            parser,
        }
    }

    pub fn load_document(&self, path: &Path) -> Result<Document, DocumentServiceError> {
        let content = self.file_service.read_file(path)?;
        let document = self.parser.parse(&content, path.to_path_buf());
        Ok(document)
    }

    pub fn scan_markdown_files(&self, root: &Path) -> Result<Vec<FileEntry>, DocumentServiceError> {
        self.file_service
            .scan_markdown_files(root)
            .map_err(DocumentServiceError::FileService)
    }

    pub fn validate_notes_structure(
        &self,
        root: &Path,
    ) -> Result<NotesStructure, DocumentServiceError> {
        self.file_service
            .validate_notes_structure(root)
            .map_err(DocumentServiceError::FileService)
    }
}

#[derive(Clone, PartialEq)]
pub struct ApplicationServices {
    pub document_service: DocumentService,
}

impl ApplicationServices {
    pub fn new() -> Self {
        let file_service = Arc::new(RealFileService::new());
        let parser = Arc::new(crate::domain::parsing::PulldownMarkdownParser::new());
        let document_service = DocumentService::new(file_service, parser);

        Self { document_service }
    }

    #[cfg(test)]
    pub fn with_mock_file_service(file_service: Arc<dyn FileService>) -> Self {
        let parser = Arc::new(crate::domain::parsing::PulldownMarkdownParser::new());
        let document_service = DocumentService::new(file_service, parser);

        Self { document_service }
    }
}

impl Default for ApplicationServices {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::MockFileService;

    #[test]
    fn test_document_service_with_mock() {
        let mut mock_fs = MockFileService::new();
        mock_fs.add_file("/test.md", "- Item 1\n- Item 2");

        let services = ApplicationServices::with_mock_file_service(Arc::new(mock_fs));
        let doc = services
            .document_service
            .load_document(Path::new("/test.md"))
            .unwrap();

        assert_eq!(doc.outline.len(), 2);
        assert_eq!(doc.outline[0].content, "Item 1");
        assert_eq!(doc.outline[1].content, "Item 2");
    }

    #[test]
    fn test_scan_files_with_mock() {
        let mut mock_fs = MockFileService::new();
        mock_fs.add_file("/notes/test1.md", "# Test 1");
        mock_fs.add_file("/notes/test2.md", "# Test 2");

        let services = ApplicationServices::with_mock_file_service(Arc::new(mock_fs));
        let files = services
            .document_service
            .scan_markdown_files(Path::new("/notes"))
            .unwrap();

        assert_eq!(files.len(), 2);
    }
}
