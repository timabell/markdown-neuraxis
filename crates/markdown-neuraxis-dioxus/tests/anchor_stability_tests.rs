//! Anchor ID Stability Tests - Check if IDs change during editing cycles

use markdown_neuraxis_engine::editing::snapshot::{Block, BlockContent, InlineNode, InlineSegment};
use markdown_neuraxis_engine::editing::{AnchorId, Document};

/// Flatten hierarchical blocks into a list of (content, id) pairs
fn flatten_blocks(blocks: &[Block], out: &mut Vec<(String, AnchorId)>) {
    for block in blocks {
        // Extract plain text from segments
        let content = segments_to_plain_text(&block.segments);

        out.push((content, block.id));

        if let BlockContent::Children(children) = &block.content {
            flatten_blocks(children, out);
        }
    }
}

/// Extract plain text from segments (test helper)
fn segments_to_plain_text(segments: &[InlineSegment]) -> String {
    segments
        .iter()
        .map(|seg| inline_node_to_text(&seg.kind))
        .collect()
}

/// Recursively extract plain text from an inline node
fn inline_node_to_text(node: &InlineNode) -> String {
    match node {
        InlineNode::Text(s) => s.clone(),
        InlineNode::Strong(children) | InlineNode::Emphasis(children) => {
            children.iter().map(inline_node_to_text).collect()
        }
        InlineNode::Code(s) => s.clone(),
        InlineNode::Strikethrough(s) => s.clone(),
        InlineNode::WikiLink { target, alias } => alias.as_ref().unwrap_or(target).clone(),
        InlineNode::Link { text, .. } => text.clone(),
        InlineNode::Image { alt, .. } => alt.clone(),
        InlineNode::HardBreak => "\n".to_string(),
    }
}

#[cfg(test)]
mod anchor_stability_tests {
    use super::*;

    #[test]
    fn test_anchor_ids_stable_during_focus_cycles() {
        // Create document from the actual test data that shows the bug
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        // Capture initial anchor IDs
        let initial_snapshot = doc.snapshot();
        let mut initial_blocks = Vec::new();
        flatten_blocks(&initial_snapshot.blocks, &mut initial_blocks);

        let initial_anchor_map: std::collections::HashMap<String, u128> = initial_blocks
            .iter()
            .map(|(content, id)| (content.clone(), id.0))
            .collect();

        println!("Initial anchor IDs:");
        for (content, id) in &initial_anchor_map {
            println!("  '{}' -> {}", content, id);
        }

        // Find a nested item to test
        let nested_item = initial_blocks
            .iter()
            .find(|(content, _)| content.contains("indented 1.2"))
            .expect("Should have nested item");

        println!("\nTesting anchor stability for: '{}'", nested_item.0);
        let original_anchor_id = nested_item.1.0;

        // Step 1: Focus (this might trigger re-parsing)
        println!("Step 1: Focusing on item...");
        doc.create_anchors_from_tree();

        let after_focus_snapshot = doc.snapshot();
        let mut after_focus_blocks = Vec::new();
        flatten_blocks(&after_focus_snapshot.blocks, &mut after_focus_blocks);

        let after_focus_id = after_focus_blocks
            .iter()
            .find(|(content, _)| *content == nested_item.0)
            .expect("Item should still exist")
            .1
            .0;

        assert_eq!(
            original_anchor_id, after_focus_id,
            "ANCHOR INSTABILITY BUG: Anchor ID changed during focus! '{}' {} -> {}",
            nested_item.0, original_anchor_id, after_focus_id
        );

        // Step 2: Multiple regenerations without content changes
        println!("Step 2: Multiple anchor regenerations...");

        for cycle in 0..3 {
            doc.create_anchors_from_tree();
            let cycle_snapshot = doc.snapshot();
            let mut cycle_blocks = Vec::new();
            flatten_blocks(&cycle_snapshot.blocks, &mut cycle_blocks);

            let cycle_item = cycle_blocks
                .iter()
                .find(|(content, _)| *content == nested_item.0)
                .expect("Item should still exist");

            assert_eq!(
                original_anchor_id, cycle_item.1.0,
                "ANCHOR INSTABILITY BUG: Anchor ID changed during cycle {}! '{}' {} -> {}",
                cycle, nested_item.0, original_anchor_id, cycle_item.1.0
            );
        }
    }

    #[test]
    fn test_anchor_uniqueness_after_multiple_regenerations() {
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

        for cycle in 0..5 {
            println!("Anchor regeneration cycle {}", cycle);
            doc.create_anchors_from_tree();

            let snapshot = doc.snapshot();
            let mut blocks = Vec::new();
            flatten_blocks(&snapshot.blocks, &mut blocks);

            let anchor_ids: Vec<u128> = blocks.iter().map(|(_, id)| id.0).collect();
            let unique_ids: std::collections::HashSet<u128> = anchor_ids.iter().cloned().collect();

            println!(
                "  Total blocks: {}, Unique IDs: {}",
                anchor_ids.len(),
                unique_ids.len()
            );

            if anchor_ids.len() != unique_ids.len() {
                let mut id_to_content: std::collections::HashMap<u128, String> =
                    std::collections::HashMap::new();
                let mut duplicates = Vec::new();

                for (content, id) in &blocks {
                    if let Some(existing) = id_to_content.get(&id.0) {
                        duplicates.push((id.0, existing.clone(), content.clone()));
                    } else {
                        id_to_content.insert(id.0, content.clone());
                    }
                }

                panic!(
                    "ANCHOR COLLISION BUG FOUND in cycle {}: {} duplicates: {:#?}",
                    cycle,
                    duplicates.len(),
                    duplicates
                );
            }
        }
    }

    #[test]
    fn test_specific_bug_scenario_from_diagnostics() {
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();
        let mut blocks = Vec::new();
        flatten_blocks(&snapshot.blocks, &mut blocks);

        let indented_1_blocks: Vec<_> = blocks
            .iter()
            .filter(|(content, _)| content.starts_with("indented 1") && !content.contains("."))
            .collect();

        let indented_1_2_blocks: Vec<_> = blocks
            .iter()
            .filter(|(content, _)| content.starts_with("indented 1.2"))
            .collect();

        println!("'indented 1' blocks:");
        for (content, id) in &indented_1_blocks {
            println!("  '{}' -> {}", content, id.0);
        }

        println!("'indented 1.2' blocks:");
        for (content, id) in &indented_1_2_blocks {
            println!("  '{}' -> {}", content, id.0);
        }

        // Check for same anchor ID for different content
        for (content1, id1) in &indented_1_blocks {
            for (content2, id2) in &indented_1_2_blocks {
                assert_ne!(
                    id1.0, id2.0,
                    "DIAGNOSTIC BUG REPRODUCED: '{}' and '{}' have same anchor ID: {}",
                    content1, content2, id1.0
                );
            }
        }

        // Check for duplicates within each group
        let mut seen_ids = std::collections::HashSet::new();
        for (content, id) in indented_1_blocks.iter().chain(indented_1_2_blocks.iter()) {
            assert!(
                seen_ids.insert(id.0),
                "DUPLICATE ANCHOR ID: {} for '{}'",
                id.0,
                content
            );
        }
    }
}
