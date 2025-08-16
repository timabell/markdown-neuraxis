use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlineItem {
    pub content: String,
    pub level: usize,
    pub children: Vec<OutlineItem>,
    pub metadata: HashMap<String, String>,
}

impl OutlineItem {
    pub fn new(content: String, level: usize) -> Self {
        Self {
            content,
            level,
            children: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_children(content: String, level: usize, children: Vec<OutlineItem>) -> Self {
        Self {
            content,
            level,
            children,
            metadata: HashMap::new(),
        }
    }

    pub fn add_child(&mut self, child: OutlineItem) {
        self.children.push(child);
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub path: PathBuf,
    pub outline: Vec<OutlineItem>,
    pub frontmatter: HashMap<String, String>,
}

impl Document {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            outline: Vec::new(),
            frontmatter: HashMap::new(),
        }
    }

    pub fn with_outline(path: PathBuf, outline: Vec<OutlineItem>) -> Self {
        Self {
            path,
            outline,
            frontmatter: HashMap::new(),
        }
    }

    pub fn add_outline_item(&mut self, item: OutlineItem) {
        self.outline.push(item);
    }
}
