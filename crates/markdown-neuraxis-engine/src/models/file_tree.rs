use crate::models::MarkdownFile;
use relative_path::{RelativePath, RelativePathBuf};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct FileTreeNode {
    pub name: String,
    pub relative_path: RelativePathBuf,
    pub markdown_file: Option<MarkdownFile>, // Only Some for files, None for folders
    pub is_folder: bool,
    pub is_expanded: bool,
    pub children: BTreeMap<String, FileTreeNode>,
}

impl FileTreeNode {
    pub fn new_folder(name: String, relative_path: RelativePathBuf) -> Self {
        Self {
            name,
            relative_path,
            markdown_file: None,
            is_folder: true,
            is_expanded: false,
            children: BTreeMap::new(),
        }
    }

    pub fn new_file(_name: String, relative_path: RelativePathBuf) -> Self {
        let markdown_file = MarkdownFile::new(relative_path.clone());
        Self {
            name: markdown_file.display_name().to_string(),
            relative_path,
            markdown_file: Some(markdown_file),
            is_folder: false,
            is_expanded: false,
            children: BTreeMap::new(),
        }
    }

    pub fn insert_file(&mut self, relative_path: &Path) {
        let components: Vec<_> = relative_path.components().collect();
        if components.is_empty() {
            return;
        }

        let first_component = components[0].as_os_str().to_string_lossy().to_string();

        if components.len() == 1 {
            // This is a file in the current directory
            let file_relative_path = if self.relative_path.as_str().is_empty() {
                RelativePathBuf::from(&first_component)
            } else {
                self.relative_path.join(&first_component)
            };

            self.children.insert(
                first_component.clone(),
                FileTreeNode::new_file(first_component, file_relative_path),
            );
        } else {
            // This is a folder, recurse
            let remaining_path = relative_path.iter().skip(1).collect::<PathBuf>();
            let folder_relative_path = if self.relative_path.as_str().is_empty() {
                RelativePathBuf::from(&first_component)
            } else {
                self.relative_path.join(&first_component)
            };

            self.children
                .entry(first_component.clone())
                .or_insert_with(|| FileTreeNode::new_folder(first_component, folder_relative_path))
                .insert_file(&remaining_path);
        }
    }

    /// Remove a file from the tree, returns true if the node itself should be removed
    pub fn remove_file(&mut self, relative_path: &Path) -> bool {
        let components: Vec<_> = relative_path.components().collect();
        if components.is_empty() {
            return false;
        }

        let first_component = components[0].as_os_str().to_string_lossy().to_string();

        if components.len() == 1 {
            // This is the file to remove
            self.children.remove(&first_component);
        } else {
            // Recurse into subfolder
            let remaining_path = relative_path.iter().skip(1).collect::<PathBuf>();
            if let Some(child) = self.children.get_mut(&first_component) {
                child.remove_file(&remaining_path);
                // Remove empty folders
                if child.is_folder && child.children.is_empty() {
                    self.children.remove(&first_component);
                }
            }
        }

        // Return true if this node should be removed (is folder and now empty)
        self.is_folder && self.children.is_empty()
    }

    pub fn toggle_expanded(&mut self, relative_path: &RelativePath) -> bool {
        if self.relative_path == relative_path {
            self.is_expanded = !self.is_expanded;
            return true;
        }

        for child in self.children.values_mut() {
            if child.toggle_expanded(relative_path) {
                return true;
            }
        }
        false
    }

    pub fn expand(&mut self, relative_path: &RelativePath) -> bool {
        if self.relative_path == relative_path {
            if self.is_folder && !self.is_expanded {
                self.is_expanded = true;
                return true;
            }
            return false;
        }

        for child in self.children.values_mut() {
            if child.expand(relative_path) {
                return true;
            }
        }
        false
    }

    pub fn collapse(&mut self, relative_path: &RelativePath) -> bool {
        if self.relative_path == relative_path {
            if self.is_folder && self.is_expanded {
                self.is_expanded = false;
                return true;
            }
            return false;
        }

        for child in self.children.values_mut() {
            if child.collapse(relative_path) {
                return true;
            }
        }
        false
    }

