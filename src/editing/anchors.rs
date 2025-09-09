use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use xi_rope::delta::Transformer;
use xi_rope::{Delta, RopeInfo};

use crate::editing::Document;

/// Stable identifier for a text range that survives edits
#[derive(Clone, Debug, PartialEq)]
pub struct Anchor {
    pub id: AnchorId,
    pub range: std::ops::Range<usize>, // byte range in the rope
    pub node_id: Option<usize>,        // tree-sitter node ID for direct mapping
                                       // TODO v2: add bias/stickiness and kind hints
}

/// Unique identifier for an anchor
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AnchorId(pub u128);

/// Calculate the overlap between two byte ranges
/// Returns the overlapping portion length, or 0 if they don't overlap
#[cfg(test)]
pub(crate) fn calculate_range_overlap(
    range1: &std::ops::Range<usize>,
    range2: &std::ops::Range<usize>,
) -> usize {
    let start = range1.start.max(range2.start);
    let end = range1.end.min(range2.end);
    end.saturating_sub(start)
}

/// Transform anchors through a delta operation
pub(crate) fn transform_anchors(doc: &mut Document, delta: &Delta<RopeInfo>) {
    // Create a transformer for this delta
    let mut transformer = Transformer::new(delta);
    let doc_len = doc.len();

    // Transform each anchor's range through the delta
    for anchor in &mut doc.anchors {
        // Transform both start and end positions with different strategies:
        // For the start: use after=true so insertions at the exact start move the anchor forward
        // For the end: use after=false so insertions at the exact end don't expand the anchor
        let new_start = transformer.transform(anchor.range.start, true);
        let new_end = transformer.transform(anchor.range.end, false);

        // Only update if the transformation produces a valid range
        if new_start <= new_end && new_end <= doc_len {
            anchor.range = new_start..new_end;
        } else {
            // If transformation results in invalid range, clamp to valid bounds
            // This will be refined in rebinding
            let clamped_start = new_start.min(doc_len);
            let clamped_end = new_end.min(doc_len).max(clamped_start);
            anchor.range = clamped_start..clamped_end;
        }
    }

    // Remove anchors that have become empty or invalid
    let final_doc_len = doc.len();
    doc.anchors.retain(|anchor| {
        anchor.range.start < anchor.range.end && anchor.range.end <= final_doc_len
    });
}

/// Rebind anchors in changed regions to maintain stable block associations
/// Applies deterministic rebinding when needed to prevent anchor confusion
pub(crate) fn rebind_anchors_in_changed_regions(
    doc: &mut Document,
    changed: &[std::ops::Range<usize>],
) {
    if doc.tree.is_none() || changed.is_empty() {
        return;
    }

    let tree = doc.tree.as_ref().unwrap();
    let root_node = tree.root_node();

    // Collect what the new anchor structure would be
    let mut new_anchor_data = Vec::new();
    collect_anchor_ranges_recursive(root_node, &mut new_anchor_data);

    // Check if changes affect existing anchor ranges
    // This catches the anchor confusion case where list items are edited
    let affects_anchor_content = changed.iter().any(|change_range| {
        doc.anchors.iter().any(|anchor| {
            // Check if the change is within an anchor's range (not at the boundaries)
            change_range.start > anchor.range.start && change_range.start < anchor.range.end
        })
    });

    // Check if changes are at the boundaries or outside existing anchors
    let changes_outside_anchors = changed.iter().all(|change_range| {
        // Changes that are completely before, after, or at boundaries of all anchors
        doc.anchors.iter().all(|anchor| {
            change_range.end <= anchor.range.start || // Before anchor
            change_range.start >= anchor.range.end || // After anchor  
            change_range.start == anchor.range.start || // At start boundary
            change_range.start == anchor.range.end // At end boundary
        })
    });

    if affects_anchor_content {
        // Changes within existing anchors - apply deterministic rebinding
        apply_deterministic_rebinding(doc, new_anchor_data);
    } else if new_anchor_data.len() != doc.anchors.len() && !changes_outside_anchors {
        // Anchor count changed AND changes affect anchor positions - this indicates structural changes
        apply_deterministic_rebinding(doc, new_anchor_data);
    }
    // Otherwise, changes are outside anchors (like inserting before document)
    // Trust transform_anchors - it already handled the position shifts
}

