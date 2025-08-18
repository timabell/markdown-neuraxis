use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    BulletList {
        items: Vec<ListItem>,
    },
    NumberedList {
        items: Vec<ListItem>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Quote(String),
    Rule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub content: String,
    pub level: usize,
    pub children: Vec<ListItem>,
    pub nested_content: Vec<ContentBlock>,
}

impl ListItem {
    pub fn new(content: String, level: usize) -> Self {
        Self {
            content,
            level,
            children: Vec::new(),
            nested_content: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub path: PathBuf,
    pub content: Vec<ContentBlock>,
}

impl Document {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            content: Vec::new(),
        }
    }

    pub fn with_content(path: PathBuf, content: Vec<ContentBlock>) -> Self {
        Self { path, content }
    }
}