    /// Find a folder by target name or path (case-insensitive).
    /// Matches against full relative path or final folder name.
    pub fn find_folder_recursive(&self, target: &str) -> Option<RelativePathBuf> {
        // Normalize: trim whitespace and trailing slashes
        let target_normalized = target.trim().trim_end_matches('/');
        let target_lower = target_normalized.to_lowercase();

        for child in self.children.values() {
            if child.is_folder {
                // Check if full path matches (case-insensitive)
                if child.relative_path.as_str().to_lowercase() == target_lower {
                    return Some(child.relative_path.clone());
                }

                // Check if final folder name matches (case-insensitive)
                let folder_name_lower = child.name.to_lowercase();
                if folder_name_lower == target_lower {
                    return Some(child.relative_path.clone());
                }

                // Recurse into child folders
                if let Some(found) = child.find_folder_recursive(target) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn get_flattened_items(&self, depth: usize) -> Vec<FileTreeItem> {
        let mut items = Vec::new();

        // Include the current node
        items.push(FileTreeItem {
            node: self.clone(),
            depth,
        });

        // Include children if expanded
        if self.is_expanded {
            // Sort children: folders first, then files, both case-insensitive alphabetically
            let mut sorted_children: Vec<_> = self.children.values().collect();
            sorted_children.sort_by(|a, b| {
                // Folders come before files
                match (a.is_folder, b.is_folder) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        // Both are folders or both are files, sort alphabetically case-insensitive
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                }
            });

            for child in sorted_children {
                items.extend(child.get_flattened_items(depth + 1));
            }
        }

        items
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileTreeItem {
    pub node: FileTreeNode,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileTree {
    pub root: FileTreeNode,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Self {
        let root_name = root_path
            .file_name()
            .unwrap_or_else(|| root_path.as_os_str())
            .to_string_lossy()
            .to_string();

        Self {
            root: FileTreeNode::new_folder(root_name, RelativePathBuf::new()),
        }
    }

    pub fn build_from_files(root_path: PathBuf, files: &[PathBuf]) -> Self {
        let root_name = root_path
            .file_name()
            .unwrap_or_else(|| root_path.as_os_str())
            .to_string_lossy()
            .to_string();

        let mut root = FileTreeNode::new_folder(root_name, RelativePathBuf::new());
        root.is_expanded = true;

        for file in files {
            if let Ok(relative_path) = file.strip_prefix(&root_path) {
                root.insert_file(relative_path);
            }
        }

        Self { root }
    }

    pub fn toggle_folder(&mut self, relative_path: &RelativePath) {
        self.root.toggle_expanded(relative_path);
    }

    pub fn expand_folder(&mut self, relative_path: &RelativePath) {
        self.root.expand(relative_path);
    }

    pub fn collapse_folder(&mut self, relative_path: &RelativePath) {
        self.root.collapse(relative_path);
    }

    /// Find a folder by target name or path (case-insensitive).
    /// Matches against:
    /// 1. Full relative path (e.g., "1_Projects/active")
    /// 2. Final folder name only (e.g., "Projects" matches "1_Projects")
    pub fn find_folder(&self, target: &str) -> Option<RelativePathBuf> {
        self.root.find_folder_recursive(target)
    }

    /// Expand all folders along a path (all ancestors and the target folder).
    pub fn expand_to_folder(&mut self, relative_path: &RelativePath) {
        // Expand each ancestor folder from root down to the target
        let mut current_path = RelativePathBuf::new();
        for component in relative_path.iter() {
            current_path.push(component);
            self.root.expand(&current_path);
        }
    }

    /// Add a new file to the tree
    pub fn add_file(&mut self, file_path: &Path, notes_root: &Path) {
        if let Ok(relative_path) = file_path.strip_prefix(notes_root) {
            self.root.insert_file(relative_path);
        }
    }

    /// Remove a file from the tree, cleaning up empty parent folders
    pub fn remove_file(&mut self, file_path: &Path, notes_root: &Path) {
        if let Ok(relative_path) = file_path.strip_prefix(notes_root) {
            self.root.remove_file(relative_path);
        }
    }

    pub fn get_items(&self) -> Vec<FileTreeItem> {
        // Return only children of root, not the root itself
        let mut items = Vec::new();

        // Sort children: folders first, then files, both case-insensitive alphabetically
        let mut sorted_children: Vec<_> = self.root.children.values().collect();
        sorted_children.sort_by(|a, b| {
            // Folders come before files
            match (a.is_folder, b.is_folder) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Both are folders or both are files, sort alphabetically case-insensitive
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            }
        });

        for child in sorted_children {
            items.extend(child.get_flattened_items(0));
        }

        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_tree_structure() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![
            PathBuf::from("/test/notes/inbox.md"),
            PathBuf::from("/test/notes/1_Projects/project1.md"),
            PathBuf::from("/test/notes/1_Projects/project2.md"),
            PathBuf::from("/test/notes/2_Areas/area1.md"),
        ];

        let tree = FileTree::build_from_files(root_path, &files);
        let items = tree.get_items();

        // Should have inbox.md, 1_Projects, and 2_Areas as children
        assert_eq!(tree.root.children.len(), 3);
        assert!(tree.root.children.contains_key("inbox.md"));
        assert!(tree.root.children.contains_key("1_Projects"));
        assert!(tree.root.children.contains_key("2_Areas"));

        // Check that items are generated correctly
        assert!(!items.is_empty());

        // Check that folders and files are properly nested at root level (depth 0)
        let folder_items: Vec<_> = items
            .iter()
            .filter(|item| item.node.is_folder && item.depth == 0)
            .collect();
        let file_items: Vec<_> = items
            .iter()
            .filter(|item| !item.node.is_folder && item.depth == 0)
            .collect();

        assert_eq!(folder_items.len(), 2); // 1_Projects, 2_Areas
        assert_eq!(file_items.len(), 1); // inbox.md
    }

    #[test]
    fn test_folder_toggle() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/1_Projects/project1.md")];

        let mut tree = FileTree::build_from_files(root_path, &files);

        // Initially expanded
        assert!(tree.root.is_expanded);

        // Toggle folder
        let relative_projects_path = RelativePathBuf::from("1_Projects");
        tree.toggle_folder(&relative_projects_path);
        let projects_node = tree.root.children.get("1_Projects").unwrap();
        assert!(projects_node.is_expanded);

        // Toggle again
        tree.toggle_folder(&relative_projects_path);
        let projects_node = tree.root.children.get("1_Projects").unwrap();
        assert!(!projects_node.is_expanded);
    }

    #[test]
    fn test_sorting_folders_before_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let notes_dir = temp_dir.path().to_path_buf();

        // Create mixed structure with folders and files
        let folder_a = notes_dir.join("a_folder");
        let folder_z = notes_dir.join("z_folder");
        std::fs::create_dir_all(&folder_a).unwrap();
        std::fs::create_dir_all(&folder_z).unwrap();

        // Create files in folders and at root level
        std::fs::write(folder_a.join("file_in_a.md"), "content").unwrap();
        std::fs::write(folder_z.join("file_in_z.md"), "content").unwrap();
        std::fs::write(notes_dir.join("apple.md"), "content").unwrap();
        std::fs::write(notes_dir.join("zebra.md"), "content").unwrap();

        let files = vec![
            folder_a.join("file_in_a.md"),
            folder_z.join("file_in_z.md"),
            notes_dir.join("apple.md"),
            notes_dir.join("zebra.md"),
        ];
        let tree = FileTree::build_from_files(notes_dir.clone(), &files);
        let items = tree.get_items();

        // Should be: a_folder, z_folder, apple.md, zebra.md (no root folder)
        assert_eq!(items.len(), 4);

        assert_eq!(items[0].node.name, "a_folder");
        assert!(items[0].node.is_folder);

        assert_eq!(items[1].node.name, "z_folder");
        assert!(items[1].node.is_folder);

        assert_eq!(items[2].node.name, "apple");
        assert!(!items[2].node.is_folder);

        assert_eq!(items[3].node.name, "zebra");
        assert!(!items[3].node.is_folder);
    }

    #[test]
    fn test_case_insensitive_alphabetical_sorting() {
        let temp_dir = tempfile::tempdir().unwrap();
        let notes_dir = temp_dir.path().to_path_buf();

        // Create folders with mixed case names and add files to them
        let folders = ["Apple_folder", "banana_folder", "Cherry_folder"];
        let mut file_paths = Vec::new();
        for folder in &folders {
            let folder_path = notes_dir.join(folder);
            std::fs::create_dir_all(&folder_path).unwrap();
            // Add a file to each folder so they appear in the tree
            let file_path = folder_path.join("content.md");
            std::fs::write(&file_path, "content").unwrap();
            file_paths.push(file_path);
        }

        // Create files with mixed case names at root level
        let files_to_create = ["Delta.md", "echo.md", "Foxtrot.md"];
        for file in &files_to_create {
            let path = notes_dir.join(file);
            std::fs::write(&path, "content").unwrap();
            file_paths.push(path);
        }

        let tree = FileTree::build_from_files(notes_dir.clone(), &file_paths);
        let items = tree.get_items();

        // Should be sorted case-insensitive: Apple_folder, banana_folder, Cherry_folder, Delta.md, echo.md, Foxtrot.md (no root)
        assert_eq!(items.len(), 6);

        // Folders first, sorted case-insensitive
        assert_eq!(items[0].node.name, "Apple_folder");
        assert!(items[0].node.is_folder);
        assert_eq!(items[1].node.name, "banana_folder");
        assert!(items[1].node.is_folder);
        assert_eq!(items[2].node.name, "Cherry_folder");
        assert!(items[2].node.is_folder);

        // Files after folders, sorted case-insensitive
        assert_eq!(items[3].node.name, "Delta");
        assert!(!items[3].node.is_folder);
        assert_eq!(items[4].node.name, "echo");
        assert!(!items[4].node.is_folder);
        assert_eq!(items[5].node.name, "Foxtrot");
        assert!(!items[5].node.is_folder);
    }

    #[test]
    fn test_empty_directory_sorting() {
        let temp_dir = tempfile::tempdir().unwrap();
        let notes_dir = temp_dir.path().to_path_buf();

        // Create an empty directory structure
        let empty_files = vec![];
        let tree = FileTree::build_from_files(notes_dir.clone(), &empty_files);
        let items = tree.get_items();

        // Should have no items (empty directory, no root shown)
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_find_folder_by_full_path() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![
            PathBuf::from("/test/notes/1_Projects/active/task.md"),
            PathBuf::from("/test/notes/2_Areas/work.md"),
        ];

        let tree = FileTree::build_from_files(root_path, &files);

        // Find by full path
        let result = tree.find_folder("1_Projects/active");
        assert_eq!(result, Some(RelativePathBuf::from("1_Projects/active")));

        // Find top-level folder
        let result = tree.find_folder("2_Areas");
        assert_eq!(result, Some(RelativePathBuf::from("2_Areas")));
    }

    #[test]
    fn test_find_folder_by_name_case_insensitive() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/Projects/task.md")];

        let tree = FileTree::build_from_files(root_path, &files);

        // Case insensitive match on folder name
        let result = tree.find_folder("projects");
        assert_eq!(result, Some(RelativePathBuf::from("Projects")));

        let result = tree.find_folder("PROJECTS");
        assert_eq!(result, Some(RelativePathBuf::from("Projects")));
    }

    #[test]
    fn test_find_folder_returns_none_for_files() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/inbox.md")];

        let tree = FileTree::build_from_files(root_path, &files);

        // Should not find files
        let result = tree.find_folder("inbox");
        assert_eq!(result, None);

        let result = tree.find_folder("inbox.md");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_folder_returns_none_for_nonexistent() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/1_Projects/task.md")];

        let tree = FileTree::build_from_files(root_path, &files);

        let result = tree.find_folder("nonexistent");
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_folder_with_trailing_slash() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/Projects/task.md")];

        let tree = FileTree::build_from_files(root_path, &files);

        // Should match with trailing slash
        let result = tree.find_folder("Projects/");
        assert_eq!(result, Some(RelativePathBuf::from("Projects")));
    }

