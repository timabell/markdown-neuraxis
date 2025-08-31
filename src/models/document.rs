use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Direct operations on bullets for instant editing
#[derive(Debug, Clone)]
pub enum BulletOperation {
    /// Update bullet content instantly
    UpdateContent(BlockId, String),
    /// Split bullet at cursor position, content after cursor goes to new bullet
    SplitAtCursor(BlockId, String, usize), // content, cursor_position
    /// Indent bullet (increase nesting level)
    Indent(BlockId),
    /// Outdent bullet (decrease nesting level)  
    Outdent(BlockId),
    /// Merge with previous bullet (backspace at start)
    MergeWithPrevious(BlockId),
    /// Delete empty bullet
    DeleteEmpty(BlockId),
}

/// Split string at cursor position (character-aware, not byte-aware)
fn split_string_at_position(text: &str, cursor_pos: usize) -> (String, String) {
    let chars: Vec<char> = text.chars().collect();
    let split_pos = cursor_pos.min(chars.len());
    let before: String = chars[..split_pos].iter().collect();
    let after: String = chars[split_pos..].iter().collect();
    (before, after)
}

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

    /// Execute a bullet operation instantly
    pub fn execute_bullet_operation(&mut self, operation: BulletOperation) -> bool {
        match operation {
            BulletOperation::UpdateContent(block_id, content) => {
                self.update_bullet_content(block_id, content)
            }
            BulletOperation::SplitAtCursor(block_id, content, cursor_pos) => {
                let (before, after) = split_string_at_position(&content, cursor_pos);
                self.split_bullet_at_cursor(block_id, before, after)
                    .is_some()
            }
            BulletOperation::Indent(block_id) => self.indent_bullet(block_id),
            BulletOperation::Outdent(block_id) => self.outdent_bullet(block_id),
            BulletOperation::MergeWithPrevious(block_id) => {
                self.merge_bullet_with_previous(block_id)
            }
            BulletOperation::DeleteEmpty(block_id) => self.delete_empty_bullet(block_id),
        }
    }

    /// Update bullet content instantly
    fn update_bullet_content(&mut self, block_id: BlockId, content: String) -> bool {
        // Clear any editing state since we're doing instant updates
        self.editing_block = None;

        // Try to find and update the bullet
        for (_, block) in &mut self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    if let Some((_, item)) = items.iter_mut().find(|(id, _)| *id == block_id) {
                        item.content = content;
                        item.segments = None; // Clear segments for raw content
                        return true;
                    }
                    // Also search nested bullets
                    if Self::update_nested_bullet_content(items, block_id, &content) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Helper to update nested bullet content
    fn update_nested_bullet_content(
        items: &mut [(BlockId, ListItem)],
        target_id: BlockId,
        content: &str,
    ) -> bool {
        for (_, item) in items {
            // Check direct children
            if let Some((_, child)) = item.children.iter_mut().find(|(id, _)| *id == target_id) {
                child.content = content.to_string();
                child.segments = None;
                return true;
            }
            // Recurse into nested children
            if Self::update_nested_bullet_content(&mut item.children, target_id, content) {
                return true;
            }
        }
        false
    }

    /// Split bullet at cursor position - creates new bullet with content after cursor
    fn split_bullet_at_cursor(
        &mut self,
        block_id: BlockId,
        before_cursor: String,
        after_cursor: String,
    ) -> Option<BlockId> {
        for (_, block) in &mut self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    if let Some(pos) = items.iter().position(|(id, _)| *id == block_id) {
                        let original_level = items[pos].1.level;
                        let original_marker = items[pos].1.marker.clone();

                        // Update original bullet with content before cursor
                        items[pos].1.content = before_cursor;
                        items[pos].1.segments = None;

                        // Create new bullet with content after cursor
                        let new_id = BlockId::new();
                        let new_item = ListItem {
                            content: after_cursor,
                            segments: None,
                            level: original_level,
                            children: vec![],
                            nested_content: vec![],
                            marker: original_marker,
                        };

                        // Insert new bullet right after original
                        items.insert(pos + 1, (new_id, new_item));
                        return Some(new_id);
                    }

                    // Handle nested bullets
                    if let Some(new_id) = Self::split_nested_bullet_at_cursor(
                        items,
                        block_id,
                        &before_cursor,
                        &after_cursor,
                    ) {
                        return Some(new_id);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Helper to split nested bullets
    fn split_nested_bullet_at_cursor(
        items: &mut [(BlockId, ListItem)],
        target_id: BlockId,
        before_cursor: &str,
        after_cursor: &str,
    ) -> Option<BlockId> {
        for (_, item) in items {
            if let Some(pos) = item.children.iter().position(|(id, _)| *id == target_id) {
                let original_level = item.children[pos].1.level;
                let original_marker = item.children[pos].1.marker.clone();

                // Update original
                item.children[pos].1.content = before_cursor.to_string();
                item.children[pos].1.segments = None;

                // Create new bullet
                let new_id = BlockId::new();
                let new_item = ListItem {
                    content: after_cursor.to_string(),
                    segments: None,
                    level: original_level,
                    children: vec![],
                    nested_content: vec![],
                    marker: original_marker,
                };

                item.children.insert(pos + 1, (new_id, new_item));
                return Some(new_id);
            }

            // Recurse
            if let Some(new_id) = Self::split_nested_bullet_at_cursor(
                &mut item.children,
                target_id,
                before_cursor,
                after_cursor,
            ) {
                return Some(new_id);
            }
        }
        None
    }

    /// Indent bullet (move to child of previous bullet)
    fn indent_bullet(&mut self, _block_id: BlockId) -> bool {
        todo!("Implement indent logic")
    }

    /// Outdent bullet (move to parent level)
    fn outdent_bullet(&mut self, _block_id: BlockId) -> bool {
        todo!("Implement outdent logic")
    }

    /// Merge bullet with previous (backspace at start)
    fn merge_bullet_with_previous(&mut self, block_id: BlockId) -> bool {
        // Clear any editing state since we're doing instant updates
        self.editing_block = None;

        // Find and merge the bullet
        for (_, block) in &mut self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    // Check direct items first
                    if let Some(pos) = items.iter().position(|(id, _)| *id == block_id) {
                        if pos > 0 {
                            // Get content from current bullet
                            let current_content = items[pos].1.content.clone();
                            let current_children = items[pos].1.children.clone();

                            // Merge content with previous bullet
                            items[pos - 1].1.content.push_str(&current_content);

                            // Move children to previous bullet
                            items[pos - 1].1.children.extend(current_children);

                            // Remove current bullet
                            items.remove(pos);
                            return true;
                        }
                    }
                    // Also search nested bullets
                    if Self::merge_nested_bullet_with_previous(items, block_id) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Helper to merge nested bullets with previous
    fn merge_nested_bullet_with_previous(
        items: &mut [(BlockId, ListItem)],
        target_id: BlockId,
    ) -> bool {
        for (_, item) in items {
            // Check direct children
            if let Some(pos) = item.children.iter().position(|(id, _)| *id == target_id) {
                if pos > 0 {
                    // Get content from current bullet
                    let current_content = item.children[pos].1.content.clone();
                    let current_children = item.children[pos].1.children.clone();

                    // Merge content with previous bullet
                    item.children[pos - 1].1.content.push_str(&current_content);

                    // Move children to previous bullet
                    item.children[pos - 1].1.children.extend(current_children);

                    // Remove current bullet
                    item.children.remove(pos);
                    return true;
                }
            }
            // Recurse into nested children
            if Self::merge_nested_bullet_with_previous(&mut item.children, target_id) {
                return true;
            }
        }
        false
    }

    /// Delete empty bullet
    fn delete_empty_bullet(&mut self, block_id: BlockId) -> bool {
        // Clear any editing state since we're doing instant updates
        self.editing_block = None;

        // Find and delete the bullet
        for (_, block) in &mut self.blocks {
            match block {
                ContentBlock::BulletList { items } | ContentBlock::NumberedList { items } => {
                    // Check direct items first
                    if let Some(pos) = items.iter().position(|(id, _)| *id == block_id) {
                        items.remove(pos);
                        return true;
                    }
                    // Also search nested bullets
                    if Self::delete_nested_empty_bullet(items, block_id) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Helper to delete nested empty bullets
    fn delete_nested_empty_bullet(
        items: &mut Vec<(BlockId, ListItem)>,
        target_id: BlockId,
    ) -> bool {
        for (_, item) in items {
            // Check direct children
            if let Some(pos) = item.children.iter().position(|(id, _)| *id == target_id) {
                item.children.remove(pos);
                return true;
            }
            // Recurse into nested children
            if Self::delete_nested_empty_bullet(&mut item.children, target_id) {
                return true;
            }
        }
        false
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

    #[test]
    fn test_bullet_operation_update_content() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::BulletList {
                items: vec![(
                    BlockId::new(),
                    ListItem::new("Original content".to_string(), 0),
                )],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let block_id = doc_state.blocks[0].1.items()[0].0;

        // Update bullet content
        let success = doc_state.execute_bullet_operation(BulletOperation::UpdateContent(
            block_id,
            "Updated content".to_string(),
        ));

        assert!(success);
        assert_eq!(
            doc_state.blocks[0].1.items()[0].1.content,
            "Updated content"
        );
    }

    #[test]
    fn test_bullet_operation_split_at_cursor() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::BulletList {
                items: vec![(BlockId::new(), ListItem::new("Hello World".to_string(), 0))],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let original_block_id = doc_state.blocks[0].1.items()[0].0;

        // Split at position 6 (after "Hello ")
        let success = doc_state.execute_bullet_operation(BulletOperation::SplitAtCursor(
            original_block_id,
            "Hello World".to_string(),
            6,
        ));

        assert!(success);
        assert_eq!(doc_state.blocks[0].1.items().len(), 2);
        assert_eq!(doc_state.blocks[0].1.items()[0].1.content, "Hello ");
        assert_eq!(doc_state.blocks[0].1.items()[1].1.content, "World");
    }

    #[test]
    #[ignore = "Operation not implemented yet"]
    fn test_bullet_operation_delete_empty() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::BulletList {
                items: vec![
                    (BlockId::new(), ListItem::new("First bullet".to_string(), 0)),
                    (BlockId::new(), ListItem::new("".to_string(), 0)),
                    (BlockId::new(), ListItem::new("Third bullet".to_string(), 0)),
                ],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let empty_block_id = doc_state.blocks[0].1.items()[1].0;

        // Delete empty bullet
        let success =
            doc_state.execute_bullet_operation(BulletOperation::DeleteEmpty(empty_block_id));

        assert!(success);
        assert_eq!(doc_state.blocks[0].1.items().len(), 2);
        assert_eq!(doc_state.blocks[0].1.items()[0].1.content, "First bullet");
        assert_eq!(doc_state.blocks[0].1.items()[1].1.content, "Third bullet");
    }

    #[test]
    #[ignore = "Operation not implemented yet"]
    fn test_bullet_operation_merge_with_previous() {
        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::BulletList {
                items: vec![
                    (BlockId::new(), ListItem::new("First".to_string(), 0)),
                    (BlockId::new(), ListItem::new(" Second".to_string(), 0)),
                    (BlockId::new(), ListItem::new("Third".to_string(), 0)),
                ],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);
        let second_block_id = doc_state.blocks[0].1.items()[1].0;

        // Merge second bullet with previous
        let success =
            doc_state.execute_bullet_operation(BulletOperation::MergeWithPrevious(second_block_id));

        assert!(success);
        assert_eq!(doc_state.blocks[0].1.items().len(), 2);
        assert_eq!(doc_state.blocks[0].1.items()[0].1.content, "First Second");
        assert_eq!(doc_state.blocks[0].1.items()[1].1.content, "Third");
    }

    #[test]
    #[ignore = "Operation not implemented yet"]
    fn test_bullet_operation_nested_bullets() {
        // Create a nested structure with child bullets
        let mut parent = ListItem::new("Parent bullet".to_string(), 0);
        let child_id = BlockId::new();
        parent
            .children
            .push((child_id, ListItem::new("Child bullet".to_string(), 1)));

        let document = Document::with_content(
            RelativePathBuf::from("test.md"),
            vec![ContentBlock::BulletList {
                items: vec![(BlockId::new(), parent)],
            }],
        );

        let mut doc_state = DocumentState::from_document(document);

        // Update nested bullet content
        let success = doc_state.execute_bullet_operation(BulletOperation::UpdateContent(
            child_id,
            "Updated child".to_string(),
        ));

        assert!(success);
        assert_eq!(
            doc_state.blocks[0].1.items()[0].1.children[0].1.content,
            "Updated child"
        );
    }

    #[test]
    fn test_split_string_at_position() {
        // Test ASCII
        assert_eq!(
            split_string_at_position("hello", 0),
            ("".to_string(), "hello".to_string())
        );
        assert_eq!(
            split_string_at_position("hello", 3),
            ("hel".to_string(), "lo".to_string())
        );
        assert_eq!(
            split_string_at_position("hello", 5),
            ("hello".to_string(), "".to_string())
        );

        // Test boundary - should not panic
        assert_eq!(
            split_string_at_position("hello", 10),
            ("hello".to_string(), "".to_string())
        );

        // Test Unicode emoji
        assert_eq!(
            split_string_at_position("👋 hello", 1),
            ("👋".to_string(), " hello".to_string())
        );
        assert_eq!(
            split_string_at_position("👋 hello", 2),
            ("👋 ".to_string(), "hello".to_string())
        );

        // Test multi-byte characters
        assert_eq!(
            split_string_at_position("café", 3),
            ("caf".to_string(), "é".to_string())
        );
        assert_eq!(
            split_string_at_position("café", 4),
            ("café".to_string(), "".to_string())
        );
    }
}
