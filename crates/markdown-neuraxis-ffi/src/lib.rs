//! UniFFI bindings for markdown-neuraxis mobile apps
//!
//! Provides a minimal FFI interface for the Kotlin Android app to parse
//! and render markdown documents using the Rust engine.
//!
//! See ADR-0011 for the implementation plan.

use markdown_neuraxis_engine::{BlockKind, Document, Marker, RenderBlock, Snapshot, TextSegment};
use std::sync::Mutex;

uniffi::setup_scaffolding!();

// ============ Errors ============

/// Errors that can cross the FFI boundary
/// Note: Field is named `reason` not `message` to avoid conflict with Throwable.message in Kotlin
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("Parse error: {reason}")]
    ParseError { reason: String },
}

// ============ Document Handle ============

/// A handle to a parsed markdown document.
///
/// This wraps the engine's Document type and provides a simple API
/// for mobile apps to interact with markdown content.
#[derive(uniffi::Object)]
pub struct DocumentHandle {
    inner: Mutex<Document>,
}

#[uniffi::export]
impl DocumentHandle {
    /// Create a document from markdown content string.
    #[uniffi::constructor]
    pub fn from_string(content: String) -> Result<Self, FfiError> {
        let mut doc =
            Document::from_bytes(content.as_bytes()).map_err(|e| FfiError::ParseError {
                reason: e.to_string(),
            })?;
        doc.create_anchors_from_tree();

        Ok(Self {
            inner: Mutex::new(doc),
        })
    }

    /// Get the current text content of the document.
    pub fn get_text(&self) -> String {
        // Recover from poisoned mutex (another thread panicked while holding lock)
        let doc = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        doc.text()
    }

    /// Get a snapshot of the document for UI rendering.
    pub fn get_snapshot(&self) -> SnapshotDto {
        // Recover from poisoned mutex (another thread panicked while holding lock)
        let doc = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let snapshot = doc.snapshot();
        SnapshotDto::from_engine(snapshot)
    }
}

// ============ DTOs ============

/// UI-ready snapshot of a document.
#[derive(uniffi::Record)]
pub struct SnapshotDto {
    /// Document version for change detection
    pub version: u64,
    /// Flat list of blocks for rendering
    pub blocks: Vec<RenderBlockDto>,
}

impl SnapshotDto {
    fn from_engine(snapshot: Snapshot) -> Self {
        Self {
            version: snapshot.version,
            blocks: snapshot
                .blocks
                .into_iter()
                .map(RenderBlockDto::from_engine)
                .collect(),
        }
    }
}

/// A single renderable block in the document.
#[derive(uniffi::Record)]
pub struct RenderBlockDto {
    /// Stable identifier for this block (persists across edits)
    pub id: String,
    /// Block type (e.g., "heading", "list_item", "paragraph")
    pub kind: String,
    /// Heading level (1-6) if this is a heading, 0 otherwise
    pub heading_level: u8,
    /// List marker if this is a list item
    pub list_marker: Option<String>,
    /// Nesting depth for indentation
    pub depth: u32,
    /// The text content of this block
    pub content: String,
    /// Parsed inline segments (wiki-links, URLs, plain text)
    pub segments: Vec<TextSegmentDto>,
}

impl RenderBlockDto {
    fn from_engine(block: RenderBlock) -> Self {
        let (kind, heading_level, list_marker) = match block.kind {
            BlockKind::Paragraph => ("paragraph".to_string(), 0, None),
            BlockKind::Heading { level } => ("heading".to_string(), level, None),
            BlockKind::ListItem { marker, .. } => {
                let marker_str = match marker {
                    Marker::Dash => "-".to_string(),
                    Marker::Asterisk => "*".to_string(),
                    Marker::Plus => "+".to_string(),
                    Marker::Numbered(s) => s, // "1.", "2.", etc.
                };
                ("list_item".to_string(), 0, Some(marker_str))
            }
            BlockKind::CodeFence { .. } => ("code_fence".to_string(), 0, None),
            BlockKind::ThematicBreak => ("thematic_break".to_string(), 0, None),
            BlockKind::BlockQuote => ("block_quote".to_string(), 0, None),
            BlockKind::UnhandledMarkdown => ("unhandled".to_string(), 0, None),
        };

        let segments = block
            .segments
            .unwrap_or_default()
            .into_iter()
            .map(TextSegmentDto::from_engine)
            .collect();

        Self {
            id: block.id.0.to_string(),
            kind,
            heading_level,
            list_marker,
            depth: block.depth as u32,
            content: block.content,
            segments,
        }
    }
}

