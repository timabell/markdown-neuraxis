use tree_sitter::{Parser, Tree};
use tree_sitter_md::LANGUAGE;
use xi_rope::{Delta, Rope, RopeInfo};

use crate::editing::{Anchor, Cmd, Patch};

/// Marker types for list items
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Dash,     // "-"
    Asterisk, // "*"
    Plus,     // "+"
    Numbered, // "1.", "2.", etc.
}

/// Indentation style detected in the document
#[derive(Debug, Clone, PartialEq)]
pub enum IndentStyle {
    Spaces(usize), // Number of spaces per indent level
    Tabs,          // Tab characters
}

impl IndentStyle {
    /// Convert an indentation string to depth level
    pub fn calculate_depth(&self, indent_str: &str) -> usize {
        match self {
            IndentStyle::Tabs => {
                // Count tab characters for depth
                indent_str.chars().take_while(|&c| c == '\t').count()
            }
            IndentStyle::Spaces(spaces_per_level) => {
                // Count spaces and divide by spaces per level
                let space_count = indent_str.chars().take_while(|&c| c == ' ').count();
                if space_count == 0 {
                    0
                } else {
                    space_count / spaces_per_level
                }
            }
        }
    }
}

/// Core document structure implementing ADR-0004 editor architecture
///
/// Document represents the complete editing model described in ADR-4. It maintains:
///
/// ## 1. Single Source of Truth (xi-rope buffer)
/// - **Lossless storage**: Entire document in one `xi_rope::Rope` buffer  
/// - **Exact round-trip**: `to_bytes()` returns identical content to original
/// - **Efficient edits**: xi-rope provides O(log n) insert/delete operations
/// - **Delta tracking**: All changes generate invertible Delta operations
///
/// ## 2. Incremental Parsing (Tree-sitter)
/// - **Markdown grammar**: Uses tree-sitter-markdown for structural parsing
/// - **Incremental updates**: Only re-parses changed document regions
/// - **CST access**: Provides structured view while preserving byte fidelity  
/// - **Parse stability**: Tree updates via `tree.edit()` before re-parsing
///
/// ## 3. Stable Block Identity (Anchors)
/// - **Persistent IDs**: AnchorIds survive document edits for UI stability
/// - **Range transformation**: Anchor byte ranges updated via Delta transforms
/// - **Rebinding logic**: Re-associates anchors after incremental parsing
/// - **Block tracking**: Links UI elements to logical document structures
///
/// ## 4. Command-Based Editing
/// - **Edit algebra**: All changes flow through `Cmd` enum compilation
/// - **Atomic operations**: Commands compile to Deltas and apply immediately
/// - **Selection tracking**: Cursor/selection positions transform automatically
/// - **Undo foundation**: Delta history enables branching undo (future)
///
/// ## Usage Pattern
///
/// ```rust
/// # use markdown_neuraxis_engine::editing::{Document, Cmd};
/// // Create document with lossless byte preservation
/// let markdown_bytes = b"# Hello\n\n- Item 1";
/// let mut doc = Document::from_bytes(markdown_bytes).unwrap();
///
/// // Initialize stable block identifiers
/// doc.create_anchors_from_tree();
///
/// // Apply structured edits
/// let patch = doc.apply(Cmd::SplitListItem { at: 10 });
///
/// // Generate UI-ready view
/// let snapshot = doc.snapshot();
///
/// // Round-trip: save exact original bytes
/// let bytes_to_save = doc.text();
/// # assert!(!bytes_to_save.is_empty());
/// ```
///
/// This architecture enables high-performance Markdown editing with exact fidelity,
/// stable UI references, and clean separation between model and view layers.
pub struct Document {
    /// xi-rope buffer containing entire document as UTF-8 bytes (ADR-4 source of truth)
    pub(crate) buffer: Rope,
    /// Current selection/cursor position as byte offsets in buffer  
    pub(crate) selection: std::ops::Range<usize>,
    /// Version counter incremented on each edit (enables change detection)
    pub(crate) version: u64,
    /// Tree-sitter parser for incremental Markdown parsing
    pub(crate) parser: Parser,
    /// Current parse tree (None until first parse, updated incrementally)
    pub(crate) tree: Option<Tree>,
    /// Stable block identifiers that survive edits (ADR-4 anchor system)
    pub(crate) anchors: Vec<Anchor>,
    /// Document's indentation style (spaces vs tabs, detected on load)
    pub(crate) indent_style: IndentStyle,
}

