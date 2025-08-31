use xi_rope::Rope;

/// Core document structure that holds the text buffer and provides editing operations.
/// Uses xi-rope for efficient text manipulation and preserves exact byte representation.
pub struct Document {
    /// The rope buffer containing the entire document as UTF-8 bytes
    buffer: Rope,
}

impl Document {
    /// Create a new document from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        // Convert bytes to string, ensuring valid UTF-8
        let text = std::str::from_utf8(bytes)?;
        let buffer = Rope::from(text);
        Ok(Self { buffer })
    }

    /// Get the document's content as raw bytes (exact round-trip)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buffer.to_string().into_bytes()
    }

    /// Apply a command to the document (placeholder for now)
    pub fn apply(&mut self, _cmd: Cmd) -> Patch {
        todo!("Command application will be implemented later")
    }

    /// Get a snapshot of the document for rendering (placeholder for now)
    pub fn snapshot(&self) -> Snapshot {
        todo!("Snapshot generation will be implemented later")
    }
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

    #[test]
    fn test_document_from_bytes_valid_utf8() {
        let text = "# Hello World\n\nThis is a test document.";
        let bytes = text.as_bytes();

        let doc = Document::from_bytes(bytes).expect("Should create document from valid UTF-8");

        // The document should be created successfully
        assert_eq!(doc.to_bytes(), bytes);
    }

    #[test]
    fn test_document_from_bytes_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence

        let result = Document::from_bytes(&invalid_bytes);

        // Should return an error for invalid UTF-8
        assert!(result.is_err());
    }

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
    fn test_document_empty() {
        let empty = b"";

        let doc = Document::from_bytes(empty).expect("Should create empty document");
        let result = doc.to_bytes();

        // Empty document should round-trip correctly
        assert_eq!(result, empty);
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
