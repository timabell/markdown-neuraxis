use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// BlockId is an ephemeral unique identifier so that the UI can keep track of blocks in a markdown file as they move around during editing
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
    pub path: RelativePathBuf,
    pub blocks: Vec<(BlockId, ContentBlock)>,
    pub editing_block: Option<(BlockId, String)>, // block_id and raw markdown
    pub selected_block: Option<BlockId>,          // currently selected block for navigation
}

// one piece of a longer markdown document, documents are divided up into blocks such as headings, lists, paragraphs etc to allow them to be selected and edited individually
/// Trait for list-like content blocks
pub trait ListBlock {
    fn items(&self) -> &Vec<(BlockId, ListItem)>;
    fn items_mut(&mut self) -> &mut Vec<(BlockId, ListItem)>;
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
        items: Vec<(BlockId, ListItem)>,
    },
    NumberedList {
        items: Vec<(BlockId, ListItem)>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Quote(String),
    Rule,
}

impl ListBlock for ContentBlock {
    fn items(&self) -> &Vec<(BlockId, ListItem)> {
        match self {
            ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => items,
            _ => panic!("Called ListBlock methods on non-list ContentBlock"),
        }
    }

    fn items_mut(&mut self) -> &mut Vec<(BlockId, ListItem)> {
        match self {
            ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => items,
            _ => panic!("Called ListBlock methods on non-list ContentBlock"),
        }
    }
}

impl ContentBlock {
    pub fn to_markdown(&self) -> String {
        match self {
            ContentBlock::Heading { level, text } => {
                format!("{} {}", "#".repeat(*level as usize), text)
            }
            ContentBlock::Paragraph { segments } => segments_to_markdown(segments),
            ContentBlock::BulletList { items } => {
                let list_items: Vec<ListItem> =
                    items.iter().map(|(_, item)| item.clone()).collect();
                items_to_markdown(&list_items, false)
            }
            ContentBlock::NumberedList { items } => {
                let list_items: Vec<ListItem> =
                    items.iter().map(|(_, item)| item.clone()).collect();
                items_to_markdown(&list_items, true)
            }
            ContentBlock::CodeBlock { language, code } => {
                let separator = if code.ends_with('\n') { "" } else { "\n" };
                if let Some(lang) = language {
                    format!("```{lang}\n{code}{separator}```")
                } else {
                    format!("```\n{code}{separator}```")
                }
            }
            ContentBlock::Quote(text) => {
                format!("> {text}")
            }
            ContentBlock::Rule => "---".to_string(),
        }
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
    fn render_item(item: &ListItem, is_numbered: bool, counter: &mut usize) -> Vec<String> {
        let mut lines = Vec::new();

        let marker = if is_numbered {
            let num = *counter;
            *counter += 1;
            format!("{num}. ")
        } else {
            // Use stored marker if available, otherwise default to dash
            if let Some(ref bullet_marker) = item.marker {
                match bullet_marker {
                    BulletMarker::Numbered => {
                        // If this item was originally numbered, use numbered format
                        let num = *counter;
                        *counter += 1;
                        format!("{num}. ")
                    }
                    _ => format!("{} ", bullet_marker.to_string()),
                }
            } else {
                "- ".to_string()
            }
        };

        let indent = "\t".repeat(item.level);
        let content = if let Some(ref segments) = item.segments {
            segments_to_markdown(segments)
        } else {
            item.content.clone()
        };

        lines.push(format!("{indent}{marker}{content}"));

        // Recursively render children with proper indentation
        if !item.children.is_empty() {
            // Check if children should be rendered as numbered list based on their markers
            let children_items: Vec<ListItem> = item
                .children
                .iter()
                .map(|(_, child)| child.clone())
                .collect();
            let should_be_numbered = children_items
                .iter()
                .any(|child| matches!(child.marker, Some(BulletMarker::Numbered)));
            let child_lines = items_to_markdown(&children_items, should_be_numbered);
            for child_line in child_lines.split('\n') {
                if !child_line.is_empty() {
                    lines.push(child_line.to_string());
                }
            }
        }

        lines
    }

    let mut counter = 1;
    let mut all_lines = Vec::new();

    for item in items {
        all_lines.extend(render_item(item, is_numbered, &mut counter));
    }

    all_lines.join("\n")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BulletMarker {
    Dash,     // -
    Star,     // *
    Numbered, // 1., 2., etc.
}

impl BulletMarker {
    pub fn to_string(&self) -> &'static str {
        match self {
            BulletMarker::Dash => "-",
            BulletMarker::Star => "*",
            BulletMarker::Numbered => "1.", // Will be replaced with actual numbers during rendering
        }
    }
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
    pub children: Vec<(BlockId, ListItem)>,
    pub nested_content: Vec<ContentBlock>,
    pub marker: Option<BulletMarker>, // Store original bullet marker type
}

impl ListItem {
    pub fn new(content: String, level: usize) -> Self {
        Self {
            content,
            segments: None,
            level,
            children: Vec::new(),
            nested_content: Vec::new(),
            marker: None,
        }
    }

