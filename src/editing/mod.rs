use tree_sitter::{Parser, Tree};
use tree_sitter_md::LANGUAGE;
use xi_rope::delta::Builder;
use xi_rope::{Delta, Rope, RopeInfo};

/// Core document structure that holds the text buffer and provides editing operations.
/// Uses xi-rope for efficient text manipulation and preserves exact byte representation.
pub struct Document {
    /// The rope buffer containing the entire document as UTF-8 bytes
    buffer: Rope,
    /// Current selection/cursor position as byte offsets
    selection: std::ops::Range<usize>,
    /// Version number that increments with each edit
    version: u64,
    /// Tree-sitter parser for incremental parsing
    parser: Parser,
    /// Current parse tree (None until first parse)
    tree: Option<Tree>,
    /// Anchors for stable block IDs that survive edits
    anchors: Vec<Anchor>,
}

impl Document {
    /// Create a new document from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        // Convert bytes to string, ensuring valid UTF-8
        let text = std::str::from_utf8(bytes)?;
        let buffer = Rope::from(text);
        let len = buffer.len();

        // Initialize tree-sitter parser with markdown block grammar
        let mut parser = Parser::new();
        parser.set_language(&LANGUAGE.into())?;

        // Initial parse of the document
        let tree = parser.parse(buffer.to_string(), None);

