use crate::editing::{AnchorId, Document, anchors::find_anchor_for_range, document::Marker};

/// Snapshot of the document for rendering
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub version: u64,
    pub blocks: Vec<RenderBlock>,
}

/// A renderable block in the document
#[derive(Clone, PartialEq)]
pub struct RenderBlock {
    pub id: AnchorId,
    pub kind: BlockKind,
    pub byte_range: std::ops::Range<usize>,
    pub content_range: std::ops::Range<usize>,
    pub depth: usize,
    pub content: String,
}

/// Block types for rendering
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    Paragraph,
    Heading { level: u8 },
    ListItem { marker: Marker, depth: usize },
    CodeFence { lang: Option<String> },
}

/// Get a snapshot of the document for rendering
pub(crate) fn create_snapshot(doc: &Document) -> Snapshot {
    let mut blocks = Vec::new();

    if let Some(ref tree) = doc.tree {
        let root_node = tree.root_node();
        collect_render_blocks_recursive(doc, root_node, &mut blocks, 0);
    }

    Snapshot {
        version: doc.version,
        blocks,
    }
}

/// Recursively collect render blocks from the tree-sitter CST
fn collect_render_blocks_recursive(
    doc: &Document,
    node: tree_sitter::Node,
    blocks: &mut Vec<RenderBlock>,
    current_depth: usize,
) {
    let node_kind = node.kind();
    let byte_range = node.byte_range();

    // Skip empty nodes
    if byte_range.is_empty() {
        return;
    }

    match node_kind {
        "atx_heading" => {
            let level = extract_heading_level(doc, &node);
            let content_range = extract_heading_content_range(doc, &node);
            let anchor_id = find_anchor_for_range(doc, &byte_range);
            let content = doc.slice_to_cow(content_range.clone()).trim().to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::Heading { level },
                byte_range,
                content_range,
                depth: current_depth,
                content,
            });
        }
        "list_item" => {
            let marker = extract_list_marker(doc, &node);
            let list_depth = calculate_list_depth(doc, &node);
            let content_range = extract_list_item_content_range(doc, &node);
            let anchor_id = find_anchor_for_range(doc, &byte_range);
            let content = doc.slice_to_cow(content_range.clone()).trim().to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::ListItem {
                    marker,
                    depth: list_depth,
                },
                byte_range,
                content_range,
                depth: list_depth,
                content,
            });

            // Also recursively process children to find nested list items
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_render_blocks_recursive(doc, child, blocks, list_depth);
            }
        }
        "paragraph" => {
            // Only create paragraph render blocks if they're not inside list items
            // Check if the parent is a list_item
            let is_inside_list_item = node.parent().map(|p| p.kind()) == Some("list_item");

            if !is_inside_list_item {
                // Top-level paragraph
                let anchor_id = find_anchor_for_range(doc, &byte_range);
                let content = doc.slice_to_cow(byte_range.clone()).trim().to_string();

                blocks.push(RenderBlock {
                    id: anchor_id,
                    kind: BlockKind::Paragraph,
                    byte_range: byte_range.clone(),
                    content_range: byte_range.clone(), // For paragraphs, content equals byte range
                    depth: current_depth,
                    content,
                });
            }
            // If inside a list item, skip the paragraph block entirely
            // The list item will handle its own content
        }
        "fenced_code_block" => {
            let lang = extract_code_fence_language(doc, &node);
            let content_range = extract_code_fence_content_range(doc, &node);
            let anchor_id = find_anchor_for_range(doc, &byte_range);
            let content = doc.slice_to_cow(content_range.clone()).to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::CodeFence { lang },
                byte_range,
                content_range,
                depth: current_depth,
                content,
            });
        }
        "indented_code_block" => {
            let anchor_id = find_anchor_for_range(doc, &byte_range);
            let content = doc.slice_to_cow(byte_range.clone()).to_string();

            blocks.push(RenderBlock {
                id: anchor_id,
                kind: BlockKind::CodeFence { lang: None },
                byte_range: byte_range.clone(),
                content_range: byte_range.clone(),
                depth: current_depth,
                content,
            });
        }
        _ => {
            // For other node types, recursively process children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_render_blocks_recursive(doc, child, blocks, current_depth);
            }
        }
    }
}