/// Apply deterministic rebinding when structural changes have occurred
fn apply_deterministic_rebinding(
    doc: &mut Document,
    new_anchor_data: Vec<(std::ops::Range<usize>, Option<usize>, String)>,
) {
    let old_anchors = doc.anchors.clone();
    doc.anchors.clear();

    // Sort both by position for deterministic processing
    let mut old_anchors_by_position = old_anchors.clone();
    old_anchors_by_position.sort_by_key(|a| a.range.start);
    let mut sorted_new_data = new_anchor_data;
    sorted_new_data.sort_by_key(|(range, _, _)| range.start);

    // For stable rebinding, keep track of which old anchor IDs have been used
    let mut used_old_ids = std::collections::HashSet::new();

    // Create new anchors, preserving IDs where possible
    for (new_index, (new_range, new_node_id, semantic_type)) in
        sorted_new_data.into_iter().enumerate()
    {
        let anchor_id = determine_anchor_id_deterministic(
            new_node_id,
            new_index,
            &old_anchors,
            &old_anchors_by_position,
            &new_range,
            &semantic_type,
            &mut used_old_ids,
        );

        let anchor = Anchor {
            id: anchor_id,
            range: new_range,
            node_id: new_node_id,
        };
        doc.anchors.push(anchor);
    }
}

/// Determine anchor ID using completely deterministic rules
fn determine_anchor_id_deterministic(
    new_node_id: Option<usize>,
    position_index: usize,
    old_anchors: &[Anchor],
    old_anchors_by_position: &[Anchor],
    new_range: &std::ops::Range<usize>,
    _semantic_type: &str,
    used_old_ids: &mut std::collections::HashSet<AnchorId>,
) -> AnchorId {
    // Rule 1: If we have a node_id, try to find an old anchor with the same node_id
    if let Some(node_id) = new_node_id {
        for old_anchor in old_anchors {
            if old_anchor.node_id == Some(node_id) && !used_old_ids.contains(&old_anchor.id) {
                used_old_ids.insert(old_anchor.id);
                return old_anchor.id;
            }
        }
    }

    // Rule 2: Use positional mapping for unmatched anchors (when node IDs change)
    // This handles the common case where tree-sitter changes node IDs for unmodified content
    if position_index < old_anchors_by_position.len() {
        let candidate_anchor = &old_anchors_by_position[position_index];
        if !used_old_ids.contains(&candidate_anchor.id) {
            used_old_ids.insert(candidate_anchor.id);
            return candidate_anchor.id;
        }
    }

    // Rule 3: Generate new ID if no good match found
    generate_dynamic_anchor_id(position_index, new_range.clone())
}

/// Collect anchor ranges, node IDs, and semantic type from the tree
fn collect_anchor_ranges_recursive(
    node: tree_sitter::Node,
    ranges: &mut Vec<(std::ops::Range<usize>, Option<usize>, String)>,
) {
    let node_kind = node.kind();
    let should_create_anchor = matches!(
        node_kind,
        "atx_heading" | "list_item" | "fenced_code_block" | "indented_code_block"
    );

    // Don't create anchors for paragraphs - they're too generic and cause issues
    // Only create anchors for explicit structural elements

    if should_create_anchor && !node.byte_range().is_empty() {
        let anchor_range = if node_kind == "list_item" {
            calculate_list_item_own_range(&node)
        } else {
            node.byte_range()
        };

        ranges.push((anchor_range, Some(node.id()), node_kind.to_string()));
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_anchor_ranges_recursive(child, ranges);
    }
}

