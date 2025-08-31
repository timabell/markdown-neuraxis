use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use xi_rope::delta::Transformer;
use xi_rope::{Delta, RopeInfo};

use crate::editing::Document;

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

/// Calculate the overlap between two byte ranges
pub(crate) fn calculate_range_overlap(
    range1: &std::ops::Range<usize>,
    range2: &std::ops::Range<usize>,
) -> usize {
    let start = range1.start.max(range2.start);
    let end = range1.end.min(range2.end);
    end.saturating_sub(start)
}

/// Find the anchor ID that best matches the given byte range
pub(crate) fn find_anchor_for_range(doc: &Document, range: &std::ops::Range<usize>) -> AnchorId {
    // Find the anchor that has the best overlap with this range
    let mut best_anchor = None;
    let mut best_overlap = 0;

    for anchor in &doc.anchors {
        let overlap = calculate_range_overlap(range, &anchor.range);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_anchor = Some(anchor.id);
        }
    }

    // If no anchor found, generate a temporary one
    // This shouldn't happen in normal operation since anchors should be created for all block nodes
    best_anchor.unwrap_or_else(|| generate_temp_anchor_id(doc, range))
}

/// Generate a temporary anchor ID for a range (fallback)
fn generate_temp_anchor_id(doc: &Document, range: &std::ops::Range<usize>) -> AnchorId {
    let mut hasher = DefaultHasher::new();

    // Hash the range to create a stable temporary ID
    range.start.hash(&mut hasher);
    range.end.hash(&mut hasher);

    // Include document version to ensure uniqueness
    doc.version.hash(&mut hasher);

    AnchorId(hasher.finish() as u128)
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
pub(crate) fn rebind_anchors_in_changed_regions(
    doc: &mut Document,
    changed: &[std::ops::Range<usize>],
) {
    if changed.is_empty() || doc.tree.is_none() {
        return;
    }

    // Collect anchors that overlap with changed regions
    let mut anchors_to_rebind = Vec::new();
    for (index, anchor) in doc.anchors.iter().enumerate() {
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
        let tree = doc.tree.as_ref().unwrap();
        let root_node = tree.root_node();
        anchors_to_rebind
            .iter()
            .map(|&anchor_index| {
                let new_range = find_best_node_for_anchor(doc, root_node, anchor_index);
                (anchor_index, new_range)
            })
            .collect()
    };

    // Apply the range updates
    for (anchor_index, new_range) in ranges_to_update {
        if let Some(range) = new_range {
            doc.anchors[anchor_index].range = range;
        }
    }

    // Remove anchors that couldn't be rebound properly
    let doc_len = doc.len();
    doc.anchors
        .retain(|anchor| anchor.range.start < anchor.range.end && anchor.range.end <= doc_len);

    // Create anchors for new block-level nodes in changed regions
    if let Some(ref tree) = doc.tree {
        let root_node = tree.root_node();
        let mut new_nodes = Vec::new();
        collect_new_block_nodes_in_regions(root_node, changed, &mut new_nodes);

        for node in new_nodes {
            let node_range = node.byte_range();

            // Check if we already have an anchor for this range
            let has_existing_anchor = doc.anchors.iter().any(|anchor| {
                // Consider ranges that substantially overlap as already covered
                calculate_range_overlap(&anchor.range, &node_range) > node_range.len() / 2
            });

            if !has_existing_anchor {
                let anchor_id = generate_anchor_id(doc);
                let anchor = Anchor {
                    id: anchor_id,
                    range: node_range,
                };
                doc.anchors.push(anchor);
            }
        }
    }
}

/// Find the best node to rebind an anchor to
fn find_best_node_for_anchor(
    doc: &Document,
    root_node: tree_sitter::Node,
    anchor_index: usize,
) -> Option<std::ops::Range<usize>> {
    let anchor = &doc.anchors[anchor_index];
    let mut best_node = None;
    let mut best_overlap = 0;

    // Search for the node that best overlaps with the anchor's current range
    find_best_overlap_recursive(root_node, &anchor.range, &mut best_node, &mut best_overlap);

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
        find_best_overlap_recursive(child, target_range, best_node, best_overlap);
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
        collect_new_block_nodes_in_regions(child, changed, new_nodes);
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

/// Recursively collect anchors for block-level nodes in the tree
fn collect_anchors_recursive(node: tree_sitter::Node, anchors: &mut Vec<Anchor>) {
    // Only create anchors for block-level markdown elements that will appear in render blocks
    let node_kind = node.kind();
    let should_create_anchor = matches!(
        node_kind,
        "atx_heading" | "list_item" | "fenced_code_block" | "indented_code_block"
    );

    // For paragraphs, only create anchors if they are not inside list items
    let should_create_anchor = should_create_anchor
        || (node_kind == "paragraph" && node.parent().map(|p| p.kind()) != Some("list_item"));

    if should_create_anchor && !node.byte_range().is_empty() {
        let anchor_id = generate_static_anchor_id(anchors.len());
        let anchor = Anchor {
            id: anchor_id,
            range: node.byte_range(),
        };
        anchors.push(anchor);
    }

    // Recursively process child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_anchors_recursive(child, anchors);
    }
}

/// Generate a unique anchor ID
fn generate_anchor_id(doc: &Document) -> AnchorId {
    let mut hasher = DefaultHasher::new();

    // Include current time to ensure uniqueness
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    timestamp.hash(&mut hasher);

    // Include current anchor count to avoid collisions within same timestamp
    doc.anchors.len().hash(&mut hasher);

    // Include current document version
    doc.version.hash(&mut hasher);

    AnchorId(hasher.finish() as u128)
}

/// Generate a static anchor ID for initial tree creation
fn generate_static_anchor_id(index: usize) -> AnchorId {
    let mut hasher = DefaultHasher::new();

    // Include a magic number to differentiate from dynamic IDs
    let magic = 0x1234567890abcdefu64;
    magic.hash(&mut hasher);

    // Include index to ensure uniqueness within this generation
    index.hash(&mut hasher);

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
}