/// Extract heading level from an ATX heading node
fn extract_heading_level(doc: &Document, node: &tree_sitter::Node) -> u8 {
    let text = doc.slice_to_cow(node.byte_range());
    // Count the number of # characters at the start
    let level = text.chars().take_while(|&c| c == '#').count() as u8;
    level.clamp(1, 6) // ATX headings are level 1-6
}

/// Extract content range for a heading (after the # markers and space)
fn extract_heading_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find where the content starts (after # and space)
    let mut content_start = byte_range.start;
    let chars = text.char_indices();

    // Skip the # characters
    for (i, ch) in chars {
        if ch == '#' {
            content_start = byte_range.start + i + 1;
        } else {
            break;
        }
    }

    // Skip exactly one space after the #'s
    if text.as_bytes().get(content_start - byte_range.start) == Some(&b' ') {
        content_start += 1;
    }

    // Content ends at the end of the heading line, but exclude any trailing newline
    let mut content_end = byte_range.end;
    if text.ends_with('\n') {
        content_end -= 1;
    }

    content_start..content_end
}

/// Extract list marker from a list item node
fn extract_list_marker(doc: &Document, node: &tree_sitter::Node) -> Marker {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range);

    // Find the marker in the text
    let trimmed = text.trim_start();

    if trimmed.starts_with("- ") {
        Marker::Dash
    } else if trimmed.starts_with("* ") {
        Marker::Asterisk
    } else if trimmed.starts_with("+ ") {
        Marker::Plus
    } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        // Numbered list (1., 2., etc.)
        Marker::Numbered
    } else {
        // Default to dash if we can't determine
        Marker::Dash
    }
}

/// Calculate the depth of a list item based on indentation
fn calculate_list_depth(doc: &Document, node: &tree_sitter::Node) -> usize {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range);

    // Count leading spaces/tabs
    let indent_chars = text.chars().take_while(|&c| c == ' ' || c == '\t').count();

    // Each 2 spaces = 1 depth level (common markdown convention)
    indent_chars / 2
}

/// Extract content range for a list item (after the marker and space)
fn extract_list_item_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find the start of content (after indentation and marker)
    let trimmed = text.trim_start();
    let indent_len = text.len() - trimmed.len();

    let mut marker_len = 0;
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        marker_len = 2; // "- " or "* " or "+ "
    } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        // Find the numbered marker like "1. "
        if let Some(dot_pos) = trimmed.find(". ") {
            marker_len = dot_pos + 2; // "N. "
        }
    }

    let content_start = byte_range.start + indent_len + marker_len;

    // For list items, the content should only be the text on the first line,
    // not including nested content
    let first_line_text = &text[indent_len + marker_len..];
    let content_end = if let Some(newline_pos) = first_line_text.find('\n') {
        content_start + newline_pos
    } else {
        byte_range.end
    };

    content_start..content_end
}

/// Extract language from a fenced code block
fn extract_code_fence_language(doc: &Document, node: &tree_sitter::Node) -> Option<String> {
    // Look for the info string on the first line
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range);

    if let Some(first_line_end) = text.find('\n') {
        let first_line = &text[..first_line_end];

        // Remove the fence markers (``` or ~~~) and get the language
        let lang_part = first_line
            .trim_start_matches('`')
            .trim_start_matches('~')
            .trim();

        if lang_part.is_empty() {
            None
        } else {
            Some(lang_part.to_string())
        }
    } else {
        None
    }
}