        Ok(Self {
            buffer,
            selection: len..len, // Start with cursor at end
            version: 0,
            parser,
            tree,
            anchors: Vec::new(),
        })
    }

    /// Get the document's content as raw bytes (exact round-trip)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buffer.to_string().into_bytes()
    }

    /// Apply a command to the document
    pub fn apply(&mut self, cmd: Cmd) -> Patch {
        // Build delta from command
        let delta = self.compile_command(&cmd);

        // Track changed ranges for the patch
        let mut changed = Vec::new();
        let mut cursor = 0;
        for op in delta.els.iter() {
            match op {
                xi_rope::delta::DeltaElement::Copy(_from, to) => {
                    cursor = *to;
                }
                xi_rope::delta::DeltaElement::Insert(inserted) => {
                    let start = cursor;
                    let end = cursor + inserted.len();
                    changed.push(start..end);
                    cursor = end;
                }
            }
        }

        // Apply delta to buffer
        let _old_buffer = self.buffer.clone();
        self.buffer = delta.apply(&self.buffer);

        // Update tree-sitter with incremental parse
        if let Some(ref mut tree) = self.tree {
            // TODO: Implement proper incremental editing with tree.edit()
            // For now, we'll re-parse the entire document
            self.tree = self.parser.parse(self.buffer.to_string(), Some(tree));
        } else {
            self.tree = self.parser.parse(self.buffer.to_string(), None);
        }

        // Transform anchors through the delta
        self.transform_anchors(&delta);

        // Rebind anchors in changed regions after incremental parse
        self.rebind_anchors_in_changed_regions(&changed);

        // Transform selection through command
        let new_selection = self.transform_selection_for_command(&self.selection, &cmd);
        self.selection = new_selection.clone();

        // Increment version
        self.version += 1;

        Patch {
            changed,
            new_selection,
            version: self.version,
        }
    }

    /// Compile a command into a delta
    fn compile_command(&self, cmd: &Cmd) -> Delta<RopeInfo> {
        match cmd {
            Cmd::InsertText { at, text } => {
                let mut builder = Builder::new(self.buffer.len());
                let insert_rope = Rope::from(text);
                builder.replace(*at..*at, insert_rope);
                builder.build()
            }
            Cmd::DeleteRange { range } => {
                let mut builder = Builder::new(self.buffer.len());
                builder.delete(range.clone());
                builder.build()
            }
            Cmd::SplitListItem { at } => {
                // Find the start of the current line
                let line_start = self.find_line_start(*at);
                let line_text = self.get_line_at(line_start);

                // Extract indent and marker from current line
                let (indent, marker) = self.extract_list_info(&line_text);

                // Build the text to insert: newline + indent + marker
                let mut insert_text = String::from("\n");
                if let Some(indent_str) = indent {
                    insert_text.push_str(&indent_str);
                }
                if let Some(marker_str) = marker {
                    insert_text.push_str(&marker_str);
                    insert_text.push(' '); // Space after marker
                }

                // Create insertion delta
                let mut builder = Builder::new(self.buffer.len());
                let insert_rope = Rope::from(insert_text);
                builder.replace(*at..*at, insert_rope);
                builder.build()
            }
            Cmd::IndentLines { range } => {
                let indent_str = "  "; // 2 spaces for indent
                self.modify_line_starts(range, |_line| Some(indent_str.to_string()))
            }
            Cmd::OutdentLines { range } => {
                self.modify_line_starts(range, |line| {
                    // Remove up to 2 spaces from the start
                    if line.starts_with("  ") || line.starts_with(" ") {
                        Some(String::new()) // Will remove leading space(s)
                    } else {
                        None // No change
                    }
                })
            }
            Cmd::ToggleMarker { line_start, to } => {
                let line_text = self.get_line_at(*line_start);
                let new_marker = match to {
                    Marker::Dash => "- ",
                    Marker::Asterisk => "* ",
                    Marker::Plus => "+ ",
                    Marker::Numbered => "1. ",
                };

                // Find existing marker if any
                let trimmed = line_text.trim_start();
                let indent_len = line_text.len() - trimmed.len();

                // Check for existing marker
                let (marker_len, had_marker) = if trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || trimmed.starts_with("+ ")
                {
                    (2, true)
                } else if trimmed.starts_with(char::is_numeric) {
                    // Find numbered marker like "1. " or "10. "
                    if let Some(dot_pos) = trimmed.find(". ") {
                        (dot_pos + 2, true)
                    } else {
                        (0, false)
                    }
                } else {
                    (0, false)
                };

                let mut builder = Builder::new(self.buffer.len());
                let new_marker_rope = Rope::from(new_marker);

                if had_marker {
                    // Replace existing marker
                    let marker_range =
                        (*line_start + indent_len)..(*line_start + indent_len + marker_len);
                    builder.replace(marker_range, new_marker_rope);
                } else {
                    // Add new marker
                    let insert_pos = *line_start + indent_len;
                    builder.replace(insert_pos..insert_pos, new_marker_rope);
                }

                builder.build()
            }
        }
    }

    /// Transform selection based on the command being applied
    fn transform_selection_for_command(
        &self,
        range: &std::ops::Range<usize>,
        cmd: &Cmd,
    ) -> std::ops::Range<usize> {
        match cmd {
            Cmd::InsertText { at, text } => {
                // If insertion point is before or at selection start, shift selection right
                let text_len = text.len();
                if *at <= range.start {
                    (range.start + text_len)..(range.end + text_len)
                } else if *at < range.end {
                    // Insertion is within selection - grow the end
                    range.start..(range.end + text_len)
                } else {
                    // Insertion is after selection - no change
                    range.clone()
                }
            }
            Cmd::DeleteRange { range: del_range } => {
                let del_len = del_range.len();
                if del_range.end <= range.start {
                    // Deletion is completely before selection - shift left
                    (range.start - del_len)..(range.end - del_len)
                } else if del_range.start >= range.end {
                    // Deletion is completely after selection - no change
                    range.clone()
                } else {
                    // Deletion overlaps with selection - collapse to deletion point
                    let collapse_point = del_range.start;
                    collapse_point..collapse_point
                }
            }
            Cmd::SplitListItem { at } => {
                // Similar to insertion logic
                let insert_len = self.calculate_split_insert_length(*at);
                if *at <= range.start {
                    (range.start + insert_len)..(range.end + insert_len)
                } else if *at < range.end {
                    range.start..(range.end + insert_len)
                } else {
                    range.clone()
                }
            }
            Cmd::IndentLines { .. } | Cmd::OutdentLines { .. } | Cmd::ToggleMarker { .. } => {
                // For line-based operations, the selection position might shift
                // but for now, keep it simple and leave unchanged
                range.clone()
            }
        }
    }

    /// Calculate how many characters will be inserted by a split operation
    fn calculate_split_insert_length(&self, at: usize) -> usize {
        let line_start = self.find_line_start(at);
        let line_text = self.get_line_at(line_start);
        let (indent, marker) = self.extract_list_info(&line_text);

        let mut len = 1; // newline
        if let Some(indent_str) = indent {
            len += indent_str.len();
        }
        if let Some(marker_str) = marker {
            len += marker_str.len() + 1; // marker + space
        }
        len
    }

    /// Find the start of the line containing the given offset
    fn find_line_start(&self, offset: usize) -> usize {
        let text = self.buffer.slice_to_cow(..offset);
        if let Some(newline_pos) = text.rfind('\n') {
            newline_pos + 1
        } else {
            0
        }
    }

    /// Get the text of the line starting at the given offset
    fn get_line_at(&self, line_start: usize) -> String {
        let text = self.buffer.slice_to_cow(line_start..);
        if let Some(newline_pos) = text.find('\n') {
            text[..newline_pos].to_string()
        } else {
            text.to_string()
        }
    }

    /// Extract indentation and list marker from a line
    fn extract_list_info(&self, line: &str) -> (Option<String>, Option<String>) {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        let indent = if indent_len > 0 {
            Some(line[..indent_len].to_string())
        } else {
            None
        };

        // Check for list markers
        let marker = if trimmed.starts_with("- ") {
            Some("-".to_string())
        } else if trimmed.starts_with("* ") {
            Some("*".to_string())
        } else if trimmed.starts_with("+ ") {
            Some("+".to_string())
        } else if trimmed.starts_with(char::is_numeric) {
            // Check for numbered list
            trimmed
                .find(". ")
                .map(|dot_pos| trimmed[..dot_pos + 1].to_string())
        } else {
            None
        };

        (indent, marker)
    }

    /// Modify line starts within a range
    fn modify_line_starts(
        &self,
        range: &std::ops::Range<usize>,
        modifier: impl Fn(&str) -> Option<String>,
    ) -> Delta<RopeInfo> {
        let mut builder = Builder::new(self.buffer.len());
        let text = self.buffer.to_string();

        // Split text into lines while tracking positions
        let mut lines = Vec::new();
        let mut line_start = 0;
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                lines.push((line_start, i));
                line_start = i + 1;
            }
        }
        if line_start < text.len() {
            lines.push((line_start, text.len()));
        }

        for (line_start, line_end) in lines {
            let line = &text[line_start..line_end];

            // Check if this line overlaps with the range
            if line_start < range.end && line_end >= range.start {
                if let Some(prefix) = modifier(line) {
                    if prefix.is_empty() {
                        // Removing indentation - delete some characters at line start
                        let skip_len = if line.starts_with("  ") {
                            2
                        } else if line.starts_with(" ") {
                            1
                        } else {
                            0
                        };

                        if skip_len > 0 {
                            builder.delete(line_start..(line_start + skip_len));
                        }
                    } else {
                        // Adding indentation - insert at line start
                        let prefix_rope = Rope::from(prefix);
                        builder.replace(line_start..line_start, prefix_rope);
                    }
                }
            }
        }

        builder.build()
    }

    /// Get a snapshot of the document for rendering (placeholder for now)
    pub fn snapshot(&self) -> Snapshot {
        // TODO: Implement snapshot generation from tree-sitter parse tree and anchors
        // This will walk the CST and generate RenderBlocks for UI consumption
        Snapshot {
            version: self.version,
            blocks: Vec::new(), // Placeholder for now
        }
    }

    /// Transform anchors through a delta operation
    fn transform_anchors(&mut self, delta: &Delta<RopeInfo>) {
        use xi_rope::delta::Transformer;

        // Create a transformer for this delta
        let mut transformer = Transformer::new(delta);

        // Transform each anchor's range through the delta
        for anchor in &mut self.anchors {
            // Transform both start and end positions with different strategies:
            // For the start: use after=true so insertions at the exact start move the anchor forward
            // For the end: use after=false so insertions at the exact end don't expand the anchor
            let new_start = transformer.transform(anchor.range.start, true);
            let new_end = transformer.transform(anchor.range.end, false);

            // Only update if the transformation produces a valid range
            if new_start <= new_end && new_end <= self.buffer.len() {
                anchor.range = new_start..new_end;
            } else {
                // If transformation results in invalid range, clamp to valid bounds
                // This will be refined in rebinding
                let clamped_start = new_start.min(self.buffer.len());
                let clamped_end = new_end.min(self.buffer.len()).max(clamped_start);
                anchor.range = clamped_start..clamped_end;
            }
        }

        // Remove anchors that have become empty or invalid
        self.anchors.retain(|anchor| {
            anchor.range.start < anchor.range.end && anchor.range.end <= self.buffer.len()
        });
    }

    /// Rebind anchors in changed regions to maintain stable block associations
    fn rebind_anchors_in_changed_regions(&mut self, changed: &[std::ops::Range<usize>]) {
        if changed.is_empty() || self.tree.is_none() {
            return;
        }

        // Collect anchors that overlap with changed regions
        let mut anchors_to_rebind = Vec::new();
        for (index, anchor) in self.anchors.iter().enumerate() {
            for changed_range in changed {
                // Check if anchor significantly overlaps with changed region
                // Only rebind if there's substantial overlap, not just touching boundaries
                let overlap = calculate_range_overlap(&anchor.range, changed_range);
                if overlap > 0 && overlap > changed_range.len() / 4 {
                    anchors_to_rebind.push(index);
                    break;
                }
            }
        }

        // For each anchor that needs rebinding, find the best matching node
        // We need to do this separately to avoid borrowing issues
        let ranges_to_update: Vec<(usize, Option<std::ops::Range<usize>>)> = {
            let tree = self.tree.as_ref().unwrap();
            let root_node = tree.root_node();
            anchors_to_rebind
                .iter()
                .map(|&anchor_index| {
                    let new_range = self.find_best_node_for_anchor(root_node, anchor_index);
                    (anchor_index, new_range)
                })
                .collect()
        };

        // Apply the range updates
        for (anchor_index, new_range) in ranges_to_update {
            if let Some(range) = new_range {
                self.anchors[anchor_index].range = range;
            }
        }

        // Remove anchors that couldn't be rebound properly
        self.anchors.retain(|anchor| {
            anchor.range.start < anchor.range.end && anchor.range.end <= self.buffer.len()
        });

        // Create anchors for new block-level nodes in changed regions
        if let Some(ref tree) = self.tree {
            let root_node = tree.root_node();
            let mut new_nodes = Vec::new();
            Self::collect_new_block_nodes_in_regions(root_node, changed, &mut new_nodes);

            for node in new_nodes {
                let node_range = node.byte_range();

                // Check if we already have an anchor for this range
                let has_existing_anchor = self.anchors.iter().any(|anchor| {
                    // Consider ranges that substantially overlap as already covered
                    calculate_range_overlap(&anchor.range, &node_range) > node_range.len() / 2
                });

                if !has_existing_anchor {
                    let anchor_id = self.generate_anchor_id();
                    let anchor = Anchor {
                        id: anchor_id,
                        range: node_range,
                    };
                    self.anchors.push(anchor);
                }
            }
        }
    }

    /// Find the best node to rebind an anchor to
    fn find_best_node_for_anchor(
        &self,
        root_node: tree_sitter::Node,
        anchor_index: usize,
    ) -> Option<std::ops::Range<usize>> {
        let anchor = &self.anchors[anchor_index];
        let mut best_node = None;
        let mut best_overlap = 0;

        // Search for the node that best overlaps with the anchor's current range
        Self::find_best_overlap_recursive(
            root_node,
            &anchor.range,
            &mut best_node,
            &mut best_overlap,
        );

        best_node.map(|node| node.byte_range())
    }

    /// Recursively search for the node with the best overlap with a given range
    fn find_best_overlap_recursive<'a>(
        node: tree_sitter::Node<'a>,
        target_range: &std::ops::Range<usize>,
        best_node: &mut Option<tree_sitter::Node<'a>>,
        best_overlap: &mut usize,
    ) {
        let node_kind = node.kind();
        let is_block_node = matches!(
            node_kind,
            "atx_heading" | "list_item" | "paragraph" | "fenced_code_block" | "indented_code_block"
        );

        if is_block_node {
            let node_range = node.byte_range();
            let overlap = calculate_range_overlap(target_range, &node_range);

            if overlap > *best_overlap {
                *best_overlap = overlap;
                *best_node = Some(node);
            }
        }

        // Search children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::find_best_overlap_recursive(child, target_range, best_node, best_overlap);
        }
    }

    /// Recursively collect new block nodes that appear in changed regions
    fn collect_new_block_nodes_in_regions<'a>(
        node: tree_sitter::Node<'a>,
        changed: &[std::ops::Range<usize>],
        new_nodes: &mut Vec<tree_sitter::Node<'a>>,
    ) {
        let node_kind = node.kind();
        let is_block_node = matches!(
            node_kind,
            "atx_heading" | "list_item" | "paragraph" | "fenced_code_block" | "indented_code_block"
        );

        if is_block_node {
            let node_range = node.byte_range();

            // Check if this node is in a changed region
            for changed_range in changed {
                if node_range.start < changed_range.end && node_range.end > changed_range.start {
                    new_nodes.push(node);
                    break;
                }
            }
        }

        // Process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_new_block_nodes_in_regions(child, changed, new_nodes);
        }
    }

    /// Get the current selection range
    pub fn selection(&self) -> std::ops::Range<usize> {
        self.selection.clone()
    }

    /// Set the selection range
    pub fn set_selection(&mut self, selection: std::ops::Range<usize>) {
        self.selection = selection;
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the current text content
    pub fn text(&self) -> String {
        self.buffer.to_string()
    }

    /// Create anchors from the current tree-sitter parse tree
    pub fn create_anchors_from_tree(&mut self) {
        self.anchors.clear();

        if let Some(ref tree) = self.tree {
            let root_node = tree.root_node();
            let mut new_anchors = Vec::new();
            Self::collect_anchors_recursive(root_node, &mut new_anchors);
            self.anchors = new_anchors;
        }
    }

    /// Recursively collect anchors for block-level nodes in the tree
    fn collect_anchors_recursive(node: tree_sitter::Node, anchors: &mut Vec<Anchor>) {
        // Only create anchors for block-level markdown elements
        let node_kind = node.kind();
        let should_create_anchor = matches!(
            node_kind,
            "atx_heading" | "list_item" | "paragraph" | "fenced_code_block" | "indented_code_block"
        );

        if should_create_anchor && !node.byte_range().is_empty() {
            let anchor_id = Self::generate_static_anchor_id(anchors.len());
            let anchor = Anchor {
                id: anchor_id,
                range: node.byte_range(),
            };
            anchors.push(anchor);
        }

        // Recursively process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_anchors_recursive(child, anchors);
        }
    }

    /// Generate a unique anchor ID
    fn generate_anchor_id(&self) -> AnchorId {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();

        // Include current time to ensure uniqueness
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        timestamp.hash(&mut hasher);

        // Include current anchor count to avoid collisions within same timestamp
        self.anchors.len().hash(&mut hasher);

        // Include current document version
        self.version.hash(&mut hasher);

        AnchorId(hasher.finish() as u128)
    }

    /// Generate a static anchor ID for initial tree creation
    fn generate_static_anchor_id(index: usize) -> AnchorId {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Include a magic number to differentiate from dynamic IDs
        let magic = 0x1234567890abcdefu64;
        magic.hash(&mut hasher);

        // Include index to ensure uniqueness within this generation
        index.hash(&mut hasher);

        AnchorId(hasher.finish() as u128)
    }
}