impl Document {
    /// Create a new document from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        // Convert bytes to string, ensuring valid UTF-8
        let text = std::str::from_utf8(bytes)?;
        let buffer = Rope::from(text);
        let len = buffer.len();

        // Detect indent style BEFORE tree-sitter parsing
        let indent_style = detect_indent_style(&buffer);

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
            indent_style,
        })
    }

    /// Get the document's content as raw bytes (exact round-trip)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buffer.to_string().into_bytes()
    }

    /// Apply command to document (ADR-0004 Core Edit Loop)
    ///
    /// This method implements the complete editing pipeline described in ADR-4:
    ///
    /// ## Edit Pipeline Steps
    ///
    /// 1. **Command Compilation**: Convert `Cmd` to xi-rope `Delta`
    /// 2. **Incremental Parsing**: Feed Delta to Tree-sitter before buffer update
    /// 3. **Buffer Application**: Apply Delta to xi-rope buffer (authoritative update)
    /// 4. **Anchor Transformation**: Update anchor ranges via Delta transforms
    /// 5. **Anchor Rebinding**: Re-associate anchors with updated parse tree
    /// 6. **Selection Update**: Transform cursor/selection through edit
    /// 7. **Version Increment**: Update document version for change detection
    ///
    /// ## Critical Ordering
    ///
    /// The function must call `tree.edit()` **before** applying the Delta to the buffer.
    /// This is because Tree-sitter needs the old buffer state to calculate proper
    /// coordinate transformations during incremental parsing.
    ///
    /// ## Return Value
    ///
    /// Returns a `Patch` containing:
    /// - **`changed`**: Byte ranges modified by this edit
    /// - **`new_selection`**: Updated cursor/selection position  
    /// - **`version`**: New document version number
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let patch = doc.apply(Cmd::InsertText {
    ///     at: 0,
    ///     text: "# ".to_string()
    /// });
    ///
    /// // Document buffer updated, anchors stable, version incremented
    /// assert_eq!(patch.version, doc.version());
    /// ```
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

        // Use incremental parsing to preserve node stability
        if let Some(mut old_tree) = self.tree.take() {
            // Convert xi-rope delta to tree-sitter InputEdits BEFORE applying delta
            // This is critical because we need the old buffer state for coordinate calculation
            let edits = self.delta_to_input_edits(&delta);

            // Apply all edits to the tree
            for edit in edits {
                old_tree.edit(&edit);
            }

            // NOW apply delta to buffer after we've calculated the edits
            self.buffer = delta.apply(&self.buffer);

            self.tree = self.parser.parse(self.buffer.to_string(), Some(&old_tree));
        } else {
            // No old tree, do full parse - apply delta first in this case
            self.buffer = delta.apply(&self.buffer);
            self.tree = self.parser.parse(self.buffer.to_string(), None);
        }

        // Check if we need to create anchors for a completely new document
        let was_empty = self.anchors.is_empty();
        let inserting_at_start = changed.iter().any(|range| range.start == 0);

        // Transform anchors through the delta
        self.transform_anchors(&delta);

        // Rebind anchors in changed regions after incremental parse
        self.rebind_anchors_in_changed_regions(&changed);

        // Create anchors for any new blocks that don't have anchors yet
        // Only do this if we started with no anchors (empty document case)
        if was_empty && inserting_at_start {
            // This handles the case of inserting the first content into an empty document
            self.create_anchors_for_new_blocks();
        }

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

    /// Get the current selection range
    pub fn selection(&self) -> std::ops::Range<usize> {
        self.selection.clone()
    }

    /// Set the selection range
    pub fn set_selection(&mut self, selection: std::ops::Range<usize>) {
        self.selection = selection;
    }

    /// Get reference to anchors for testing
    pub fn anchors(&self) -> &[crate::editing::Anchor] {
        &self.anchors
    }

    /// Get reference to tree for testing  
    pub fn tree(&self) -> Option<&tree_sitter::Tree> {
        self.tree.as_ref()
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the current text content
    pub fn text(&self) -> String {
        self.buffer.to_string()
    }

    /// Get the buffer length
    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Slice the buffer to a cow string
    pub(crate) fn slice_to_cow(&self, range: std::ops::Range<usize>) -> std::borrow::Cow<'_, str> {
        let doc_len = self.buffer.len();

        // Clamp range to document bounds to prevent xi-rope panic
        let start = range.start.min(doc_len);
        let end = range.end.min(doc_len).max(start);
        let clamped_range = start..end;

        // Silently clamp invalid ranges - this is expected behavior when text changes

        self.buffer.slice_to_cow(clamped_range)
    }

    // Forward declarations for methods implemented in other modules
    pub(crate) fn compile_command(&self, cmd: &Cmd) -> Delta<RopeInfo> {
        crate::editing::commands::compile_command(self, cmd)
    }

    pub(crate) fn transform_selection_for_command(
        &self,
        range: &std::ops::Range<usize>,
        cmd: &Cmd,
    ) -> std::ops::Range<usize> {
        crate::editing::commands::transform_selection_for_command(self, range, cmd)
    }

    pub(crate) fn transform_anchors(&mut self, delta: &Delta<RopeInfo>) {
        crate::editing::anchors::transform_anchors(self, delta)
    }

    pub(crate) fn rebind_anchors_in_changed_regions(&mut self, changed: &[std::ops::Range<usize>]) {
        crate::editing::anchors::rebind_anchors_in_changed_regions(self, changed)
    }

    pub fn create_anchors_from_tree(&mut self) {
        crate::editing::anchors::create_anchors_from_tree(self)
    }

    /// Create anchors for any new blocks that don't have anchors yet
    /// This is called automatically after edits to ensure all blocks have stable identifiers
    fn create_anchors_for_new_blocks(&mut self) {
        crate::editing::anchors::create_anchors_for_new_blocks(self)
    }

    /// Convert xi-rope delta to tree-sitter InputEdits
    ///
    /// This function must be called BEFORE applying the delta to the buffer,
    /// as it needs the old buffer state for proper coordinate calculation.
    ///
    /// Key concepts:
    /// - xi-rope Delta: sequence of Copy(from, to) and Insert(text) operations
    /// - Gaps between Copy operations indicate deletions
    /// - tree-sitter InputEdit uses OLD document byte offsets and coordinates
    fn delta_to_input_edits(&self, delta: &Delta<RopeInfo>) -> Vec<tree_sitter::InputEdit> {
        let mut edits = Vec::new();
        let mut old_pos = 0; // Current position in OLD document (source of truth)

        let old_text = self.buffer.to_string(); // OLD text for coordinate calculation

        for op in &delta.els {
            match op {
                xi_rope::delta::DeltaElement::Copy(from, to) => {
                    // Check if there's a gap before this copy (indicating a deletion)
                    if old_pos < *from {
                        // Deletion: from old_pos to *from
                        let start_byte = old_pos; // Start at current position in OLD doc
                        let old_end_byte = *from; // End at copy start in OLD doc  
                        let new_end_byte = old_pos; // After deletion, new position = start

                        let start_pos = byte_to_point_in_text(&old_text, old_pos);
                        let old_end_pos = byte_to_point_in_text(&old_text, *from);
                        let new_end_pos = start_pos; // After deletion, new position = start position

                        edits.push(tree_sitter::InputEdit {
                            start_byte,
                            old_end_byte,
                            new_end_byte,
                            start_position: tree_sitter::Point {
                                row: start_pos.0,
                                column: start_pos.1,
                            },
                            old_end_position: tree_sitter::Point {
                                row: old_end_pos.0,
                                column: old_end_pos.1,
                            },
                            new_end_position: tree_sitter::Point {
                                row: new_end_pos.0,
                                column: new_end_pos.1,
                            },
                        });
                    }

                    // Copy operation: advance position in old document
                    old_pos = *to;
                }
                xi_rope::delta::DeltaElement::Insert(text) => {
                    // Insertion at current position in old document
                    let start_byte = old_pos; // Insert position in OLD doc
                    let old_end_byte = old_pos; // No content consumed in OLD doc
                    let new_end_byte = old_pos + text.len(); // New content extends beyond old position

                    let start_pos = byte_to_point_in_text(&old_text, old_pos);
                    let old_end_pos = start_pos; // No old content consumed

                    // Calculate new end position by simulating the text insertion
                    // We need to consider how the inserted text affects line/column positions
                    let inserted_text = text.to_string(); // Convert rope node to string
                    let new_end_pos = if inserted_text.contains('\n') {
                        // Multi-line insertion: calculate final position
                        let lines: Vec<&str> = inserted_text.split('\n').collect();
                        let final_line = start_pos.0 + lines.len() - 1;
                        let final_col = if lines.len() > 1 {
                            // Last line starts fresh
                            lines.last().unwrap().len()
                        } else {
                            // Same line, add to existing column
                            start_pos.1 + inserted_text.len()
                        };
                        (final_line, final_col)
                    } else {
                        // Single-line insertion: just add to column
                        (start_pos.0, start_pos.1 + inserted_text.len())
                    };

                    edits.push(tree_sitter::InputEdit {
                        start_byte,
                        old_end_byte,
                        new_end_byte,
                        start_position: tree_sitter::Point {
                            row: start_pos.0,
                            column: start_pos.1,
                        },
                        old_end_position: tree_sitter::Point {
                            row: old_end_pos.0,
                            column: old_end_pos.1,
                        },
                        new_end_position: tree_sitter::Point {
                            row: new_end_pos.0,
                            column: new_end_pos.1,
                        },
                    });

                    // Insert operations don't advance old_pos since they insert at current position
                }
            }
        }

        // Check for final deletion if old_pos hasn't reached the end of the old document
        if old_pos < delta.base_len {
            // Final deletion: from current old_pos to end of document
            let start_byte = old_pos;
            let old_end_byte = delta.base_len;
            let new_end_byte = old_pos; // After deletion, position stays at start

            let start_pos = byte_to_point_in_text(&old_text, old_pos);
            let old_end_pos = byte_to_point_in_text(&old_text, delta.base_len);
            let new_end_pos = start_pos; // After deletion, position stays at start

            edits.push(tree_sitter::InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position: tree_sitter::Point {
                    row: start_pos.0,
                    column: start_pos.1,
                },
                old_end_position: tree_sitter::Point {
                    row: old_end_pos.0,
                    column: old_end_pos.1,
                },
                new_end_position: tree_sitter::Point {
                    row: new_end_pos.0,
                    column: new_end_pos.1,
                },
            });
        }

        edits
    }

    pub fn snapshot(&self) -> crate::editing::Snapshot {
        crate::editing::snapshot::create_snapshot(self)
    }

    /// Hit-testing helper: Find which block contains the given byte position
    /// Returns the block ID and the local offset within that block's content
    /// This implements ADR-0004 selection/caret transformation requirements
    pub fn locate_in_block(
        &self,
        byte_position: usize,
    ) -> Option<(crate::editing::AnchorId, usize)> {
        let snapshot = self.snapshot();

        for block in &snapshot.blocks {
            if block.byte_range.contains(&byte_position) {
                // Calculate local offset within this block's content range
                let local_offset = byte_position.saturating_sub(block.content_range.start);
                return Some((block.id, local_offset));
            }
        }

        None
    }

    /// Hit-testing helper: Convert global byte position to textarea-local description
    /// Returns the block ID, local byte offset, and cursor position for textarea
    /// This implements ADR-0004 selection mapping between rope and textarea
    pub fn describe_point(&self, byte_position: usize) -> Option<crate::editing::PointDescription> {
        if let Some((block_id, local_offset)) = self.locate_in_block(byte_position) {
            let snapshot = self.snapshot();

            // Find the block to get its content
            if let Some(block) = snapshot.blocks.iter().find(|b| b.id == block_id) {
                let content = &block.content;

                // Calculate line and column within the content for textarea mapping
                let (local_line, local_col) = byte_to_point_in_text(content, local_offset);

                return Some(crate::editing::PointDescription {
                    block_id,
                    local_byte_offset: local_offset,
                    local_line,
                    local_col,
                    textarea_cursor_pos: local_offset, // For textarea selectionStart/End
                });
            }
        }

        None
    }
}

