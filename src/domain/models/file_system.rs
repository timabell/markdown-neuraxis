use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub is_directory: bool,
    pub name: String,
}

impl FileEntry {
    pub fn new(path: PathBuf, is_directory: bool) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        Self {
            path,
            is_directory,
            name,
        }
    }

    pub fn is_markdown(&self) -> bool {
        !self.is_directory
            && self
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| ext == "md")
    }
}

#[derive(Debug, Clone)]
pub struct NotesStructure {
    pub root: PathBuf,
    pub pages_dir: PathBuf,
    pub journal_dir: PathBuf,
    pub assets_dir: PathBuf,
}

impl NotesStructure {
    pub fn new(root: PathBuf) -> Self {
        Self {
            pages_dir: root.join("pages"),
            journal_dir: root.join("journal"),
            assets_dir: root.join("assets"),
            root,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.pages_dir.exists()
    }
}