/// Create anchors from the current tree-sitter parse tree
pub fn create_anchors_from_tree(doc: &mut Document) {
    doc.anchors.clear();

    if let Some(ref tree) = doc.tree {
        let root_node = tree.root_node();
        let mut new_anchors = Vec::new();
        collect_anchors_recursive(root_node, &mut new_anchors);
        doc.anchors = new_anchors;
    }
}

/// Create anchors for any new blocks that don't have anchors yet
pub fn create_anchors_for_new_blocks(doc: &mut Document) {
    if doc.tree.is_none() {
        return;
    }

    let tree = doc.tree.as_ref().unwrap();
    let root_node = tree.root_node();
    let mut new_block_anchors = Vec::new();
    collect_anchors_recursive(root_node, &mut new_block_anchors);

    // Find blocks that don't have anchors yet
    for new_anchor in new_block_anchors {
        // Check if this range already has an anchor - be more sophisticated about overlap detection
        let has_anchor = doc.anchors.iter().any(|existing_anchor| {
            // Check for node ID match (most reliable)
            if existing_anchor.node_id.is_some() && existing_anchor.node_id == new_anchor.node_id {
                return true;
            }

            // Check for overlapping ranges (handles cases where ranges shift slightly)
            existing_anchor.range.start < new_anchor.range.end
                && new_anchor.range.start < existing_anchor.range.end
        });

        if !has_anchor {
            // Add this new anchor
            doc.anchors.push(new_anchor);
        }
    }
}

/// Recursively collect anchors for block-level nodes in the tree  
fn collect_anchors_recursive(node: tree_sitter::Node, anchors: &mut Vec<Anchor>) {
    let node_kind = node.kind();
    let should_create_anchor = matches!(
        node_kind,
        "atx_heading" | "list_item" | "fenced_code_block" | "indented_code_block"
    );

    // Don't create anchors for paragraphs - they're too generic and cause issues
    // Only create anchors for explicit structural elements

    if should_create_anchor && !node.byte_range().is_empty() {
        // CRITICAL FIX: For list_item nodes, only use the range of the first line
        // to avoid overlapping with child list items
        let anchor_range = if node_kind == "list_item" {
            calculate_list_item_own_range(&node)
        } else {
            node.byte_range()
        };

        let anchor_id = generate_static_anchor_id(anchors.len(), anchor_range.clone());
        let node_id = node.id();

        let anchor = Anchor {
            id: anchor_id,
            range: anchor_range,
            node_id: Some(node_id),
        };

        anchors.push(anchor);
    }

    // Recursively process child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_anchors_recursive(child, anchors);
    }
}

/// Calculate the range for just the list item's own content (excluding children)
fn calculate_list_item_own_range(node: &tree_sitter::Node) -> std::ops::Range<usize> {
    let full_range = node.byte_range();

    // For a list_item, find the first child list (if any) and stop there
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "list" {
            // The list item's own content ends where the child list begins
            return full_range.start..child.byte_range().start;
        }
    }

    // No child list found - but still need to be careful about newlines
    // Look for the end of the first line
    // This is a simplified version - ideally we'd parse the actual text
    // but for now, assume the list item content is just the first part
    full_range
}

/// Generate a static anchor ID for initial tree creation
fn generate_static_anchor_id(index: usize, byte_range: std::ops::Range<usize>) -> AnchorId {
    let mut hasher = DefaultHasher::new();

    // Include a magic number to differentiate from dynamic IDs
    let magic = 0x1234567890abcdefu64;
    magic.hash(&mut hasher);

    // Include index to ensure uniqueness within this generation
    index.hash(&mut hasher);

    // Include byte range to ensure uniqueness across different content
    byte_range.start.hash(&mut hasher);
    byte_range.end.hash(&mut hasher);

    AnchorId(hasher.finish() as u128)
}