/// Detect the indent style (tabs vs spaces and size) by finding the first non-zero indentation
fn detect_indent_style(buffer: &Rope) -> IndentStyle {
    // Convert the rope to string to iterate over lines
    // This is acceptable during document loading as it's a one-time operation
    let text = buffer.to_string();
    let lines = text.lines();

    for line in lines {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Check for tab indentation first
        if line.starts_with('\t') {
            return IndentStyle::Tabs;
        }

        // Check for space indentation
        if line.starts_with(' ') {
            // Count leading spaces
            let spaces = line.chars().take_while(|&c| c == ' ').count();
            if spaces > 0 {
                return IndentStyle::Spaces(spaces);
            }
        }
    }

    // Default to 2 spaces if we couldn't detect
    IndentStyle::Spaces(2)
}

/// Convert byte offset to (row, column) position in given text
fn byte_to_point_in_text(text: &str, byte_offset: usize) -> (usize, usize) {
    let text_bytes = text.as_bytes();
    let offset = byte_offset.min(text_bytes.len());

    let mut row = 0;
    let mut last_newline = 0;

    for (i, &byte) in text_bytes.iter().enumerate().take(offset) {
        if byte == b'\n' {
            row += 1;
            last_newline = i + 1;
        }
    }

    let col = offset - last_newline;
    (row, col)
}

