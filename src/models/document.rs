use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlineItem {
    pub content: String,
    pub level: usize,
    pub children: Vec<OutlineItem>,
}

impl OutlineItem {
    pub fn new(content: String, level: usize) -> Self {
        Self {
            content,
            level,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub path: PathBuf,
    pub outline: Vec<OutlineItem>,
}

impl Document {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            outline: Vec::new(),
        }
    }

    pub fn with_outline(path: PathBuf, outline: Vec<OutlineItem>) -> Self {
        Self { path, outline }
    }
}
