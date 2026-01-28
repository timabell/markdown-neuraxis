use std::collections::BTreeMap;
use std::path::PathBuf;

/// Represents a discovered markdown file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    pub display_name: String,
}

/// In-memory model of discovered files with incremental update support.
///
/// Uses BTreeMap for automatic sorted ordering by path.
#[derive(Debug, Default)]
pub struct FileModel {
    files: BTreeMap<PathBuf, FileEntry>,
}

impl FileModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single file to the model
    pub fn add_file(&mut self, relative_path: PathBuf, display_name: String) {
        self.files.insert(
            relative_path.clone(),
            FileEntry {
                relative_path,
                display_name,
            },
        );
    }

    /// Add multiple files in a batch
    pub fn add_files(&mut self, entries: impl IntoIterator<Item = (PathBuf, String)>) {
        for (path, name) in entries {
            self.add_file(path, name);
        }
    }

    /// Get all files, sorted by path
    pub fn files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.values()
    }

    /// Get the number of files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the model is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Remove a single file from the model
    ///
    /// Returns the removed entry if it existed
    pub fn remove_file(&mut self, relative_path: &PathBuf) -> Option<FileEntry> {
        self.files.remove(relative_path)
    }

    /// Remove all files from the model
    pub fn clear(&mut self) {
        self.files.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_model_is_empty() {
        let model = FileModel::new();
        assert!(model.is_empty());
        assert_eq!(model.len(), 0);
    }

    #[test]
    fn test_add_single_file() {
        let mut model = FileModel::new();
        model.add_file(PathBuf::from("notes/hello.md"), "hello.md".to_string());

        assert_eq!(model.len(), 1);
        assert!(!model.is_empty());

        let files: Vec<_> = model.files().collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, PathBuf::from("notes/hello.md"));
        assert_eq!(files[0].display_name, "hello.md");
    }

    #[test]
    fn test_add_batch() {
        let mut model = FileModel::new();
        model.add_files([
            (PathBuf::from("a.md"), "a.md".to_string()),
            (PathBuf::from("b.md"), "b.md".to_string()),
            (PathBuf::from("c.md"), "c.md".to_string()),
        ]);

        assert_eq!(model.len(), 3);
    }

    #[test]
    fn test_files_sorted_by_path() {
        let mut model = FileModel::new();
        // Add files in non-sorted order
        model.add_file(PathBuf::from("z/last.md"), "last.md".to_string());
        model.add_file(PathBuf::from("a/first.md"), "first.md".to_string());
        model.add_file(PathBuf::from("m/middle.md"), "middle.md".to_string());

        let paths: Vec<_> = model.files().map(|f| &f.relative_path).collect();
        assert_eq!(
            paths,
            vec![
                &PathBuf::from("a/first.md"),
                &PathBuf::from("m/middle.md"),
                &PathBuf::from("z/last.md"),
            ]
        );
    }

    #[test]
    fn test_duplicate_path_overwrites() {
        let mut model = FileModel::new();
        model.add_file(PathBuf::from("test.md"), "original".to_string());
        model.add_file(PathBuf::from("test.md"), "updated".to_string());

        assert_eq!(model.len(), 1);
        let files: Vec<_> = model.files().collect();
        assert_eq!(files[0].display_name, "updated");
    }

    #[test]
    fn test_remove_file() {
        let mut model = FileModel::new();
        model.add_files([
            (PathBuf::from("a.md"), "a.md".to_string()),
            (PathBuf::from("b.md"), "b.md".to_string()),
        ]);

        assert_eq!(model.len(), 2);

        // Remove existing file
        let removed = model.remove_file(&PathBuf::from("a.md"));
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().display_name, "a.md");
        assert_eq!(model.len(), 1);

        // Remove non-existent file
        let removed = model.remove_file(&PathBuf::from("nonexistent.md"));
        assert!(removed.is_none());
        assert_eq!(model.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut model = FileModel::new();
        model.add_files([
            (PathBuf::from("a.md"), "a.md".to_string()),
            (PathBuf::from("b.md"), "b.md".to_string()),
        ]);

        assert_eq!(model.len(), 2);
        model.clear();
        assert!(model.is_empty());
        assert_eq!(model.len(), 0);
    }
}