    #[test]
    fn test_expand_to_folder() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![PathBuf::from("/test/notes/1_Projects/active/deep/task.md")];

        let mut tree = FileTree::build_from_files(root_path, &files);

        // Initially only root is expanded
        let projects = tree.root.children.get("1_Projects").unwrap();
        assert!(!projects.is_expanded);

        // Expand to deep folder
        tree.expand_to_folder(&RelativePathBuf::from("1_Projects/active/deep"));

        // All folders along the path should be expanded
        let projects = tree.root.children.get("1_Projects").unwrap();
        assert!(projects.is_expanded);

        let active = projects.children.get("active").unwrap();
        assert!(active.is_expanded);

        let deep = active.children.get("deep").unwrap();
        assert!(deep.is_expanded);
    }

    #[test]
    fn test_remove_file() {
        let root_path = PathBuf::from("/test/notes");
        let files = vec![
            PathBuf::from("/test/notes/file1.md"),
            PathBuf::from("/test/notes/file2.md"),
            PathBuf::from("/test/notes/folder/nested.md"),
        ];
        let mut tree = FileTree::build_from_files(root_path.clone(), &files);

        // Should have 3 items initially (folder + 2 root files, nested is inside)
        let items = tree.get_items();
        assert_eq!(items.len(), 3);

        // Remove a root file
        tree.remove_file(&PathBuf::from("/test/notes/file1.md"), &root_path);
        let items = tree.get_items();
        assert_eq!(items.len(), 2);
        assert!(!items.iter().any(|i| i.node.name == "file1"));

        // Remove nested file
        tree.remove_file(&PathBuf::from("/test/notes/folder/nested.md"), &root_path);
        let items = tree.get_items();
        // Folder should be removed too since it's now empty
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].node.name, "file2");
    }
}
