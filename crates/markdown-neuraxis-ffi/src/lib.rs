//! UniFFI bindings for markdown-neuraxis mobile apps
//!
//! Provides a minimal FFI interface for the Kotlin Android app to parse
//! and render markdown documents using the Rust engine.
//!
//! See ADR-0011 for the implementation plan.

use markdown_neuraxis_engine::Document;
use markdown_neuraxis_engine::editing::snapshot::{
    Block, BlockContent, BlockKind, InlineSegment, SegmentKind, Snapshot,
};
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
        let source = doc.text();
        let snapshot = doc.snapshot();
        SnapshotDto::from_engine(snapshot, &source)
    }
}

// ============ DTOs ============

/// UI-ready snapshot of a document.
#[derive(uniffi::Record)]
pub struct SnapshotDto {
    /// Document version for change detection (placeholder - always 0 for now)
    pub version: u64,
    /// Hierarchical tree of blocks for rendering
    pub blocks: Vec<BlockDto>,
}

impl SnapshotDto {
    fn from_engine(snapshot: Snapshot, source: &str) -> Self {
        let blocks = convert_blocks(&snapshot.blocks, source);
        Self {
            version: 0, // TODO: Add version to Snapshot when needed
            blocks,
        }
    }
}

/// Convert engine blocks to DTOs recursively, preserving tree structure.
/// List containers are "unwrapped" - their children are promoted to the parent level.
fn convert_blocks(blocks: &[Block], source: &str) -> Vec<BlockDto> {
    let mut result = Vec::new();
    for block in blocks {
        convert_block_into(block, source, &mut result);
    }
    result
}

/// Convert a single engine block to DTOs, appending to the result vector.
/// Some blocks (List, Root) are "unwrapped" and their children are added directly.
fn convert_block_into(block: &Block, source: &str, result: &mut Vec<BlockDto>) {
    match &block.kind {
        BlockKind::Root | BlockKind::List => {
            // Unwrap containers: add children directly to result
            if let BlockContent::Children(children) = &block.content {
                for child in children {
                    convert_block_into(child, source, result);
                }
            }
            return;
        }
        _ => {}
    }

    // Extract content from line ranges
    let content: String = block
        .lines
        .iter()
        .map(|line| &source[line.content.clone()])
        .collect::<Vec<_>>()
        .join("\n");

    let (kind, heading_level, list_marker) = match &block.kind {
        BlockKind::Root | BlockKind::List => unreachable!(), // Handled above
        BlockKind::Paragraph => ("paragraph".to_string(), 0, None),
        BlockKind::Heading { level } => ("heading".to_string(), *level, None),
        BlockKind::ListItem { marker } => ("list_item".to_string(), 0, Some(marker.clone())),
        BlockKind::FencedCode { .. } => ("code_fence".to_string(), 0, None),
        BlockKind::ThematicBreak => ("thematic_break".to_string(), 0, None),
        BlockKind::BlockQuote => ("block_quote".to_string(), 0, None),
    };

    // Convert engine segments to DTOs (engine now provides flat segments)
    let segments: Vec<TextSegmentDto> = block
        .segments
        .iter()
        .map(TextSegmentDto::from_segment)
        .collect();

    // Process children recursively
    let children = if let BlockContent::Children(child_blocks) = &block.content {
        convert_blocks(child_blocks, source)
    } else {
        Vec::new()
    };

    result.push(BlockDto {
        id: block.id.0.to_string(),
        kind,
        heading_level,
        list_marker,
        content,
        segments,
        children,
    });
}

/// A single block in the document tree.
#[derive(uniffi::Record)]
pub struct BlockDto {
    /// Stable identifier for this block (persists across edits)
    pub id: String,
    /// Block type (e.g., "heading", "list_item", "paragraph")
    pub kind: String,
    /// Heading level (1-6) if this is a heading, 0 otherwise
    pub heading_level: u8,
    /// List marker if this is a list item
    pub list_marker: Option<String>,
    /// The text content of this block
    pub content: String,
    /// Parsed inline segments (wiki-links, URLs, plain text)
    pub segments: Vec<TextSegmentDto>,
    /// Child blocks (e.g., nested list items)
    pub children: Vec<BlockDto>,
}

/// A segment of inline content within a block.
#[derive(uniffi::Record)]
pub struct TextSegmentDto {
    /// Segment type: "text", "wiki_link", "url", "emphasis", "strong", "code", "link", "image"
    pub kind: String,
    /// The text content or link target
    pub content: String,
}