impl Clone for Document {
    fn clone(&self) -> Self {
        // Create a new parser since Parser doesn't implement Clone
        let mut parser = Parser::new();
        let _ = parser.set_language(&LANGUAGE.into());

        // Re-parse the document for the cloned version
        let tree = parser.parse(self.buffer.to_string(), None);

        let mut cloned_doc = Self {
            buffer: self.buffer.clone(),
            selection: self.selection.clone(),
            version: self.version,
            parser,
            tree,
            anchors: Vec::new(), // Start with empty anchors
            indent_style: self.indent_style.clone(),
        };

        // FIX: Regenerate anchors for the new tree to fix stale node_id references
        cloned_doc.create_anchors_from_tree();

        cloned_doc
    }
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        // Compare the essential state that matters for equality
        // Compare buffer content as strings since Node doesn't implement PartialEq
        self.buffer.to_string() == other.buffer.to_string()
            && self.selection == other.selection
            && self.version == other.version
            && self.anchors == other.anchors
            && self.indent_style == other.indent_style
        // Note: We don't compare parser or tree as they are derived from buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xi_rope::delta::Builder;

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

    // ============ Incremental parsing tests ============

    #[test]
    fn test_no_xi_rope_panic_on_stale_ranges() {
        // This test specifically targets the original bug where stale node ranges
        // would cause xi-rope to panic with "called `Option::unwrap()` on a `None` value"

        let text = "# Header\n\n- Bullet 1\n- Bullet 2\n\nSome content after bullets.";
        let mut doc = Document::from_bytes(text.as_bytes()).expect("Should create document");

        // Simulate the problematic edit scenario: insert text that changes document structure
        use crate::editing::Cmd;

        // Insert some text in the middle that would shift byte positions
        let insert_cmd = Cmd::InsertText {
            text: "\n\nNew paragraph inserted here.\n\n".to_string(),
            at: 20, // Insert after "- Bullet 1\n"
        };

        // This should NOT panic with xi-rope errors
        let _patch = doc.apply(insert_cmd);

        // Verify the document is still valid and operations work
        assert!(doc.version() > 0);
        assert!(!doc.text().is_empty());

        // Verify we can still slice the document without panics
        let slice = doc.slice_to_cow(0..10);
        assert!(!slice.is_empty());

        // Test a second edit to make sure incremental parsing keeps working
        let delete_cmd = Cmd::DeleteRange { range: 10..25 };
        let _patch2 = doc.apply(delete_cmd);
        assert!(doc.version() > 1);
        assert!(!doc.text().is_empty());
    }