/// Extract content range for a fenced code block (the code inside)
fn extract_code_fence_content_range(
    doc: &Document,
    node: &tree_sitter::Node,
) -> std::ops::Range<usize> {
    let byte_range = node.byte_range();
    let text = doc.slice_to_cow(byte_range.clone());

    // Find the end of the first line (opening fence)
    let content_start = if let Some(first_newline) = text.find('\n') {
        byte_range.start + first_newline + 1
    } else {
        byte_range.start
    };

    // Find the start of the last line (closing fence)
    let content_end = if let Some(last_newline) = text.rfind('\n') {
        // Check if there's a closing fence
        let potential_close = &text[last_newline + 1..];
        if potential_close.trim_start().starts_with("```")
            || potential_close.trim_start().starts_with("~~~")
        {
            byte_range.start + last_newline
        } else {
            byte_range.end
        }
    } else {
        byte_range.end
    };

    content_start..content_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{Document, commands::Cmd};

    // ============ Snapshot API tests ============

    #[test]
    fn test_snapshot_empty_document() {
        let doc = Document::from_bytes(b"").unwrap();
        let snapshot = doc.snapshot();

        assert_eq!(snapshot.version, 0);
        assert_eq!(snapshot.blocks.len(), 0);
    }

    #[test]
    fn test_snapshot_simple_heading() {
        let mut doc = Document::from_bytes(b"# Hello World").unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.version, 0);
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert_eq!(block.kind, BlockKind::Heading { level: 1 });
        assert_eq!(block.byte_range, 0..13);
        assert_eq!(block.content_range, 2..13); // After "# " prefix
        assert_eq!(block.depth, 0);
    }

    #[test]
    fn test_snapshot_multiple_headings() {
        let text = "# Heading 1\n\n## Heading 2\n\n### Heading 3";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 3);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::Heading { level: 1 }
        ));
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::Heading { level: 2 }
        ));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::Heading { level: 3 }
        ));

        // Check content ranges exclude the markdown prefixes
        assert_eq!(snapshot.blocks[0].content_range, 2..11); // After "# "
        assert_eq!(snapshot.blocks[1].content_range, 16..25); // After "## "
        assert_eq!(snapshot.blocks[2].content_range, 31..40); // After "### "
    }

    #[test]
    fn test_snapshot_simple_list() {
        let text = "- Item 1\n- Item 2\n- Item 3";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 3);

        for block in &snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0
                }
            ));
            assert_eq!(block.depth, 0);
        }

        // Content ranges should exclude structural elements like newlines
        assert_eq!(snapshot.blocks[0].content_range, 2..8); // "Item 1" (excluding newline)
        assert_eq!(snapshot.blocks[1].content_range, 11..17); // "Item 2" (excluding newline)
        assert_eq!(snapshot.blocks[2].content_range, 20..26); // "Item 3" (no trailing newline)
    }

    #[test]
    fn test_snapshot_nested_list() {
        let text = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // We should have exactly 4 list items now that we filter out paragraph nodes
        assert_eq!(snapshot.blocks.len(), 4);

        // All blocks should be list items with dash markers
        for block in &snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: _
                }
            ));
        }
    }

    #[test]
    fn test_snapshot_different_list_markers() {
        let text = "- Dash item\n* Star item\n+ Plus item\n1. Numbered item";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 4);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::ListItem {
                marker: Marker::Asterisk,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::ListItem {
                marker: Marker::Plus,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[3].kind,
            BlockKind::ListItem {
                marker: Marker::Numbered,
                depth: 0
            }
        ));
    }

    #[test]
    fn test_snapshot_mixed_content() {
        let text = "# Main Heading\n\nThis is a paragraph.\n\n- List item 1\n- List item 2\n\n## Sub Heading";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Should have: heading, paragraph, 2 list items, heading
        assert_eq!(snapshot.blocks.len(), 5);

        assert!(matches!(
            snapshot.blocks[0].kind,
            BlockKind::Heading { level: 1 }
        ));
        assert!(matches!(snapshot.blocks[1].kind, BlockKind::Paragraph));
        assert!(matches!(
            snapshot.blocks[2].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[3].kind,
            BlockKind::ListItem {
                marker: Marker::Dash,
                depth: 0
            }
        ));
        assert!(matches!(
            snapshot.blocks[4].kind,
            BlockKind::Heading { level: 2 }
        ));
    }

    #[test]
    fn test_snapshot_code_fences() {
        let text = "```rust\nfn main() {}\n```\n\n```\nplain code\n```";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        assert_eq!(snapshot.blocks.len(), 2);

        // First code fence with language
        assert!(
            matches!(snapshot.blocks[0].kind, BlockKind::CodeFence { lang: Some(ref lang) } if lang == "rust")
        );

        // Second code fence without language
        assert!(matches!(
            snapshot.blocks[1].kind,
            BlockKind::CodeFence { lang: None }
        ));
    }

    #[test]
    fn test_snapshot_anchor_association() {
        let text = "# Heading\n\n- Item 1\n- Item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Due to granular parsing, we might get more blocks than expected
        assert!(snapshot.blocks.len() >= 3);

        // Each block should have a unique anchor ID
        let mut ids = std::collections::HashSet::new();
        for block in &snapshot.blocks {
            assert!(
                ids.insert(block.id),
                "Each block should have a unique anchor ID"
            );
        }

        // Every document anchor ID should appear in the blocks
        // (though blocks may have additional temporary IDs for paragraphs etc.)
        let doc_anchor_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();
        let block_anchor_ids: std::collections::HashSet<AnchorId> =
            snapshot.blocks.iter().map(|b| b.id).collect();

        for doc_anchor_id in &doc_anchor_ids {
            assert!(
                block_anchor_ids.contains(doc_anchor_id),
                "Document anchor ID {doc_anchor_id:?} should appear in blocks"
            );
        }
    }

    #[test]
    fn test_snapshot_version_tracking() {
        let mut doc = Document::from_bytes(b"# Test").unwrap();
        doc.create_anchors_from_tree();

        let initial_snapshot = doc.snapshot();
        assert_eq!(initial_snapshot.version, 0);

        // Make an edit
        doc.apply(Cmd::InsertText {
            at: 6,
            text: " Document".to_string(),
        });

        let updated_snapshot = doc.snapshot();
        assert_eq!(updated_snapshot.version, 1);
    }

    #[test]
    fn test_snapshot_after_edits() {
        let mut doc = Document::from_bytes(b"- Item 1").unwrap();
        doc.create_anchors_from_tree();

        // Initial snapshot
        let initial_snapshot = doc.snapshot();
        assert_eq!(initial_snapshot.blocks.len(), 1);

        // Add a new list item
        doc.apply(Cmd::SplitListItem { at: 8 });
        doc.apply(Cmd::InsertText {
            at: 11,
            text: "Item 2".to_string(),
        });

        let updated_snapshot = doc.snapshot();
        assert_eq!(updated_snapshot.blocks.len(), 2);
        assert_eq!(updated_snapshot.version, 2);

        // Both should be list items
        for block in &updated_snapshot.blocks {
            assert!(matches!(
                block.kind,
                BlockKind::ListItem {
                    marker: Marker::Dash,
                    depth: 0
                }
            ));
        }
    }

    #[test]
    fn test_snapshot_content_ranges_after_edit() {
        let mut doc = Document::from_bytes(b"# Heading").unwrap();
        doc.create_anchors_from_tree();

        // Add text to the heading
        doc.apply(Cmd::InsertText {
            at: 9,
            text: " Extended".to_string(),
        });

        let snapshot = doc.snapshot();
        assert_eq!(snapshot.blocks.len(), 1);

        let block = &snapshot.blocks[0];
        assert!(matches!(block.kind, BlockKind::Heading { level: 1 }));
        assert_eq!(block.content_range, 2..18); // Should include the extended text
        assert_eq!(&doc.text()[block.content_range.clone()], "Heading Extended");
    }
}
