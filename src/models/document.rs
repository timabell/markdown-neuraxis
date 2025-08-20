use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub Uuid);

impl BlockId {
    pub fn new() -> Self {
        BlockId(Uuid::new_v4())
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::new()
    }
}

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
            // Parse multiple bullet points separated by newlines
            let lines: Vec<&str> = trimmed.lines().collect();
            let mut items = Vec::new();

            for line in lines {
                let line = line.trim();
                if line.starts_with("- ") || line.starts_with("* ") {
                    let content = line[2..].trim().to_string();
                    let segments = crate::parsing::parse_wiki_links(&content);
                    let item = if segments
                        .iter()
                        .any(|s| matches!(s, TextSegment::WikiLink { .. }))
                    {
                        ListItem::with_segments(content, segments, 0)
                    } else {
                        ListItem::new(content, 0)
                    };
                    items.push(item);
                }
            }

            if !items.is_empty() {
                return Ok(ContentBlock::BulletList { items });
            }
        }

        // Try to parse as numbered list
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains(". ")
        {
            // Parse multiple numbered items separated by newlines
            let lines: Vec<&str> = trimmed.lines().collect();
            let mut items = Vec::new();

            for line in lines {
                let line = line.trim();
                // Check if line starts with number followed by ". "
                if let Some(dot_pos) = line.find(". ") {
                    if line[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                        let content = line[dot_pos + 2..].trim().to_string();
                        let segments = crate::parsing::parse_wiki_links(&content);
                        let item = if segments
                            .iter()
                            .any(|s| matches!(s, TextSegment::WikiLink { .. }))
                        {
                            ListItem::with_segments(content, segments, 0)
                        } else {
                            ListItem::new(content, 0)
                        };
                        items.push(item);
                    }
                }
            }

            if !items.is_empty() {
                return Ok(ContentBlock::NumberedList { items });
            }
        }

        // Default to paragraph
        let segments = crate::parsing::parse_wiki_links(trimmed);
        Ok(ContentBlock::Paragraph { segments })
    }

    /// Parse markdown content that may contain multiple blocks separated by double newlines
    pub fn parse_multiple_blocks(markdown: &str) -> Vec<ContentBlock> {
        if markdown.trim().is_empty() {
            return vec![];
        }

        // Split on double newlines (handles \n\n, \r\n\r\n, etc.)
        let chunks: Vec<&str> = markdown
            .split("\n\n")
            .map(|chunk| chunk.trim())
            .filter(|chunk| !chunk.is_empty())
            .collect();

        if chunks.is_empty() {
            return vec![];
        }

        // If there's only one chunk, use the original single-block parsing
        if chunks.len() == 1 {
            if let Ok(block) = ContentBlock::from_markdown(chunks[0]) {
                return vec![block];
            }
        }

        // Parse each chunk as a separate block
        chunks
            .into_iter()
            .filter_map(|chunk| ContentBlock::from_markdown(chunk).ok())
            .collect()
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
            .map(|block| (BlockId::new(), block))
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

    pub fn finish_editing(&mut self, block_id: BlockId, new_content: String) -> Vec<BlockId> {
        if let Some(pos) = self.blocks.iter().position(|(id, _)| *id == block_id) {
            let new_blocks = ContentBlock::parse_multiple_blocks(&new_content);

            if !new_blocks.is_empty() {
                // Remove the original block
                self.blocks.remove(pos);

                // Insert new blocks at the same position
                let new_block_ids: Vec<BlockId> = new_blocks
                    .into_iter()
                    .enumerate()
                    .map(|(i, block)| {
                        let new_id = BlockId::new();
                        self.blocks.insert(pos + i, (new_id, block));
                        new_id
                    })
                    .collect();

                self.editing_block = None;
                return new_block_ids;
            }
        }

        self.editing_block = None;
        vec![] // Return empty if no blocks were created
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

    /// Insert a new block at the end of the document
    pub fn insert_block_at_end(&mut self, new_block: ContentBlock) -> BlockId {
        let block_id = BlockId::new();
        self.blocks.push((block_id, new_block));
        block_id
    }

    /// Insert a new block at the start of the document  
    pub fn insert_block_at_start(&mut self, new_block: ContentBlock) -> BlockId {
        let block_id = BlockId::new();
        self.blocks.insert(0, (block_id, new_block));
        block_id
    }

    /// Insert a new block after the specified block
    pub fn insert_block_after(
        &mut self,
        after_id: BlockId,
        new_block: ContentBlock,
    ) -> Option<BlockId> {
        if let Some(pos) = self.blocks.iter().position(|(id, _)| *id == after_id) {
            let block_id = BlockId::new();
            self.blocks.insert(pos + 1, (block_id, new_block));
            Some(block_id)
        } else {
            None
        }
    }

    /// Insert a new block before the specified block
    pub fn insert_block_before(
        &mut self,
        before_id: BlockId,
        new_block: ContentBlock,
    ) -> Option<BlockId> {
        if let Some(pos) = self.blocks.iter().position(|(id, _)| *id == before_id) {
            let block_id = BlockId::new();
            self.blocks.insert(pos, (block_id, new_block));
            Some(block_id)
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

    #[test]
    fn test_parse_multiple_blocks_single_paragraph() {
        let markdown = "This is a single paragraph.";
        let blocks = ContentBlock::parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::Paragraph { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_split_paragraphs() {
        let markdown = "First paragraph.\n\nSecond paragraph.";
        let blocks = ContentBlock::parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], ContentBlock::Paragraph { .. }));
        assert!(matches!(blocks[1], ContentBlock::Paragraph { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_mixed_content() {
        let markdown = "# Heading\n\nThis is a paragraph.\n\n- List item";
        let blocks = ContentBlock::parse_multiple_blocks(markdown);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], ContentBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], ContentBlock::Paragraph { .. }));
        assert!(matches!(blocks[2], ContentBlock::BulletList { .. }));
    }

    #[test]
    fn test_parse_multiple_blocks_empty_input() {
        let blocks = ContentBlock::parse_multiple_blocks("");
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_document_state_block_splitting() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Original paragraph".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let block_id = doc_state.blocks[0].0;

        // Edit to split the block
        let new_block_ids =
            doc_state.finish_editing(block_id, "First paragraph\n\nSecond paragraph".to_string());

        // Should have created 2 new blocks
        assert_eq!(new_block_ids.len(), 2);
        assert_eq!(doc_state.blocks.len(), 2);
        assert!(matches!(
            doc_state.blocks[0].1,
            ContentBlock::Paragraph { .. }
        ));
        assert!(matches!(
            doc_state.blocks[1].1,
            ContentBlock::Paragraph { .. }
        ));
    }

    #[test]
    fn test_document_state_insert_operations() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Middle paragraph".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let middle_id = doc_state.blocks[0].0;

        // Insert at start
        let start_id = doc_state.insert_block_at_start(ContentBlock::Paragraph {
            segments: vec![TextSegment::Text("First paragraph".to_string())],
        });

        // Insert at end
        let end_id = doc_state.insert_block_at_end(ContentBlock::Paragraph {
            segments: vec![TextSegment::Text("Last paragraph".to_string())],
        });

        // Should now have 3 blocks in correct order
        assert_eq!(doc_state.blocks.len(), 3);
        assert_eq!(doc_state.blocks[0].0, start_id);
        assert_eq!(doc_state.blocks[1].0, middle_id);
        assert_eq!(doc_state.blocks[2].0, end_id);
    }

    #[test]
    fn test_block_id_uuid_uniqueness() {
        let id1 = BlockId::new();
        let id2 = BlockId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_block_splitting_creates_correct_blocks() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Original paragraph".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let original_block_id = doc_state.blocks[0].0;

        // Start editing
        doc_state.start_editing(original_block_id);

        // Split the block
        let new_block_ids = doc_state.finish_editing(
            original_block_id,
            "First paragraph\n\nSecond paragraph".to_string(),
        );

        // Should have created 2 new blocks
        assert_eq!(new_block_ids.len(), 2);
        assert_eq!(doc_state.blocks.len(), 2);

        // Editing state should be cleared after splitting
        assert!(doc_state.editing_block.is_none());
        assert!(doc_state.is_editing(new_block_ids[0]).is_none());
        assert!(doc_state.is_editing(new_block_ids[1]).is_none());

        // First block should contain only the first part
        let first_block_markdown = doc_state.blocks[0].1.to_markdown();
        assert_eq!(first_block_markdown, "First paragraph");

        // Second block should contain only the second part
        let second_block_markdown = doc_state.blocks[1].1.to_markdown();
        assert_eq!(second_block_markdown, "Second paragraph");
    }

    #[test]
    fn test_bug_reproduction_split_while_editing() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Hello".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let original_block_id = doc_state.blocks[0].0;

        // Start editing "Hello"
        doc_state.start_editing(original_block_id);
        assert_eq!(doc_state.is_editing(original_block_id).unwrap(), "Hello");

        // User types "Hello\n\nWorld" and saves
        let new_block_ids =
            doc_state.finish_editing(original_block_id, "Hello\n\nWorld".to_string());

        // Now we have 2 blocks
        assert_eq!(new_block_ids.len(), 2);

        // BUG CHECK: If user clicks on first block to edit again,
        // what content should be shown in the textarea?
        let first_block_id = new_block_ids[0];
        doc_state.start_editing(first_block_id);

        // This should be "Hello", not "Hello\n\nWorld"
        let editing_content = doc_state.is_editing(first_block_id).unwrap();
        assert_eq!(editing_content, "Hello");
    }

    #[test]
    fn test_no_splitting_clears_edit_state() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Original paragraph".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let original_block_id = doc_state.blocks[0].0;

        // Start editing
        doc_state.start_editing(original_block_id);

        // Edit without splitting (no double newlines)
        let new_block_ids =
            doc_state.finish_editing(original_block_id, "Modified paragraph".to_string());

        // Should have only one block
        assert_eq!(new_block_ids.len(), 1);
        assert_eq!(doc_state.blocks.len(), 1);

        // Should not be in edit mode anymore
        assert!(doc_state.editing_block.is_none());
        assert!(doc_state.is_editing(new_block_ids[0]).is_none());
    }

    #[test]
    fn test_insert_block_at_end_workflow() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Existing paragraph".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        assert_eq!(doc_state.blocks.len(), 1);

        // Simulate adding a new block at end (like clicking the + button)
        let new_block = ContentBlock::Paragraph {
            segments: vec![TextSegment::Text("".to_string())],
        };
        let new_block_id = doc_state.insert_block_at_end(new_block);

        // Should now have 2 blocks
        assert_eq!(doc_state.blocks.len(), 2);

        // Start editing the new empty block
        doc_state.start_editing(new_block_id);
        assert_eq!(doc_state.is_editing(new_block_id).unwrap(), "");

        // User types content in the new block
        let _new_blocks = doc_state.finish_editing(new_block_id, "New content here".to_string());

        // Should still have 2 blocks with correct content
        assert_eq!(doc_state.blocks.len(), 2);
        assert_eq!(doc_state.blocks[0].1.to_markdown(), "Existing paragraph");
        assert_eq!(doc_state.blocks[1].1.to_markdown(), "New content here");
    }

    #[test]
    fn test_numbered_list_parsing_from_editor() {
        // Test parsing numbered list that would happen when user types in the editor
        let markdown = "1. first item\n2. second item\n3. third item";

        let result = ContentBlock::from_markdown(markdown).unwrap();

        if let ContentBlock::NumberedList { items } = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].content, "first item");
            assert_eq!(items[1].content, "second item");
            assert_eq!(items[2].content, "third item");
        } else {
            panic!("Expected numbered list, got: {:?}", result);
        }
    }

    #[test]
    fn test_bullet_list_parsing_from_editor() {
        // Test parsing that would happen when user types a bullet list in the editor
        let markdown = "- bullet one\n- bullet two\n- bullet three";

        // This simulates what happens when ContentBlock::from_markdown is called
        let result = ContentBlock::from_markdown(markdown).unwrap();

        if let ContentBlock::BulletList { items } = result {
            // Debug what we actually get
            println!("Number of items: {}", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("Item {}: '{}'", i, item.content);
            }

            // Should have 3 separate items
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].content, "bullet one");
            assert_eq!(items[1].content, "bullet two");
            assert_eq!(items[2].content, "bullet three");
        } else {
            panic!("Expected bullet list, got: {:?}", result);
        }
    }

    #[test]
    fn test_escape_key_saves_and_exits_editing() {
        use std::path::PathBuf;

        let document = Document::with_content(
            PathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Original content".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let block_id = doc_state.blocks[0].0;

        // Start editing
        doc_state.start_editing(block_id);
        assert!(doc_state.is_editing(block_id).is_some());
        assert_eq!(doc_state.is_editing(block_id).unwrap(), "Original content");

        // Simulate escape key: save and exit (this is what the UI should do)
        let _new_block_ids = doc_state.finish_editing(block_id, "Modified content".to_string());

        // Should no longer be in editing mode
        assert!(doc_state.editing_block.is_none());
        assert!(doc_state.is_editing(block_id).is_none());

        // Content should be saved
        assert_eq!(doc_state.blocks[0].1.to_markdown(), "Modified content");
    }
}