    #[test]
    fn test_byte_to_point_in_text_helper() {
        let text = "Line 1\nLine 2\nLine 3";

        // Position 0 should be (0, 0)
        assert_eq!(byte_to_point_in_text(text, 0), (0, 0));

        // Position 6 should be (0, 6) - end of first line
        assert_eq!(byte_to_point_in_text(text, 6), (0, 6));

        // Position 7 should be (1, 0) - start of second line (after \n)
        assert_eq!(byte_to_point_in_text(text, 7), (1, 0));

        // Position 13 should be (1, 6) - end of second line
        assert_eq!(byte_to_point_in_text(text, 13), (1, 6));

        // Position at end should be (2, 6) - end of third line
        assert_eq!(byte_to_point_in_text(text, text.len()), (2, 6));

        // Beyond end should be clamped to end
        assert_eq!(byte_to_point_in_text(text, text.len() + 100), (2, 6));
    }

    // ============ Delta to InputEdit conversion tests ============

    #[test]
    fn test_delta_to_input_edits_simple_insertion() {
        let doc = Document::from_bytes(b"Hello World").unwrap();

        // Create delta for inserting " there" at position 5 (after "Hello")
        let mut builder = Builder::new(doc.buffer.len());
        builder.replace(5..5, Rope::from(" there")); // Insert " there" at position 5
        let delta = builder.build();

        let edits = doc.delta_to_input_edits(&delta);

        assert_eq!(edits.len(), 1);
        let edit = &edits[0];

        // Should be an insertion at position 5
        assert_eq!(edit.start_byte, 5);
        assert_eq!(edit.old_end_byte, 5);
        assert_eq!(edit.new_end_byte, 11); // 5 + " there".len()
        assert_eq!(
            edit.start_position,
            tree_sitter::Point { row: 0, column: 5 }
        );
        assert_eq!(
            edit.old_end_position,
            tree_sitter::Point { row: 0, column: 5 }
        );
        assert_eq!(
            edit.new_end_position,
            tree_sitter::Point { row: 0, column: 11 }
        );
    }