    pub fn with_segments(content: String, segments: Vec<TextSegment>, level: usize) -> Self {
        Self {
            content,
            segments: Some(segments),
            level,
            children: Vec::new(),
            nested_content: Vec::new(),
            marker: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub path: RelativePathBuf,
    pub content: Vec<ContentBlock>,
}

impl Document {
    pub fn new(path: RelativePathBuf) -> Self {
        Self {
            path,
            content: Vec::new(),
        }
    }

    pub fn with_content(path: RelativePathBuf, content: Vec<ContentBlock>) -> Self {
        Self { path, content }
    }
}

impl DocumentState {
    /// Helper method to find a nested list item by BlockId
    fn find_nested_item(children: &[(BlockId, ListItem)], target_id: BlockId) -> Option<&ListItem> {
        for (id, item) in children {
            if *id == target_id {
                return Some(item);
            }
            // Recursively search in nested children
            if let Some(found) = Self::find_nested_item(&item.children, target_id) {
                return Some(found);
            }
        }
        None
    }

    /// Helper method to find a mutable nested list item by BlockId
    fn find_nested_item_mut(
        children: &mut [(BlockId, ListItem)],
        target_id: BlockId,
    ) -> Option<&mut ListItem> {
        for (id, item) in children {
            if *id == target_id {
                return Some(item);
            }
            // Recursively search in nested children
            if let Some(found) = Self::find_nested_item_mut(&mut item.children, target_id) {
                return Some(found);
            }
        }
        None
    }
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
            selected_block: None,
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
        // First try to find a block with this ID
        if let Some((_, block)) = self.blocks.iter().find(|(id, _)| *id == block_id) {
            let raw_markdown = block.to_markdown();
            self.editing_block = Some((block_id, raw_markdown));
            return;
        }

        // If not found, try to find a list item with this ID (including nested items)
        for (_, block) in &self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    if let Some((_, list_item)) = items.iter().find(|(id, _)| *id == block_id) {
                        let raw_markdown = list_item.content.clone();
                        self.editing_block = Some((block_id, raw_markdown));
                        return;
                    }
                    // Also search in nested children
                    for (_, item) in items {
                        if let Some(found_item) = Self::find_nested_item(&item.children, block_id) {
                            let raw_markdown = found_item.content.clone();
                            self.editing_block = Some((block_id, raw_markdown));
                            return;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn finish_editing(&mut self, block_id: BlockId, new_content: String) -> Vec<BlockId> {
        // First try to find a block with this ID
        if let Some(pos) = self.blocks.iter().position(|(id, _)| *id == block_id) {
            let new_blocks = crate::parsing::parse_multiple_blocks(&new_content);

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

                // Select the first new block (or the edited block if only one)
                if let Some(first_block_id) = new_block_ids.first() {
                    self.selected_block = Some(*first_block_id);
                }

                return new_block_ids;
            }
        }

        // If not found, try to find and update a list item with this ID (including nested items)
        for (_, block) in &mut self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    if let Some((_, list_item)) = items.iter_mut().find(|(id, _)| *id == block_id) {
                        list_item.content = new_content;
                        self.editing_block = None;
                        return vec![block_id]; // Return the same ID since we updated in place
                    }
                    // Also search in nested children
                    for (_, item) in items {
                        if let Some(found_item) =
                            Self::find_nested_item_mut(&mut item.children, block_id)
                        {
                            found_item.content = new_content;
                            self.editing_block = None;
                            return vec![block_id]; // Return the same ID since we updated in place
                        }
                    }
                }
                _ => {}
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

    /// Select a block for keyboard navigation
    pub fn select_block(&mut self, block_id: BlockId) {
        self.selected_block = Some(block_id);
    }

    /// Get the currently selected block
    pub fn selected_block(&self) -> Option<BlockId> {
        self.selected_block
    }

    /// Move selection to the next block (down)
    pub fn select_next_block(&mut self) {
        if self.blocks.is_empty() {
            return;
        }

        if let Some(current_id) = self.selected_block {
            if let Some(current_pos) = self.blocks.iter().position(|(id, _)| *id == current_id) {
                if current_pos + 1 < self.blocks.len() {
                    self.selected_block = Some(self.blocks[current_pos + 1].0);
                }
            }
        } else {
            // No selection, select first block
            self.selected_block = self.blocks.first().map(|(id, _)| *id);
        }
    }

    /// Move selection to the previous block (up)
    pub fn select_previous_block(&mut self) {
        if self.blocks.is_empty() {
            return;
        }

        if let Some(current_id) = self.selected_block {
            if let Some(current_pos) = self.blocks.iter().position(|(id, _)| *id == current_id) {
                if current_pos > 0 {
                    self.selected_block = Some(self.blocks[current_pos - 1].0);
                }
            }
        } else {
            // No selection, select last block
            self.selected_block = self.blocks.last().map(|(id, _)| *id);
        }
    }

    /// Start editing the currently selected block
    pub fn start_editing_selected(&mut self) -> bool {
        if let Some(selected_id) = self.selected_block {
            self.start_editing(selected_id);
            true
        } else {
            false
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
    fn test_document_state_block_splitting() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
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
    fn test_block_navigation() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![
                ContentBlock::Paragraph {
                    segments: vec![TextSegment::Text("First block".to_string())],
                },
                ContentBlock::Paragraph {
                    segments: vec![TextSegment::Text("Second block".to_string())],
                },
                ContentBlock::Paragraph {
                    segments: vec![TextSegment::Text("Third block".to_string())],
                },
            ],
        );

        let mut doc_state = DocumentState::from_document(document);
        let first_id = doc_state.blocks[0].0;
        let second_id = doc_state.blocks[1].0;
        let third_id = doc_state.blocks[2].0;

        // Initially no selection
        assert_eq!(doc_state.selected_block(), None);

        // Select next block (should select first)
        doc_state.select_next_block();
        assert_eq!(doc_state.selected_block(), Some(first_id));

        // Select next block (should select second)
        doc_state.select_next_block();
        assert_eq!(doc_state.selected_block(), Some(second_id));

        // Select next block (should select third)
        doc_state.select_next_block();
        assert_eq!(doc_state.selected_block(), Some(third_id));

        // Select next block (should stay on third - at end)
        doc_state.select_next_block();
        assert_eq!(doc_state.selected_block(), Some(third_id));

        // Select previous block (should select second)
        doc_state.select_previous_block();
        assert_eq!(doc_state.selected_block(), Some(second_id));

        // Select previous block (should select first)
        doc_state.select_previous_block();
        assert_eq!(doc_state.selected_block(), Some(first_id));

        // Select previous block (should stay on first - at beginning)
        doc_state.select_previous_block();
        assert_eq!(doc_state.selected_block(), Some(first_id));
    }

    #[test]
    fn test_start_editing_selected() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::Paragraph {
                segments: vec![TextSegment::Text("Test content".to_string())],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let block_id = doc_state.blocks[0].0;

        // No selection, should not start editing
        assert!(!doc_state.start_editing_selected());
        assert!(doc_state.editing_block.is_none());

        // Select the block
        doc_state.select_block(block_id);
        assert_eq!(doc_state.selected_block(), Some(block_id));

        // Start editing selected block
        assert!(doc_state.start_editing_selected());
        assert!(doc_state.is_editing(block_id).is_some());
    }
}