/// Calculate the overlap between two byte ranges
fn calculate_range_overlap(
    range1: &std::ops::Range<usize>,
    range2: &std::ops::Range<usize>,
) -> usize {
    let start = range1.start.max(range2.start);
    let end = range1.end.min(range2.end);
    end.saturating_sub(start)
}

/// Commands that can be applied to the document
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    InsertText { at: usize, text: String },
    DeleteRange { range: std::ops::Range<usize> },
    SplitListItem { at: usize },
    IndentLines { range: std::ops::Range<usize> },
    OutdentLines { range: std::ops::Range<usize> },
    ToggleMarker { line_start: usize, to: Marker },
}

/// Marker types for list items
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Dash,     // "-"
    Asterisk, // "*"
    Plus,     // "+"
    Numbered, // "1.", "2.", etc.
}

/// Result of applying a command
pub struct Patch {
    pub changed: Vec<std::ops::Range<usize>>,
    pub new_selection: std::ops::Range<usize>,
    pub version: u64,
}

/// Snapshot of the document for rendering
pub struct Snapshot {
    pub version: u64,
    pub blocks: Vec<RenderBlock>,
}

/// A renderable block in the document
pub struct RenderBlock {
    pub id: AnchorId,
    pub kind: BlockKind,
    pub byte_range: std::ops::Range<usize>,
    pub content_range: std::ops::Range<usize>,
    pub depth: usize,
}