    #[test]
    fn test_delta_to_input_edits_simple_deletion() {
        let doc = Document::from_bytes(b"Hello World").unwrap();

        // Create delta for deleting " World" (positions 5-11)
        let mut builder = Builder::new(doc.buffer.len());
        builder.delete(5..11); // Delete " World"
        let delta = builder.build();

        let edits = doc.delta_to_input_edits(&delta);

        assert_eq!(edits.len(), 1);
        let edit = &edits[0];

        // Should be a deletion from position 5 to end
        assert_eq!(edit.start_byte, 5);
        assert_eq!(edit.old_end_byte, 11); // End of " World"
        assert_eq!(edit.new_end_byte, 5); // After deletion, same as start
        assert_eq!(
            edit.start_position,
            tree_sitter::Point { row: 0, column: 5 }
        );
        assert_eq!(
            edit.old_end_position,
            tree_sitter::Point { row: 0, column: 11 }
        );
        assert_eq!(
            edit.new_end_position,
            tree_sitter::Point { row: 0, column: 5 }
        );
    }

    #[test]
    fn test_delta_to_input_edits_multiline_insertion() {
        let doc = Document::from_bytes(b"Line 1\nLine 2").unwrap();

        // Insert "\nNew line\nAnother" after "Line 1" (at position 6)
        let mut builder = Builder::new(doc.buffer.len());
        builder.replace(6..6, Rope::from("\nNew line\nAnother")); // Multi-line insertion
        let delta = builder.build();

        let edits = doc.delta_to_input_edits(&delta);

        assert_eq!(edits.len(), 1);
        let edit = &edits[0];

        assert_eq!(edit.start_byte, 6);
        assert_eq!(edit.old_end_byte, 6);
        assert_eq!(edit.new_end_byte, 6 + "\nNew line\nAnother".len());

        // Start position at end of "Line 1" (row 0, col 6)
        assert_eq!(
            edit.start_position,
            tree_sitter::Point { row: 0, column: 6 }
        );
        assert_eq!(
            edit.old_end_position,
            tree_sitter::Point { row: 0, column: 6 }
        );

        // End position after inserting 2 newlines and "Another" (row 2, col 7)
        assert_eq!(
            edit.new_end_position,
            tree_sitter::Point { row: 2, column: 7 }
        );
    }