/// A segment of inline content within a block.
#[derive(uniffi::Record)]
pub struct TextSegmentDto {
    /// Segment type: "text", "wiki_link", or "url"
    pub kind: String,
    /// The text content or link target
    pub content: String,
}

impl TextSegmentDto {
    fn from_engine(segment: TextSegment) -> Self {
        match segment {
            TextSegment::Text(text) => Self {
                kind: "text".to_string(),
                content: text,
            },
            TextSegment::WikiLink { target } => Self {
                kind: "wiki_link".to_string(),
                content: target,
            },
            TextSegment::Url { href } => Self {
                kind: "url".to_string(),
                content: href,
            },
        }
    }
}

// ============ Standalone Functions ============

/// Resolve a wiki-link target to a file path.
///
/// Searches the given file paths for a match (case-insensitive, with or without .md extension).
/// Returns the matching file path, or None if not found.
#[uniffi::export]
pub fn resolve_wikilink(target: String, file_paths: Vec<String>) -> Option<String> {
    let search_name = target
        .strip_suffix(".md")
        .or_else(|| target.strip_suffix(".MD"))
        .unwrap_or(&target)
        .to_lowercase();

    file_paths.into_iter().find(|path| {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let name_without_ext = filename
            .strip_suffix(".md")
            .or_else(|| filename.strip_suffix(".MD"))
            .unwrap_or(filename);
        name_without_ext.to_lowercase() == search_name
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_from_string() {
        let content = "# Hello World\n\n- Item 1\n- Item 2";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();

        let text = doc.get_text();
        assert_eq!(text, content);
    }

    #[test]
    fn test_get_snapshot() {
        let content = "# Heading\n\nParagraph text\n\n- List item";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();

        let snapshot = doc.get_snapshot();
        // version is u64, no need to check >= 0
        assert!(!snapshot.blocks.is_empty());

        // Check first block is heading
        let heading = &snapshot.blocks[0];
        assert_eq!(heading.kind, "heading");
        assert_eq!(heading.heading_level, 1);
        assert_eq!(heading.content, "Heading");
    }

    #[test]
    fn test_render_block_dto_kinds() {
        let content = "# H1\n## H2\n\n- Dash\n* Star\n+ Plus\n1. Numbered\n\n---\n\n> Quote\n\n```rust\ncode\n```";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        // Find heading blocks
        let headings: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.kind == "heading")
            .collect();
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].heading_level, 1);
        assert_eq!(headings[1].heading_level, 2);

        // Find list items
        let list_items: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.kind == "list_item")
            .collect();
        assert!(!list_items.is_empty());
    }

    #[test]
    fn test_simple_string_parses() {
        let doc = DocumentHandle::from_string("Hello".to_string());
        assert!(doc.is_ok());
    }

    #[test]
    fn test_wiki_links_in_segments() {
        let content = "- Check [[My Page]] for info";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        // Find the list item
        let list_item = snapshot.blocks.iter().find(|b| b.kind == "list_item");
        assert!(list_item.is_some());

        let segments = &list_item.unwrap().segments;
        let wiki_link = segments.iter().find(|s| s.kind == "wiki_link");
        assert!(wiki_link.is_some());
        assert_eq!(wiki_link.unwrap().content, "My Page");
    }

    #[test]
    fn test_resolve_wikilink_exact_match() {
        let paths = vec![
            "notes/My Page.md".to_string(),
            "journal/2024_01_01.md".to_string(),
        ];
        let result = resolve_wikilink("My Page".to_string(), paths);
        assert_eq!(result, Some("notes/My Page.md".to_string()));
    }

    #[test]
    fn test_resolve_wikilink_case_insensitive() {
        let paths = vec!["Notes/my page.md".to_string()];
        let result = resolve_wikilink("My Page".to_string(), paths);
        assert_eq!(result, Some("Notes/my page.md".to_string()));
    }

    #[test]
    fn test_resolve_wikilink_with_extension() {
        let paths = vec!["docs/README.md".to_string()];
        let result = resolve_wikilink("README.md".to_string(), paths);
        assert_eq!(result, Some("docs/README.md".to_string()));
    }

    #[test]
    fn test_resolve_wikilink_not_found() {
        let paths = vec!["notes/Other.md".to_string()];
        let result = resolve_wikilink("Missing".to_string(), paths);
        assert_eq!(result, None);
    }
}