impl TextSegmentDto {
    fn from_segment(segment: &InlineSegment) -> Self {
        match &segment.kind {
            SegmentKind::Text(text) => Self {
                kind: "text".to_string(),
                content: text.clone(),
            },
            SegmentKind::WikiLink { target, alias } => Self {
                kind: "wiki_link".to_string(),
                // Use alias if present, otherwise target (for display)
                content: alias.as_ref().unwrap_or(target).clone(),
            },
            SegmentKind::Link { text, url } => Self {
                kind: "link".to_string(),
                content: format!("{}|{}", text, url),
            },
            SegmentKind::Emphasis(text) => Self {
                kind: "emphasis".to_string(),
                content: text.clone(),
            },
            SegmentKind::Strong(text) => Self {
                kind: "strong".to_string(),
                content: text.clone(),
            },
            SegmentKind::Code(text) => Self {
                kind: "code".to_string(),
                content: text.clone(),
            },
            SegmentKind::Image { alt, url } => Self {
                kind: "image".to_string(),
                content: format!("{}|{}", alt, url),
            },
            SegmentKind::Strikethrough(text) => Self {
                kind: "strikethrough".to_string(),
                content: text.clone(),
            },
            SegmentKind::HardBreak => Self {
                kind: "hard_break".to_string(),
                content: String::new(),
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

    /// Recursively collect all blocks from the tree into a flat list
    fn collect_all_blocks(blocks: &[BlockDto]) -> Vec<&BlockDto> {
        let mut result = Vec::new();
        for block in blocks {
            result.push(block);
            result.extend(collect_all_blocks(&block.children));
        }
        result
    }

    /// Find a block by kind in the tree (depth-first)
    fn find_block_by_kind<'a>(blocks: &'a [BlockDto], kind: &str) -> Option<&'a BlockDto> {
        for block in blocks {
            if block.kind == kind {
                return Some(block);
            }
            if let Some(found) = find_block_by_kind(&block.children, kind) {
                return Some(found);
            }
        }
        None
    }

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
    fn test_block_dto_kinds() {
        let content = "# H1\n## H2\n\n- Dash\n* Star\n+ Plus\n1. Numbered\n\n---\n\n> Quote\n\n```rust\ncode\n```";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        // Collect all blocks from tree
        let all_blocks = collect_all_blocks(&snapshot.blocks);

        // Find heading blocks
        let headings: Vec<_> = all_blocks.iter().filter(|b| b.kind == "heading").collect();
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].heading_level, 1);
        assert_eq!(headings[1].heading_level, 2);

        // Find list items
        let list_items: Vec<_> = all_blocks
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

        // Find the list item in the tree
        let list_item = find_block_by_kind(&snapshot.blocks, "list_item");
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

    #[test]
    fn test_emphasis_at_eof_no_newline() {
        // Minimal repro: emphasis at end of file without trailing newline
        // Bug: content range computed as (range.start + 1)..(range.end - 1)
        // but range.end points past EOF, so we get start > end
        let content = "*emphasis*";
        assert!(!content.ends_with('\n'));

        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        assert!(!snapshot.blocks.is_empty());
    }

    #[test]
    fn test_nested_list_tree_structure() {
        // Verify nested lists produce a proper tree, not a flat list
        let content = "- parent\n  - child 1\n  - child 2\n    - grandchild";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        // Top level should have the parent list item
        assert_eq!(snapshot.blocks.len(), 1);
        let parent = &snapshot.blocks[0];
        assert_eq!(parent.kind, "list_item");
        assert!(parent.content.contains("parent"));

        // Parent should have nested items
        assert!(
            !parent.children.is_empty(),
            "Parent should have nested items"
        );

        // Count total nested list items (child 1, child 2, grandchild)
        let all_nested = collect_all_blocks(&parent.children);
        let nested_list_items: Vec<_> = all_nested
            .iter()
            .filter(|b| b.kind == "list_item")
            .collect();
        assert_eq!(
            nested_list_items.len(),
            3,
            "Should have 3 nested items total (child 1, child 2, grandchild)"
        );

        // Verify we can traverse to find specific items
        let grandchild = find_block_by_kind(&parent.children, "list_item")
            .and_then(|c1| find_block_by_kind(&c1.children, "list_item"))
            .and_then(|c2| find_block_by_kind(&c2.children, "list_item"));
        assert!(grandchild.is_some(), "Should find grandchild through tree");
        assert!(grandchild.unwrap().content.contains("grandchild"));
    }

    #[test]
    fn test_blockquote_tree_structure() {
        // Verify blockquotes with content produce correct structure
        let content = "> This is a quote";
        let doc = DocumentHandle::from_string(content.to_string()).unwrap();
        let snapshot = doc.get_snapshot();

        assert_eq!(snapshot.blocks.len(), 1);
        let quote = &snapshot.blocks[0];
        assert_eq!(quote.kind, "block_quote");
        assert_eq!(quote.content, "This is a quote");
    }
}