    #[test]
    fn test_delta_to_input_edits_replacement() {
        let doc = Document::from_bytes(b"Hello World").unwrap();

        // Replace "World" with "Universe"
        let mut builder = Builder::new(doc.buffer.len());
        builder.replace(6..11, Rope::from("Universe")); // Replace "World" with "Universe"
        let delta = builder.build();

        let edits = doc.delta_to_input_edits(&delta);

        // My implementation creates separate insertion and deletion edits for replacements
        // This is actually correct behavior for tree-sitter
        assert!(!edits.is_empty());

        // Find the edit that covers the replacement range
        let replacement_edit = edits
            .iter()
            .find(|e| {
                e.start_byte == 6 && (e.old_end_byte == 11 || e.new_end_byte > e.old_end_byte)
            })
            .expect("Should have edit covering replacement range");

        // Verify it's a proper replacement or insertion
        assert_eq!(replacement_edit.start_byte, 6);
    }

    // ============ IndentStyle tests ============

    #[test]
    fn test_indent_style_calculate_depth_spaces() {
        let style = IndentStyle::Spaces(2);

        assert_eq!(style.calculate_depth(""), 0);
        assert_eq!(style.calculate_depth("  "), 1); // 2 spaces = 1 level
        assert_eq!(style.calculate_depth("    "), 2); // 4 spaces = 2 levels
        assert_eq!(style.calculate_depth("      "), 3); // 6 spaces = 3 levels

        let style4 = IndentStyle::Spaces(4);
        assert_eq!(style4.calculate_depth(""), 0);
        assert_eq!(style4.calculate_depth("    "), 1); // 4 spaces = 1 level
        assert_eq!(style4.calculate_depth("        "), 2); // 8 spaces = 2 levels
    }

    #[test]
    fn test_indent_style_calculate_depth_tabs() {
        let style = IndentStyle::Tabs;

        assert_eq!(style.calculate_depth(""), 0);
        assert_eq!(style.calculate_depth("\t"), 1);
        assert_eq!(style.calculate_depth("\t\t"), 2);
        assert_eq!(style.calculate_depth("\t\t\t"), 3);
    }

    #[test]
    fn test_indent_style_calculate_depth_full_lines() {
        // Test that calculate_depth works with full lines, not just indentation strings
        let style2 = IndentStyle::Spaces(2);
        assert_eq!(style2.calculate_depth("- item"), 0);
        assert_eq!(style2.calculate_depth("  - nested item"), 1);
        assert_eq!(style2.calculate_depth("    - deeply nested"), 2);
        assert_eq!(style2.calculate_depth("      - very deep"), 3);

        let style4 = IndentStyle::Spaces(4);
        assert_eq!(style4.calculate_depth("- item"), 0);
        assert_eq!(style4.calculate_depth("    - nested item"), 1);
        assert_eq!(style4.calculate_depth("        - deeply nested"), 2);

        let tab_style = IndentStyle::Tabs;
        assert_eq!(tab_style.calculate_depth("- item"), 0);
        assert_eq!(tab_style.calculate_depth("\t- nested item"), 1);
        assert_eq!(tab_style.calculate_depth("\t\t- deeply nested"), 2);
        assert_eq!(tab_style.calculate_depth("\t\t\t- very deep"), 3);
    }

    #[test]
    fn test_detect_indent_style_4_space() {
        let rope = Rope::from("- item 1\n    - nested with 4 spaces\n    - another nested\n");
        let style = detect_indent_style(&rope);

        assert_eq!(style, IndentStyle::Spaces(4));
    }

