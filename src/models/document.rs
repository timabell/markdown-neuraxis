use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentState {
    pub path: PathBuf,
    pub blocks: Vec<(BlockId, ContentBlock)>,
    pub editing_block: Option<(BlockId, String)>, // block_id and raw markdown
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        segments: Vec<TextSegment>,
    },
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

impl ContentBlock {
    pub fn to_markdown(&self) -> String {
        match self {
            ContentBlock::Heading { level, text } => {
                format!("{} {}", "#".repeat(*level as usize), text)
            }
            ContentBlock::Paragraph { segments } => segments_to_markdown(segments),
            ContentBlock::BulletList { items } => items_to_markdown(items, false),
            ContentBlock::NumberedList { items } => items_to_markdown(items, true),
            ContentBlock::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    format!("```{lang}\n{code}\n```")
                } else {
                    format!("```\n{code}\n```")
                }
            }
            ContentBlock::Quote(text) => {
                format!("> {text}")
            }
            ContentBlock::Rule => "---".to_string(),
        }
    }

    pub fn from_markdown(markdown: &str) -> Result<ContentBlock, String> {
        let trimmed = markdown.trim();

        // Try to parse as heading
        if trimmed.starts_with('#') {
            let level_end = trimmed.chars().take_while(|c| *c == '#').count();
            if level_end > 0 && level_end <= 6 {
                let text = trimmed[level_end..].trim().to_string();
                return Ok(ContentBlock::Heading {
                    level: level_end as u8,
                    text,
                });
            }
        }

        // Try to parse as code block
        if trimmed.starts_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.len() >= 2 && lines.last() == Some(&"```") {
                let first_line = lines[0];
                let language = if first_line.len() > 3 {
                    Some(first_line[3..].to_string())
                } else {
                    None
                };
                let code = lines[1..lines.len() - 1].join("\n");
                return Ok(ContentBlock::CodeBlock { language, code });
            }
        }

        // Try to parse as quote
        if let Some(stripped) = trimmed.strip_prefix('>') {
            let text = stripped.trim().to_string();
            return Ok(ContentBlock::Quote(text));
        }

        // Try to parse as rule
        if trimmed == "---" || trimmed == "***" {
            return Ok(ContentBlock::Rule);
        }

        // Try to parse as list
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // For now, create a simple single-item bullet list
            let content = trimmed[2..].trim().to_string();
            let segments = crate::parsing::parse_wiki_links(&content);
            let item = if segments
                .iter()
                .any(|s| matches!(s, TextSegment::WikiLink { .. }))
            {
                ListItem::with_segments(content, segments, 0)
            } else {
                ListItem::new(content, 0)
            };
            return Ok(ContentBlock::BulletList { items: vec![item] });
        }

        // Default to paragraph
        let segments = crate::parsing::parse_wiki_links(trimmed);
        Ok(ContentBlock::Paragraph { segments })
    }
}

fn segments_to_markdown(segments: &[TextSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            TextSegment::Text(text) => text.clone(),
            TextSegment::WikiLink { target } => format!("[[{target}]]"),
        })
        .collect()
}

fn items_to_markdown(items: &[ListItem], is_numbered: bool) -> String {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let marker = if is_numbered {
                format!("{}. ", i + 1)
            } else {
                "- ".to_string()
            };

            let indent = "  ".repeat(item.level);
            let content = if let Some(ref segments) = item.segments {
                segments_to_markdown(segments)
            } else {
                item.content.clone()
            };

            format!("{indent}{marker}{content}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextSegment {
    Text(String),
    WikiLink { target: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub content: String,
    pub segments: Option<Vec<TextSegment>>,
    pub level: usize,
    pub children: Vec<ListItem>,
    pub nested_content: Vec<ContentBlock>,
}

impl ListItem {
    pub fn new(content: String, level: usize) -> Self {
        Self {
            content,
            segments: None,
            level,
            children: Vec::new(),
            nested_content: Vec::new(),
        }
    }

    pub fn with_segments(content: String, segments: Vec<TextSegment>, level: usize) -> Self {
        Self {
            content,
            segments: Some(segments),
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

impl DocumentState {
    pub fn from_document(document: Document) -> Self {
        let blocks = document
            .content
            .into_iter()
            .enumerate()
            .map(|(i, block)| (BlockId(i), block))
            .collect();

        Self {
            path: document.path,
            blocks,
            editing_block: None,
        }
    }

    pub fn to_document(&self) -> Document {
        let content = self.blocks.iter().map(|(_, block)| block.clone()).collect();
        Document {
            path: self.path.clone(),
            content,
        }
    }

    pub fn start_editing(&mut self, block_id: BlockId) {
        if let Some((_, block)) = self.blocks.iter().find(|(id, _)| *id == block_id) {
            let raw_markdown = block.to_markdown();
            self.editing_block = Some((block_id, raw_markdown));
        }
    }

    pub fn finish_editing(&mut self, block_id: BlockId, new_content: String) {
        if let Some(pos) = self.blocks.iter().position(|(id, _)| *id == block_id) {
            if let Ok(new_block) = ContentBlock::from_markdown(&new_content) {
                self.blocks[pos].1 = new_block;
            }
        }
        self.editing_block = None;
    }

    pub fn is_editing(&self, block_id: BlockId) -> Option<&String> {
        if let Some((editing_id, ref content)) = self.editing_block {
            if editing_id == block_id {
                Some(content)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_to_markdown() {
        let heading = ContentBlock::Heading {
            level: 2,
            text: "Test Heading".to_string(),
        };
        assert_eq!(heading.to_markdown(), "## Test Heading");
    }

    #[test]
    fn test_paragraph_to_markdown() {
        let segments = vec![
            TextSegment::Text("This is a ".to_string()),
            TextSegment::WikiLink {
                target: "test-link".to_string(),
            },
            TextSegment::Text(" paragraph.".to_string()),
        ];
        let paragraph = ContentBlock::Paragraph { segments };
        assert_eq!(
            paragraph.to_markdown(),
            "This is a [[test-link]] paragraph."
        );
    }

    #[test]
    fn test_heading_from_markdown() {
        let result = ContentBlock::from_markdown("### Test Heading").unwrap();
        assert_eq!(
            result,
            ContentBlock::Heading {
                level: 3,
                text: "Test Heading".to_string()
            }
        );
    }

    #[test]
    fn test_paragraph_from_markdown() {
        let result = ContentBlock::from_markdown("This is a [[wiki-link]] test.").unwrap();
        if let ContentBlock::Paragraph { segments } = result {
            assert_eq!(segments.len(), 3);
            assert_eq!(segments[0], TextSegment::Text("This is a ".to_string()));
            assert_eq!(
                segments[1],
                TextSegment::WikiLink {
                    target: "wiki-link".to_string(),
                }
            );
            assert_eq!(segments[2], TextSegment::Text(" test.".to_string()));
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_roundtrip_conversion() {
        let original = ContentBlock::Heading {
            level: 1,
            text: "Main Title".to_string(),
        };
        let markdown = original.to_markdown();
        let converted = ContentBlock::from_markdown(&markdown).unwrap();
        assert_eq!(original, converted);
    }
}