/// Generate a dynamic anchor ID for new blocks created during editing
fn generate_dynamic_anchor_id(index: usize, byte_range: std::ops::Range<usize>) -> AnchorId {
    let mut hasher = DefaultHasher::new();

    // Include a different magic number to differentiate from static IDs
    let magic = 0xfedcba0987654321u64;
    magic.hash(&mut hasher);

    // Include index and range for uniqueness
    index.hash(&mut hasher);
    byte_range.start.hash(&mut hasher);
    byte_range.end.hash(&mut hasher);

    AnchorId(hasher.finish() as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{Document, commands::Cmd};

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

        // Debug: print original anchors
        eprintln!("Original anchors:");
        for (i, anchor) in original_anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone());
            eprintln!(
                "  [{}] id={:?} range={:?} content={:?}",
                i, anchor.id, anchor.range, content
            );
        }

        // Insert text at the beginning
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "Prefix: ".to_string(),
        });

        // Debug: print anchors after transformation
        eprintln!("After transformation:");
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone());
            eprintln!(
                "  [{}] id={:?} range={:?} content={:?}",
                i, anchor.id, anchor.range, content
            );
        }

        // Check correct anchor preservation behavior:
        // When inserting at the beginning, all existing anchors should be shifted by the insert length
        let insert_len = "Prefix: ".len();

        assert_eq!(
            doc.anchors.len(),
            original_anchors.len(),
            "Should preserve all original anchors"
        );

        // All anchors should be shifted by the insertion length
        for (i, original) in original_anchors.iter().enumerate() {
            let current = &doc.anchors[i];
            assert_eq!(current.id, original.id, "Anchor ID should be preserved");
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

        let _original_anchor_count = doc.anchors.len();

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
                "Anchor {i} should have valid range"
            );
            assert!(
                anchor.range.end <= doc.text().len(),
                "Anchor {i} should be within document bounds"
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
    fn test_anchor_ids_remain_stable_after_edits() {
        // FIRST: Create a document with some initial content
        let text = "# Heading One\n\nA paragraph here.\n\n- List item 1\n- List item 2";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        // Create anchors from the initial tree (this should be done ONCE)
        doc.create_anchors_from_tree();

        // Store the original anchor IDs and their content for verification
        let original_anchors: Vec<(AnchorId, String)> = doc
            .anchors
            .iter()
            .map(|a| {
                let content = doc.slice_to_cow(a.range.clone()).to_string();
                (a.id, content)
            })
            .collect();

        assert!(!original_anchors.is_empty(), "Should have initial anchors");

        // SECOND: Apply an edit that inserts text at the beginning
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "PREPENDED: ".to_string(),
        });

        // THIRD: Verify the original anchor IDs still exist
        for (original_id, original_content) in &original_anchors {
            let anchor = doc
                .anchors
                .iter()
                .find(|a| a.id == *original_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Anchor with ID {:?} should still exist after insertion",
                        original_id
                    )
                });

            // The content should still be findable (though at a different position)
            let current_content = doc.slice_to_cow(anchor.range.clone()).to_string();
            assert_eq!(
                current_content.trim(),
                original_content.trim(),
                "Anchor content should remain the same, just shifted"
            );
        }

        // FOURTH: Apply a deletion that removes part of the prepended text
        doc.apply(Cmd::DeleteRange { range: 0..5 }); // Remove "PREPE"

        // FIFTH: Verify the original anchor IDs STILL exist
        for (original_id, _) in &original_anchors {
            assert!(
                doc.anchors.iter().any(|a| a.id == *original_id),
                "Anchor with ID {:?} should still exist after deletion",
                original_id
            );
        }

        // SIXTH: Insert text in the middle of the document
        let middle_pos = doc.text().len() / 2;
        doc.apply(Cmd::InsertText {
            at: middle_pos,
            text: "\n\n## New Section\n\nMore content here.\n\n".to_string(),
        });

        // SEVENTH: Verify original anchors STILL have stable IDs
        for (original_id, _) in &original_anchors {
            assert!(
                doc.anchors.iter().any(|a| a.id == *original_id),
                "Original anchor with ID {:?} should still exist after middle insertion",
                original_id
            );
        }

        // New anchors may have been created for the new content, but that's OK
        // The key requirement is that EXISTING anchor IDs remain stable
    }

    #[test]
    fn test_anchor_deletion_overlapping_ranges() {
        // Test that anchor IDs remain stable even when deletions overlap their ranges
        let text = "# First Heading\n\nParagraph content.\n\n## Second Heading\n\nMore content.";
        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        doc.create_anchors_from_tree();

        let original_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();

        assert!(!original_ids.is_empty(), "Should have initial anchors");

        // Delete a range that overlaps with multiple anchors
        // This deletes from middle of first heading to middle of paragraph
        doc.apply(Cmd::DeleteRange { range: 8..25 }); // Delete "Heading\n\nParagraph"

        // Original anchor IDs should still exist (ranges may be adjusted)
        let current_ids: std::collections::HashSet<AnchorId> =
            doc.anchors.iter().map(|a| a.id).collect();

        // Most of the original IDs should still exist
        // Note: Some anchors might be removed if they become invalid/empty
        // but the transformation process should preserve IDs where possible
        for original_id in &original_ids {
            if current_ids.contains(original_id) {
                // If the anchor still exists, verify it has a valid range
                let anchor = doc.anchors.iter().find(|a| a.id == *original_id).unwrap();
                assert!(
                    anchor.range.start <= anchor.range.end,
                    "Anchor should have valid range"
                );
                assert!(
                    anchor.range.end <= doc.text().len(),
                    "Anchor should be within document"
                );
            }
        }
    }

    #[test]
    fn test_anchors_created_for_new_blocks() {
        // Test that anchors ARE created automatically when new blocks appear
        let mut doc = Document::from_bytes(b"").unwrap();
        assert_eq!(
            doc.anchors.len(),
            0,
            "Empty document should have no anchors"
        );

        // Apply an edit that creates a block
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "- Item 1".to_string(),
        });

        // Anchors should be created for the new block
        // This ensures blocks have stable IDs even when created dynamically
        assert!(
            !doc.anchors.is_empty(),
            "Apply should create anchors for new blocks"
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

    #[test]
    fn test_anchor_creation_for_empty_document_after_edit() {
        // Test the bug fix: anchors should be created for new blocks even if document started empty

        // Start with an empty document (no anchors initially)
        let mut doc = Document::from_bytes(b"").unwrap();
        assert_eq!(
            doc.anchors.len(),
            0,
            "Empty document should have no anchors"
        );

        // Initialize anchors (this should work even for empty documents)
        doc.create_anchors_from_tree();
        assert_eq!(
            doc.anchors.len(),
            0,
            "Empty document should still have no anchors after initialization"
        );

        // Insert text that creates a block (should get an anchor)
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "- first item".to_string(),
        });

        // The document now has content and should have created an anchor for the new block
        assert!(
            !doc.anchors.is_empty(),
            "After inserting a list item into initialized empty document, should have anchors created"
        );

        // Verify the anchor covers the new content
        let anchor = &doc.anchors[0];
        let content = doc.slice_to_cow(anchor.range.clone()).to_string();
        assert!(
            content.trim().contains("first item"),
            "Anchor should cover the new list item content"
        );
    }

    #[test]
    fn test_anchor_creation_after_deleting_all_content() {
        // Test edge case: document with anchors becomes empty, then gets content again

        // Start with a document that has content
        let mut doc = Document::from_bytes(b"# Heading\n\n- Item").unwrap();
        doc.create_anchors_from_tree();

        let initial_anchor_count = doc.anchors.len();
        assert!(initial_anchor_count > 0, "Should have initial anchors");

        // Delete all content
        doc.apply(Cmd::DeleteRange {
            range: 0..doc.text().len(),
        });

        // Should have no anchors now (they get cleaned up)
        assert_eq!(
            doc.anchors.len(),
            0,
            "Should have no anchors after deleting all content"
        );

        // Add new content - should get new anchors because anchors were initialized
        doc.apply(Cmd::InsertText {
            at: 0,
            text: "## New heading\n\n- New item".to_string(),
        });

        // Should have created anchors for the new content
        assert!(
            !doc.anchors.is_empty(),
            "Should create anchors for new content even after all content was deleted"
        );

        // Verify anchors cover the new content
        let text = doc.text();
        assert!(text.contains("## New heading"));
        assert!(text.contains("- New item"));

        for anchor in &doc.anchors {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            assert!(
                content.contains("New heading") || content.contains("New item"),
                "Anchor should cover some part of the new content"
            );
        }
    }

    #[test]
    fn test_anchor_generation_with_complex_list_items_no_overlaps() {
        // CRITICAL TEST: Anchors must never overlap, even with complex list content
        // This test covers multiline list items, nested lists, and ensures proper anchor coverage
        let text = r#"- Simple item
- Multi-line item that has
  a hard wrap in the middle
  - Nested item under multiline parent
  - Another nested item
- Another simple item
- Final item with
  multiple lines and
  even more content
  - Deep nested item"#;

        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        // Debug: print all anchors for inspection
        println!("Document text:\n{}", doc.text());
        println!("Generated anchors:");
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            println!("  Anchor {}: {:?} -> {:?}", i, anchor.range, content);
        }

        // REQUIREMENT 1: No two anchors should ever overlap
        for (i, anchor_a) in doc.anchors.iter().enumerate() {
            for (j, anchor_b) in doc.anchors.iter().enumerate() {
                if i != j {
                    let overlap = calculate_range_overlap(&anchor_a.range, &anchor_b.range);
                    assert_eq!(
                        overlap, 0,
                        "Anchors must never overlap! Anchor {} {:?} overlaps with anchor {} {:?} by {} bytes",
                        i, anchor_a.range, j, anchor_b.range, overlap
                    );
                }
            }
        }

        // REQUIREMENT 2: Each list item should get exactly one anchor covering only its own content
        // We should have anchors for:
        // - Simple item
        // - Multi-line item that has (but NOT its nested children)
        // - Nested item under multiline parent
        // - Another nested item
        // - Another simple item
        // - Final item with (but NOT its nested children)
        // - Deep nested item
        assert!(
            doc.anchors.len() >= 7,
            "Should have at least 7 anchors for the distinct list items, got {}",
            doc.anchors.len()
        );

        // REQUIREMENT 3: Each anchor should contain content from only one logical block
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            let content_lines: Vec<&str> = content.lines().collect();

            // For list items, the anchor should not contain nested list markers
            if content.trim_start().starts_with('-') {
                // Count how many list markers are in this anchor's content
                let list_marker_count = content_lines
                    .iter()
                    .filter(|line| line.trim_start().starts_with('-'))
                    .count();

                assert_eq!(
                    list_marker_count, 1,
                    "Anchor {} should contain exactly one list marker (its own), but found {} in content: {:?}",
                    i, list_marker_count, content
                );
            }
        }

        // REQUIREMENT 4: Multiline list items should have anchors covering all their lines
        // (but stopping before any nested content)
        let multiline_anchors: Vec<_> = doc
            .anchors
            .iter()
            .filter(|anchor| {
                let content = doc.slice_to_cow(anchor.range.clone()).to_string();
                content.contains("Multi-line item") || content.contains("Final item with")
            })
            .collect();

        assert!(
            !multiline_anchors.is_empty(),
            "Should have anchors for multiline list items"
        );

        for anchor in multiline_anchors {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            if content.contains("Multi-line item") {
                // Should contain the multiline content but not nested items
                assert!(content.contains("a hard wrap in the middle"));
                assert!(!content.contains("Nested item under"));
            }
            if content.contains("Final item with") {
                // Should contain the multiline content but not nested items
                assert!(content.contains("multiple lines"));
                assert!(content.contains("even more content"));
                assert!(!content.contains("Deep nested item"));
            }
        }

        // REQUIREMENT 5: All anchors should be within document bounds and non-empty
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
    fn test_anchor_generation_raw_node_ranges_show_overlap_problem() {
        // This test demonstrates what would happen if we used raw node.byte_range()
        // It shows the fundamental flaw we're trying to fix
        let text = r#"- Parent item with content
  - Child item
- Another parent"#;

        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        // Parse the document to get tree-sitter nodes
        if let Some(ref tree) = doc.tree {
            let root_node = tree.root_node();
            let mut raw_ranges = Vec::new();

            // Collect what the raw node ranges would be for list_item nodes
            collect_raw_list_item_ranges(root_node, &mut raw_ranges);

            println!("Raw list_item node ranges (demonstrating the overlap problem):");
            for (i, range) in raw_ranges.iter().enumerate() {
                let content = doc.slice_to_cow(range.clone()).to_string();
                println!("  Raw range {}: {:?} -> {:?}", i, range, content);
            }

            // Show that raw ranges would overlap
            if raw_ranges.len() >= 2 {
                let parent_range = &raw_ranges[0]; // "- Parent item with content\n  - Child item\n"
                let child_range = &raw_ranges[1]; // "- Child item\n"

                let overlap = calculate_range_overlap(parent_range, child_range);
                println!("Overlap between parent and child: {} bytes", overlap);

                // This would be the problem we're solving - raw ranges DO overlap
                assert!(
                    overlap > 0,
                    "Raw node ranges should overlap (this demonstrates the problem we're fixing)"
                );
            }
        }

        // Now test that our actual anchor generation fixes this
        doc.create_anchors_from_tree();

        println!("Fixed anchor ranges (non-overlapping):");
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            println!("  Anchor {}: {:?} -> {:?}", i, anchor.range, content);
        }

        // Verify our fix produces non-overlapping anchors
        for (i, anchor_a) in doc.anchors.iter().enumerate() {
            for (j, anchor_b) in doc.anchors.iter().enumerate() {
                if i != j {
                    let overlap = calculate_range_overlap(&anchor_a.range, &anchor_b.range);
                    assert_eq!(
                        overlap, 0,
                        "Fixed anchors must not overlap! Anchor {} {:?} overlaps with anchor {} {:?}",
                        i, anchor_a.range, j, anchor_b.range
                    );
                }
            }
        }
    }

    #[test]
    fn test_anchor_generation_edge_cases() {
        // Test edge cases for list item anchor generation
        let text = r#"- Simple item
- Item with inline `code` and **bold** text
  More content on second line
  
  Even a paragraph break
  - Nested after paragraph
- Item ending with spaces   
- Item with trailing newlines

  - Nested after blank line"#;

        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        println!("Edge cases anchor ranges:");
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            println!(
                "  Anchor {}: {:?} -> {:?}",
                i,
                anchor.range,
                content.replace('\n', "\\n")
            );
        }

        // Verify no overlaps
        for (i, anchor_a) in doc.anchors.iter().enumerate() {
            for (j, anchor_b) in doc.anchors.iter().enumerate() {
                if i != j {
                    let overlap = calculate_range_overlap(&anchor_a.range, &anchor_b.range);
                    assert_eq!(
                        overlap, 0,
                        "Anchors must not overlap! Anchor {} {:?} overlaps with anchor {} {:?}",
                        i, anchor_a.range, j, anchor_b.range
                    );
                }
            }
        }

        // Verify that multiline items include all their content up to nested items
        let multiline_anchors: Vec<_> = doc
            .anchors
            .iter()
            .filter(|anchor| {
                let content = doc.slice_to_cow(anchor.range.clone()).to_string();
                content.contains("inline `code`") || content.contains("trailing newlines")
            })
            .collect();

        for anchor in multiline_anchors {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            if content.contains("inline `code`") {
                // Should include formatted text and multiple lines but not nested items
                assert!(content.contains("**bold** text"));
                assert!(content.contains("More content on second line"));
                assert!(content.contains("Even a paragraph break"));
                assert!(!content.contains("Nested after paragraph"));
            }
        }
    }

    #[test]
    fn test_list_item_own_range_calculation_robustness() {
        // Test the robustness of calculate_list_item_own_range with tricky cases
        let text = r#"- Item without nested content
- Item with nested content
  - Child 1
  - Child 2
- Item with multiple nested lists
  - First nested list
    - Deep nest
  - Second item in first list
  
  - Second nested list after gap
    - Another deep nest
- Final simple item"#;

        let mut doc = Document::from_bytes(text.as_bytes()).unwrap();

        // Test the raw ranges to see the problem space
        if let Some(ref tree) = doc.tree {
            let root_node = tree.root_node();
            let mut raw_ranges = Vec::new();
            collect_raw_list_item_ranges(root_node, &mut raw_ranges);

            println!("Raw list_item ranges (showing overlap problem):");
            for (i, range) in raw_ranges.iter().enumerate() {
                let content = doc.slice_to_cow(range.clone()).to_string();
                println!(
                    "  Raw {}: {:?} -> {:?}",
                    i,
                    range,
                    content.replace('\n', "\\n")
                );
            }

            // Find overlaps in raw ranges
            let mut overlap_found = false;
            for (i, range_a) in raw_ranges.iter().enumerate() {
                for (j, range_b) in raw_ranges.iter().enumerate() {
                    if i != j {
                        let overlap = calculate_range_overlap(range_a, range_b);
                        if overlap > 0 {
                            overlap_found = true;
                            println!(
                                "  Raw overlap: {} and {} overlap by {} bytes",
                                i, j, overlap
                            );
                        }
                    }
                }
            }
            assert!(overlap_found, "Raw ranges should show overlap problem");
        }

        // Now test that our anchors fix the overlaps
        doc.create_anchors_from_tree();

        println!("Fixed anchor ranges:");
        for (i, anchor) in doc.anchors.iter().enumerate() {
            let content = doc.slice_to_cow(anchor.range.clone()).to_string();
            println!(
                "  Anchor {}: {:?} -> {:?}",
                i,
                anchor.range,
                content.replace('\n', "\\n")
            );
        }

        // Verify no overlaps in fixed anchors
        for (i, anchor_a) in doc.anchors.iter().enumerate() {
            for (j, anchor_b) in doc.anchors.iter().enumerate() {
                if i != j {
                    let overlap = calculate_range_overlap(&anchor_a.range, &anchor_b.range);
                    assert_eq!(
                        overlap, 0,
                        "Fixed anchors must not overlap! Anchor {} and {} overlap by {} bytes",
                        i, j, overlap
                    );
                }
            }
        }

        // Verify that parent items stop before their first nested list
        // The second item should include "Item with nested content\n  " but not the children
        let parent_with_nested: Vec<_> = doc
            .anchors
            .iter()
            .filter(|anchor| {
                let content = doc.slice_to_cow(anchor.range.clone()).to_string();
                content.contains("Item with nested content")
            })
            .collect();

        assert_eq!(
            parent_with_nested.len(),
            1,
            "Should find exactly one anchor for 'Item with nested content'"
        );
        let parent_content = doc
            .slice_to_cow(parent_with_nested[0].range.clone())
            .to_string();
        assert!(
            !parent_content.contains("Child 1"),
            "Parent anchor should not contain child content"
        );
        assert!(
            !parent_content.contains("Child 2"),
            "Parent anchor should not contain child content"
        );
    }

    // Helper function for testing - shows what raw node ranges would be
    fn collect_raw_list_item_ranges(
        node: tree_sitter::Node,
        ranges: &mut Vec<std::ops::Range<usize>>,
    ) {
        if node.kind() == "list_item" {
            // This is what would happen if we used raw node.byte_range() - it includes children!
            ranges.push(node.byte_range());
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_raw_list_item_ranges(child, ranges);
        }
    }
}
