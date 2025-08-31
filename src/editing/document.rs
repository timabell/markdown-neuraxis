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

/// Core document structure that holds the text buffer and provides editing operations.
/// Uses xi-rope for efficient text manipulation and preserves exact byte representation.
pub struct Document {
    /// The rope buffer containing the entire document as UTF-8 bytes
    pub(crate) buffer: Rope,
    /// Current selection/cursor position as byte offsets
    pub(crate) selection: std::ops::Range<usize>,
    /// Version number that increments with each edit
    pub(crate) version: u64,
    /// Tree-sitter parser for incremental parsing
    pub(crate) parser: Parser,
    /// Current parse tree (None until first parse)
    pub(crate) tree: Option<Tree>,
    /// Anchors for stable block IDs that survive edits
    pub(crate) anchors: Vec<Anchor>,
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

    /// Get the buffer length
    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Slice the buffer to a cow string
    pub(crate) fn slice_to_cow(
        &self,
        range: impl xi_rope::interval::IntervalBounds,
    ) -> std::borrow::Cow<'_, str> {
        self.buffer.slice_to_cow(range)
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

    pub fn snapshot(&self) -> crate::editing::Snapshot {
        crate::editing::snapshot::create_snapshot(self)
    }
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