/// Stable identifier for a text range that survives edits
#[derive(Clone, Debug)]
pub struct Anchor {
    pub id: AnchorId,
    pub range: std::ops::Range<usize>, // byte range in the rope
                                       // TODO v2: add bias/stickiness and kind hints
}

/// Unique identifier for an anchor
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AnchorId(pub u128);

/// Block types for rendering
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Paragraph,
    Heading { level: u8 },
    ListItem { marker: Marker, depth: usize },
    CodeFence { lang: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Basic document tests ============

    #[test]
    fn test_document_from_bytes_valid_utf8() {
        let text = "# Hello World\n\nThis is a test document.";
        let bytes = text.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should create document from valid UTF-8");

        // The document should be created successfully
        assert_eq!(doc.to_bytes(), bytes);
        assert_eq!(doc.version(), 0);
        assert_eq!(doc.selection(), text.len()..text.len());
    }

    #[test]
    fn test_document_from_bytes_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence

        let result = Document::from_bytes(&invalid_bytes);

        // Should return an error for invalid UTF-8
        assert!(result.is_err());
    }

    // ============ InsertText command tests ============

    #[test]
    fn test_insert_text_at_beginning() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(0..0);

        let patch = doc.apply(Cmd::InsertText {
            at: 0,
            text: "Start: ".to_string(),
        });

        assert_eq!(doc.text(), "Start: Hello World");
        assert_eq!(patch.version, 1);
        assert_eq!(patch.changed, vec![0..7]);
        assert_eq!(patch.new_selection, 7..7);
    }

    #[test]
    fn test_insert_text_in_middle() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(5..5);

        let patch = doc.apply(Cmd::InsertText {
            at: 5,
            text: " Beautiful".to_string(),
        });

        assert_eq!(doc.text(), "Hello Beautiful World");
        assert_eq!(patch.changed, vec![5..15]);
        assert_eq!(patch.new_selection, 15..15);
    }

    #[test]
    fn test_insert_text_at_end() {
        let mut doc = Document::from_bytes(b"Hello").unwrap();
        doc.set_selection(5..5);

        let patch = doc.apply(Cmd::InsertText {
            at: 5,
            text: " World".to_string(),
        });

        assert_eq!(doc.text(), "Hello World");
        assert_eq!(patch.changed, vec![5..11]);
    }

    #[test]
    fn test_insert_text_with_newlines() {
        let mut doc = Document::from_bytes(b"Line 1").unwrap();

        let patch = doc.apply(Cmd::InsertText {
            at: 6,
            text: "\nLine 2\nLine 3".to_string(),
        });

        assert_eq!(doc.text(), "Line 1\nLine 2\nLine 3");
        assert_eq!(patch.changed, vec![6..20]);
    }

    // ============ DeleteRange command tests ============

    #[test]
    fn test_delete_range_single_char() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(5..5);

        let patch = doc.apply(Cmd::DeleteRange { range: 5..6 });

        assert_eq!(doc.text(), "HelloWorld");
        assert_eq!(patch.new_selection, 5..5);
        assert_eq!(patch.version, 1);
    }

    #[test]
    fn test_delete_range_multiple_chars() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(11..11);

        let patch = doc.apply(Cmd::DeleteRange { range: 5..11 });

        assert_eq!(doc.text(), "Hello");
        assert_eq!(patch.new_selection, 5..5);
    }

    #[test]
    fn test_delete_range_across_lines() {
        let mut doc = Document::from_bytes(b"Line 1\nLine 2\nLine 3").unwrap();

        let patch = doc.apply(Cmd::DeleteRange { range: 6..14 });

        assert_eq!(doc.text(), "Line 1Line 3");
    }

    // ============ SplitListItem command tests ============

    #[test]
    fn test_split_list_item_basic() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();
        doc.set_selection(8..8);

        let patch = doc.apply(Cmd::SplitListItem { at: 8 });

        assert_eq!(doc.text(), "- Item 1\n- ");
        assert_eq!(patch.changed, vec![8..11]);
    }

    #[test]
    fn test_split_list_item_with_indent() {
        let mut doc = Document::from_bytes(b"  - Indented item").unwrap();

        let patch = doc.apply(Cmd::SplitListItem { at: 17 });

        assert_eq!(doc.text(), "  - Indented item\n  - ");
        assert_eq!(patch.changed, vec![17..22]);
    }

    #[test]
    fn test_split_list_item_numbered() {
        let mut doc = Document::from_bytes(b"1. First item").unwrap();

        let patch = doc.apply(Cmd::SplitListItem { at: 13 });

        assert_eq!(doc.text(), "1. First item\n1. ");
    }

    #[test]
    fn test_split_list_item_asterisk() {
        let mut doc = Document::from_bytes(b"* Star item").unwrap();

        let patch = doc.apply(Cmd::SplitListItem { at: 11 });

        assert_eq!(doc.text(), "* Star item\n* ");
    }

    #[test]
    fn test_split_list_item_plus() {
        let mut doc = Document::from_bytes(b"+ Plus item").unwrap();

        let patch = doc.apply(Cmd::SplitListItem { at: 11 });

        assert_eq!(doc.text(), "+ Plus item\n+ ");
    }

    #[test]
    fn test_split_list_item_non_list() {
        let mut doc = Document::from_bytes(b"Regular text").unwrap();

        let patch = doc.apply(Cmd::SplitListItem { at: 12 });

        // Should just insert newline for non-list items
        assert_eq!(doc.text(), "Regular text\n");
    }

    // ============ IndentLines command tests ============

    #[test]
    fn test_indent_single_line() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        let patch = doc.apply(Cmd::IndentLines { range: 0..8 });

        assert_eq!(doc.text(), "  - Item 1");
    }

    #[test]
    fn test_indent_multiple_lines() {
        let mut doc = Document::from_bytes(b"- Item 1\n- Item 2\n- Item 3").unwrap();

        let patch = doc.apply(Cmd::IndentLines { range: 0..26 });

        assert_eq!(doc.text(), "  - Item 1\n  - Item 2\n  - Item 3");
    }

    #[test]
    fn test_indent_partial_range() {
        let mut doc = Document::from_bytes(b"- Item 1\n- Item 2\n- Item 3").unwrap();

        // Indent only the middle line
        let patch = doc.apply(Cmd::IndentLines { range: 9..17 });

        assert_eq!(doc.text(), "- Item 1\n  - Item 2\n- Item 3");
    }

    #[test]
    fn test_indent_already_indented() {
        let mut doc = Document::from_bytes(b"  - Already indented").unwrap();

        let patch = doc.apply(Cmd::IndentLines { range: 0..20 });

        assert_eq!(doc.text(), "    - Already indented");
    }

    // ============ OutdentLines command tests ============

    #[test]
    fn test_outdent_single_line() {
        let mut doc = Document::from_bytes(b"  - Item 1").unwrap();

        let patch = doc.apply(Cmd::OutdentLines { range: 0..10 });

        assert_eq!(doc.text(), "- Item 1");
    }

    #[test]
    fn test_outdent_multiple_lines() {
        let mut doc = Document::from_bytes(b"  - Item 1\n  - Item 2\n  - Item 3").unwrap();

        let patch = doc.apply(Cmd::OutdentLines { range: 0..32 });

        assert_eq!(doc.text(), "- Item 1\n- Item 2\n- Item 3");
    }

    #[test]
    fn test_outdent_partial_indent() {
        let mut doc = Document::from_bytes(b" - Item 1").unwrap(); // Single space

        let patch = doc.apply(Cmd::OutdentLines { range: 0..9 });

        assert_eq!(doc.text(), "- Item 1");
    }

    #[test]
    fn test_outdent_no_indent() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        let patch = doc.apply(Cmd::OutdentLines { range: 0..8 });

        // Should not change if there's no indentation
        assert_eq!(doc.text(), "- Item 1");
    }

    #[test]
    fn test_outdent_mixed_indentation() {
        let mut doc = Document::from_bytes(b"- Item 1\n  - Item 2\n    - Item 3").unwrap();

        let patch = doc.apply(Cmd::OutdentLines { range: 0..33 });

        assert_eq!(doc.text(), "- Item 1\n- Item 2\n  - Item 3");
    }

    // ============ ToggleMarker command tests ============

    #[test]
    fn test_toggle_marker_to_dash() {
        let mut doc = Document::from_bytes(b"* Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Dash,
        });

        assert_eq!(doc.text(), "- Item 1");
    }

    #[test]
    fn test_toggle_marker_to_asterisk() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Asterisk,
        });

        assert_eq!(doc.text(), "* Item 1");
    }

    #[test]
    fn test_toggle_marker_to_plus() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Plus,
        });

        assert_eq!(doc.text(), "+ Item 1");
    }

    #[test]
    fn test_toggle_marker_to_numbered() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Numbered,
        });

        assert_eq!(doc.text(), "1. Item 1");
    }

    #[test]
    fn test_toggle_marker_from_numbered() {
        let mut doc = Document::from_bytes(b"1. Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Dash,
        });

        assert_eq!(doc.text(), "- Item 1");
    }

    #[test]
    fn test_toggle_marker_with_indent() {
        let mut doc = Document::from_bytes(b"  - Item 1").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Asterisk,
        });

        assert_eq!(doc.text(), "  * Item 1");
    }

    #[test]
    fn test_toggle_marker_add_to_plain_text() {
        let mut doc = Document::from_bytes(b"Plain text").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Dash,
        });

        assert_eq!(doc.text(), "- Plain text");
    }

    #[test]
    fn test_toggle_marker_with_indent_on_plain() {
        let mut doc = Document::from_bytes(b"  Plain text").unwrap();

        let patch = doc.apply(Cmd::ToggleMarker {
            line_start: 0,
            to: Marker::Dash,
        });

        assert_eq!(doc.text(), "  - Plain text");
    }

    // ============ Selection transformation tests ============

    #[test]
    fn test_selection_transform_after_insert() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(8..10); // "or" selected

        doc.apply(Cmd::InsertText {
            at: 5,
            text: " Beautiful".to_string(),
        });

        // Selection should shift by length of insertion
        assert_eq!(doc.selection(), 18..20);
    }

    #[test]
    fn test_selection_transform_after_delete_before() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(8..10); // "or" selected

        doc.apply(Cmd::DeleteRange { range: 0..6 }); // Delete "Hello "

        // Selection should shift left
        assert_eq!(doc.selection(), 2..4);
    }

    #[test]
    fn test_selection_transform_after_delete_containing() {
        let mut doc = Document::from_bytes(b"Hello World").unwrap();
        doc.set_selection(8..10); // "or" selected

        doc.apply(Cmd::DeleteRange { range: 6..11 }); // Delete "World"

        // Selection should collapse to deletion point
        assert_eq!(doc.selection(), 6..6);
    }

    // ============ Multiple command sequence tests ============

    #[test]
    fn test_multiple_commands_sequence() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();

        // Add a new item
        doc.apply(Cmd::SplitListItem { at: 8 });
        assert_eq!(doc.text(), "- Item 1\n- ");

        // Type text
        doc.apply(Cmd::InsertText {
            at: 11,
            text: "Item 2".to_string(),
        });
        assert_eq!(doc.text(), "- Item 1\n- Item 2");

        // Indent the second item
        doc.apply(Cmd::IndentLines { range: 9..17 });
        assert_eq!(doc.text(), "- Item 1\n  - Item 2");

        // Change marker of second item
        doc.apply(Cmd::ToggleMarker {
            line_start: 9,
            to: Marker::Asterisk,
        });
        assert_eq!(doc.text(), "- Item 1\n  * Item 2");

        // Version should be 4 after 4 commands
        assert_eq!(doc.version(), 4);
    }

    // ============ Anchor system tests ============

    #[test]
    fn test_anchor_creation_from_simple_document() {
        let text = "# Heading\n\n- Item 1\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        // Create anchors from tree-sitter parse tree
        doc.create_anchors_from_tree();

        // Should have anchors for heading and list items
        assert!(
            doc.anchors.len() >= 2,
            "Expected at least 2 anchors for heading and list items"
        );

        // Each anchor should have a unique ID
        let mut ids = std::collections::HashSet::new();
        for anchor in &doc.anchors {
            assert!(ids.insert(anchor.id), "Duplicate anchor ID found");
        }
    }

    #[test]
    fn test_anchor_transformation_insert_before() {
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item 1").unwrap();
        doc.create_anchors_from_tree();

        let original_anchors = doc.anchors.clone();

        // Insert text at the beginning
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "Prefix: ".to_string(),
        });

        // Check that the original anchors still exist and have been transformed correctly
        let insert_len = "Prefix: ".len();
        for original in &original_anchors {
            let current = doc
                .anchors
                .iter()
                .find(|a| a.id == original.id)
                .expect("Original anchor ID should still exist");

            assert_eq!(
                current.range.start,
                original.range.start + insert_len,
                "Anchor start should shift by insert length"
            );
            assert_eq!(
                current.range.end,
                original.range.end + insert_len,
                "Anchor end should shift by insert length"
            );
        }
    }

    #[test]
    fn test_anchor_transformation_insert_after() {
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item 1").unwrap();
        doc.create_anchors_from_tree();

        let original_anchors = doc.anchors.clone();
        let text_len = doc.text().len();

        // Insert text at the end
        doc.apply(Cmd::InsertText {
            at: text_len,
            text: "\n- Item 2".to_string(),
        });

        // Check that the original anchors still exist and have not changed
        for original in &original_anchors {
            let current = doc
                .anchors
                .iter()
                .find(|a| a.id == original.id)
                .expect("Original anchor ID should still exist");
            assert_eq!(
                current.range, original.range,
                "Anchor range should not change for insertion after"
            );
        }
    }

    #[test]
    fn test_anchor_transformation_delete_before() {
        let mut doc = Document::from_bytes(b"Prefix: # Heading\n\n- Item 1").unwrap();
        doc.create_anchors_from_tree();

        let original_anchors = doc.anchors.clone();

        // Delete the prefix
        doc.apply(Cmd::DeleteRange { range: 0..8 });

        // All anchors should shift left by the deletion length
        let delete_len = 8;
        for original in &original_anchors {
            let current = doc
                .anchors
                .iter()
                .find(|a| a.id == original.id)
                .expect("Original anchor ID should still exist");

            // Only anchors that start after the deleted region should be shifted
            if original.range.start >= delete_len {
                assert_eq!(
                    current.range.start,
                    original.range.start - delete_len,
                    "Anchor start should shift left by deletion length"
                );
                assert_eq!(
                    current.range.end,
                    original.range.end - delete_len,
                    "Anchor end should shift left by deletion length"
                );
            }
        }
    }

    #[test]
    fn test_anchor_transformation_delete_overlapping() {
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item 1\n- Item 2").unwrap();
        doc.create_anchors_from_tree();

        let original_anchor_count = doc.anchors.len();

        // Delete part that overlaps with some anchors
        doc.apply(Cmd::DeleteRange { range: 5..15 }); // Delete "ing\n\n- It"

        // Some anchors should be affected - either moved or marked for rebinding
        // The exact behavior depends on the implementation but we should not have invalid ranges
        for anchor in &doc.anchors {
            assert!(
                anchor.range.start <= anchor.range.end,
                "Anchor range should be valid"
            );
            assert!(
                anchor.range.end <= doc.text().len(),
                "Anchor should not extend beyond document"
            );
        }
    }

    #[test]
    fn test_anchor_rebinding_after_parse() {
        let mut doc = Document::from_bytes(b"- Item 1\n- Item 2").unwrap();
        doc.create_anchors_from_tree();

        let original_count = doc.anchors.len();

        // Add a new list item by splitting
        doc.apply(Cmd::SplitListItem { at: 8 });

        // Should have triggered incremental parse and potentially created new anchors
        // The exact count depends on implementation, but should be stable
        assert!(
            doc.anchors.len() >= original_count,
            "Should have at least the original anchors"
        );

        // All anchors should have valid ranges
        for anchor in &doc.anchors {
            assert!(
                anchor.range.start < anchor.range.end,
                "Anchor should have valid range"
            );
            assert!(
                anchor.range.end <= doc.text().len(),
                "Anchor should not extend beyond document"
            );
        }
    }

    #[test]
    fn test_anchor_ids_stable_across_edits() {
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item 1\n- Item 2").unwrap();
        doc.create_anchors_from_tree();

        // Collect original anchor IDs
        let original_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();

        // Perform several edits
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "Prefix: ".to_string(),
        });
        doc.apply(Cmd::SplitListItem { at: 25 }); // Approximate position in "- Item 1"
        doc.apply(Cmd::IndentLines { range: 30..40 }); // Approximate range

        // Original IDs should still exist (though ranges may have changed)
        let current_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();

        for original_id in &original_ids {
            assert!(
                current_ids.contains(original_id),
                "Original anchor ID should still exist after edits"
            );
        }
    }

    #[test]
    fn test_anchor_generation_for_nested_lists() {
        let text = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        doc.create_anchors_from_tree();

        // Should create anchors for all list items
        assert!(
            doc.anchors.len() >= 4,
            "Should have anchors for all list items"
        );

        // Verify ranges don't overlap improperly and are within document bounds
        for (i, anchor) in doc.anchors.iter().enumerate() {
            assert!(
                anchor.range.start < anchor.range.end,
                "Anchor {} should have valid range",
                i
            );
            assert!(
                anchor.range.end <= doc.text().len(),
                "Anchor {} should be within document bounds",
                i
            );
        }
    }

    #[test]
    fn test_empty_document_anchors() {
        let mut doc = Document::from_bytes(b"").unwrap();
        doc.create_anchors_from_tree();

        // Empty document should have no anchors
        assert_eq!(
            doc.anchors.len(),
            0,
            "Empty document should have no anchors"
        );
    }

    #[test]
    fn test_anchor_transform_through_multiple_commands() {
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item 1").unwrap();
        doc.create_anchors_from_tree();

        let original_anchors = doc.anchors.clone();

        // Apply multiple transformations
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "A: ".to_string(),
        });
        doc.apply(Cmd::InsertText {
            at: doc.text().len(),
            text: "\n- Item 2".to_string(),
        });
        doc.apply(Cmd::DeleteRange { range: 0..3 }); // Remove "A: "

        // All anchors should still be valid
        // Note: The count may have increased due to new blocks being created (e.g., "- Item 2")
        assert!(
            doc.anchors.len() >= original_anchors.len(),
            "Should have at least the original number of anchors"
        );
        for anchor in &doc.anchors {
            assert!(anchor.range.start <= anchor.range.end);
            assert!(anchor.range.end <= doc.text().len());
        }

        // The original anchor IDs should still exist
        for original in &original_anchors {
            assert!(
                doc.anchors.iter().any(|a| a.id == original.id),
                "Original anchor ID should still exist"
            );
        }

        // The original heading content should still be findable
        let text = doc.text();
        assert!(text.contains("# Heading"));
        assert!(text.contains("- Item 1"));
    }

    // ============ Round-trip preservation tests ============

    #[test]
    fn test_document_to_bytes_preserves_content() {
        let original =
            "# Markdown Document\n\n- Bullet 1\n- Bullet 2\n\n```rust\nfn main() {}\n```";
        let bytes = original.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should create document");
        let result_bytes = doc.to_bytes();

        // Should preserve exact byte representation
        assert_eq!(result_bytes, bytes);
        assert_eq!(std::str::from_utf8(&result_bytes).unwrap(), original);
    }

    #[test]
    fn test_document_with_unicode() {
        let text = "Hello 世界! 🦀\n\nRust is great! 🎉";
        let bytes = text.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should handle Unicode");
        let result = doc.to_bytes();

        // Unicode content should be preserved exactly
        assert_eq!(result, bytes);
        assert_eq!(std::str::from_utf8(&result).unwrap(), text);
    }

    #[test]
    fn test_document_with_windows_line_endings() {
        let text = "Line 1\r\nLine 2\r\nLine 3";
        let bytes = text.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should handle Windows line endings");
        let result = doc.to_bytes();

        // Windows line endings should be preserved
        assert_eq!(result, bytes);
    }

    #[test]
    fn test_document_with_mixed_line_endings() {
        let text = "Unix line\nWindows line\r\nAnother Unix\n";
        let bytes = text.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should handle mixed line endings");
        let result = doc.to_bytes();

        // Mixed line endings should be preserved exactly
        assert_eq!(result, bytes);
    }
}