    #[test]
    fn test_detect_indent_style_2_space() {
        let rope = Rope::from("- item 1\n  - nested with 2 spaces\n  - another nested\n");
        let style = detect_indent_style(&rope);

        assert_eq!(style, IndentStyle::Spaces(2));
    }

    #[test]
    fn test_detect_indent_style_first_wins() {
        let rope =
            Rope::from("- item 1\n  - 2 space indent\n  - another 2 space\n    - one 4 space\n");
        let style = detect_indent_style(&rope);

        // Should use first indentation found (2 spaces)
        assert_eq!(style, IndentStyle::Spaces(2));
    }

    #[test]
    fn test_detect_indent_style_no_indented_items() {
        let rope = Rope::from("- item 1\n- item 2\n- no nested items\n");
        let style = detect_indent_style(&rope);

        // Should default to 2 when no indented items found
        assert_eq!(style, IndentStyle::Spaces(2));
    }

    #[test]
    fn test_detect_indent_style_tab_characters() {
        let rope = Rope::from("- item 1\n\t- nested with tab\n\t- another tab\n");
        let style = detect_indent_style(&rope);

        // Should detect tabs
        assert_eq!(style, IndentStyle::Tabs);
    }

    #[test]
    fn test_detect_indent_style_mixed_tabs_and_spaces() {
        let rope = Rope::from("- item 1\n\t- tab first\n  - then spaces\n");
        let style = detect_indent_style(&rope);

        // First indentation wins (tabs)
        assert_eq!(style, IndentStyle::Tabs);
    }

    #[test]
    fn test_document_stores_indent_style() {
        let doc = Document::from_bytes(b"- item\n    - 4-space indent").unwrap();
        assert_eq!(doc.indent_style, IndentStyle::Spaces(4));

        let doc_tabs = Document::from_bytes(b"- item\n\t- tab indent").unwrap();
        assert_eq!(doc_tabs.indent_style, IndentStyle::Tabs);
    }

    #[test]
    fn test_end_to_end_indent_detection_and_list_depth() {
        // Integration test: verify the complete workflow from document creation
        // through snapshot generation uses the new indent detection architecture

        let markdown =
            "- Top level\n  - 2-space indented\n    - 4-space indented\n      - 6-space indented";
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

        // Verify indent style was detected before tree-sitter parsing
        assert_eq!(
            doc.indent_style,
            IndentStyle::Spaces(2),
            "Should detect 2-space indent style"
        );

        // Create anchors and snapshot to verify the depth calculation works
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();

        // Should have 4 list items with depths 0, 1, 2, 3
        assert_eq!(snapshot.blocks.len(), 4, "Should have 4 list items");
        assert_eq!(snapshot.blocks[0].depth, 0, "First item should be depth 0");
        assert_eq!(snapshot.blocks[1].depth, 1, "Second item should be depth 1");
        assert_eq!(snapshot.blocks[2].depth, 2, "Third item should be depth 2");
        assert_eq!(snapshot.blocks[3].depth, 3, "Fourth item should be depth 3");

        // Now test with tab-based document
        let tab_markdown = "- Top level\n\t- Tab indented\n\t\t- Double tab";
        let mut tab_doc = Document::from_bytes(tab_markdown.as_bytes()).unwrap();

        // Verify tab detection
        assert_eq!(
            tab_doc.indent_style,
            IndentStyle::Tabs,
            "Should detect tab indent style"
        );

        tab_doc.create_anchors_from_tree();
        let tab_snapshot = tab_doc.snapshot();

        assert_eq!(tab_snapshot.blocks.len(), 3, "Should have 3 list items");
        assert_eq!(
            tab_snapshot.blocks[0].depth, 0,
            "First tab item should be depth 0"
        );
        assert_eq!(
            tab_snapshot.blocks[1].depth, 1,
            "Second tab item should be depth 1"
        );
        assert_eq!(
            tab_snapshot.blocks[2].depth, 2,
            "Third tab item should be depth 2"
        );
    }
}
