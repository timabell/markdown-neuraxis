use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct FileTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_folder: bool,
    pub is_expanded: bool,
    pub children: BTreeMap<String, FileTreeNode>,
}

impl FileTreeNode {
    pub fn new_folder(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            is_folder: true,
            is_expanded: false,
            children: BTreeMap::new(),
        }
    }

    pub fn new_file(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            is_folder: false,
            is_expanded: false,
            children: BTreeMap::new(),
        }
    }

    pub fn insert_file(&mut self, relative_path: &Path, full_path: PathBuf) {
        let components: Vec<_> = relative_path.components().collect();
        if components.is_empty() {
            return;
        }

        let first_component = components[0].as_os_str().to_string_lossy().to_string();

        if components.len() == 1 {
            // This is a file in the current directory
            self.children.insert(
                first_component.clone(),
                FileTreeNode::new_file(first_component, full_path),
            );
        } else {
            // This is a folder, recurse
            let remaining_path = relative_path.iter().skip(1).collect::<PathBuf>();
            let folder_path = self.path.join(&first_component);

            self.children
                .entry(first_component.clone())
                .or_insert_with(|| FileTreeNode::new_folder(first_component, folder_path))
                .insert_file(&remaining_path, full_path);
        }
    }

    pub fn toggle_expanded(&mut self, path: &Path) -> bool {
        if self.path == path {
            self.is_expanded = !self.is_expanded;
            return true;
        }

        for child in self.children.values_mut() {
            if child.toggle_expanded(path) {
                return true;
            }
        }
        false
    }

    pub fn get_flattened_items(&self, depth: usize) -> Vec<FileTreeItem> {
        let mut items = Vec::new();

        // Always include the current node
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
            root: FileTreeNode::new_folder(root_name, root_path),
        }
    }

    pub fn build_from_files(root_path: PathBuf, files: &[PathBuf]) -> Self {
        let mut tree = Self::new(root_path.clone());
        tree.root.is_expanded = true; // Root should always be expanded

        for file in files {
            if let Ok(relative_path) = file.strip_prefix(&root_path) {
                tree.root.insert_file(relative_path, file.clone());
            }
        }

        tree
    }

    pub fn toggle_folder(&mut self, path: &Path) {
        self.root.toggle_expanded(path);
    }

    pub fn get_items(&self) -> Vec<FileTreeItem> {
        self.root.get_flattened_items(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_tree_structure() {
        let root_path = PathBuf::from("/test/pages");
        let files = vec![
            PathBuf::from("/test/pages/inbox.md"),
            PathBuf::from("/test/pages/1_Projects/project1.md"),
            PathBuf::from("/test/pages/1_Projects/project2.md"),
            PathBuf::from("/test/pages/2_Areas/area1.md"),
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

        // Root should be first
        assert_eq!(items[0].node.name, "pages");
        assert!(items[0].node.is_folder);
        assert!(items[0].node.is_expanded);
        assert_eq!(items[0].depth, 0);

        // Check that folders and files are properly nested
        let folder_items: Vec<_> = items
            .iter()
            .filter(|item| item.node.is_folder && item.depth == 1)
            .collect();
        let file_items: Vec<_> = items
            .iter()
            .filter(|item| !item.node.is_folder && item.depth == 1)
            .collect();

        assert_eq!(folder_items.len(), 2); // 1_Projects, 2_Areas
        assert_eq!(file_items.len(), 1); // inbox.md
    }

    #[test]
    fn test_folder_toggle() {
        let root_path = PathBuf::from("/test/pages");
        let files = vec![PathBuf::from("/test/pages/1_Projects/project1.md")];

        let mut tree = FileTree::build_from_files(root_path, &files);
        let projects_path = PathBuf::from("/test/pages/1_Projects");

        // Initially expanded
        assert!(tree.root.is_expanded);

        // Toggle folder
        tree.toggle_folder(&projects_path);
        let projects_node = tree.root.children.get("1_Projects").unwrap();
        assert!(projects_node.is_expanded);

        // Toggle again
        tree.toggle_folder(&projects_path);
        let projects_node = tree.root.children.get("1_Projects").unwrap();
        assert!(!projects_node.is_expanded);
    }

    #[test]
    fn test_sorting_folders_before_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        // Create mixed structure with folders and files
        let folder_a = pages_dir.join("a_folder");
        let folder_z = pages_dir.join("z_folder");
        std::fs::create_dir_all(&folder_a).unwrap();
        std::fs::create_dir_all(&folder_z).unwrap();

        // Create files in folders and at root level
        std::fs::write(folder_a.join("file_in_a.md"), "content").unwrap();
        std::fs::write(folder_z.join("file_in_z.md"), "content").unwrap();
        std::fs::write(pages_dir.join("apple.md"), "content").unwrap();
        std::fs::write(pages_dir.join("zebra.md"), "content").unwrap();

        let files = vec![
            folder_a.join("file_in_a.md"),
            folder_z.join("file_in_z.md"),
            pages_dir.join("apple.md"),
            pages_dir.join("zebra.md"),
        ];
        let tree = FileTree::build_from_files(pages_dir, &files);
        let items = tree.get_items();

        // Should be: pages (root), a_folder, z_folder, apple.md, zebra.md
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].node.name, "pages");
        assert!(items[0].node.is_folder);

        assert_eq!(items[1].node.name, "a_folder");
        assert!(items[1].node.is_folder);

        assert_eq!(items[2].node.name, "z_folder");
        assert!(items[2].node.is_folder);

        assert_eq!(items[3].node.name, "apple.md");
        assert!(!items[3].node.is_folder);

        assert_eq!(items[4].node.name, "zebra.md");
        assert!(!items[4].node.is_folder);
    }

    #[test]
    fn test_case_insensitive_alphabetical_sorting() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        // Create folders with mixed case names and add files to them
        let folders = ["Apple_folder", "banana_folder", "Cherry_folder"];
        let mut file_paths = Vec::new();
        for folder in &folders {
            let folder_path = pages_dir.join(folder);
            std::fs::create_dir_all(&folder_path).unwrap();
            // Add a file to each folder so they appear in the tree
            let file_path = folder_path.join("content.md");
            std::fs::write(&file_path, "content").unwrap();
            file_paths.push(file_path);
        }

        // Create files with mixed case names at root level
        let files_to_create = ["Delta.md", "echo.md", "Foxtrot.md"];
        for file in &files_to_create {
            let path = pages_dir.join(file);
            std::fs::write(&path, "content").unwrap();
            file_paths.push(path);
        }

        let tree = FileTree::build_from_files(pages_dir, &file_paths);
        let items = tree.get_items();

        // Should be sorted case-insensitive: pages, Apple_folder, banana_folder, Cherry_folder, Delta.md, echo.md, Foxtrot.md
        assert_eq!(items.len(), 7);
        assert_eq!(items[0].node.name, "pages");

        // Folders first, sorted case-insensitive
        assert_eq!(items[1].node.name, "Apple_folder");
        assert!(items[1].node.is_folder);
        assert_eq!(items[2].node.name, "banana_folder");
        assert!(items[2].node.is_folder);
        assert_eq!(items[3].node.name, "Cherry_folder");
        assert!(items[3].node.is_folder);

        // Files after folders, sorted case-insensitive
        assert_eq!(items[4].node.name, "Delta.md");
        assert!(!items[4].node.is_folder);
        assert_eq!(items[5].node.name, "echo.md");
        assert!(!items[5].node.is_folder);
        assert_eq!(items[6].node.name, "Foxtrot.md");
        assert!(!items[6].node.is_folder);
    }

    #[test]
    fn test_empty_directory_sorting() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        // Create an empty directory structure
        let empty_files = vec![];
        let tree = FileTree::build_from_files(pages_dir, &empty_files);
        let items = tree.get_items();

        // Should only have the root pages directory
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].node.name, "pages");
        assert!(items[0].node.is_folder);
        assert!(items[0].node.children.is_empty());
    }
}
