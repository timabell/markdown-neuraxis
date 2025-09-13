//! Anchor ID Stability Tests - Check if IDs change during editing cycles

use markdown_neuraxis_engine::editing::Document;

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
        let initial_anchor_map: std::collections::HashMap<String, u128> = initial_snapshot
            .blocks
            .iter()
            .map(|block| (block.content.clone(), block.id.0))
            .collect();

        println!("Initial anchor IDs:");
        for (content, id) in &initial_anchor_map {
            println!("  '{}' -> {}", content, id);
        }

        // Simulate what happens during editing: focus -> edit -> unfocus
        // This might trigger anchor regeneration

        // Find a nested item to test
        let nested_item = initial_snapshot
            .blocks
            .iter()
            .find(|b| b.content.contains("indented 1.2") && b.depth > 0)
            .expect("Should have nested item");

        println!("\nTesting anchor stability for: '{}'", nested_item.content);
        let original_anchor_id = nested_item.id.0;

        // Simulate editing cycle that might cause anchor ID changes
        // (This is what happens in real usage when you click -> edit -> click elsewhere)

        // Step 1: Focus (this might trigger re-parsing)
        println!("Step 1: Focusing on item...");
        doc.create_anchors_from_tree(); // Re-create anchors (as might happen during focus)

        let after_focus_snapshot = doc.snapshot();
        let after_focus_id = after_focus_snapshot
            .blocks
            .iter()
            .find(|b| b.content == nested_item.content)
            .expect("Item should still exist")
            .id
            .0;

        assert_eq!(
            original_anchor_id, after_focus_id,
            "ANCHOR INSTABILITY BUG: Anchor ID changed during focus! '{}' {} -> {}",
            nested_item.content, original_anchor_id, after_focus_id
        );

        // Step 2: Multiple regenerations without content changes (this should be stable)
        println!("Step 2: Multiple anchor regenerations...");

        for cycle in 0..3 {
            doc.create_anchors_from_tree(); // Re-create anchors multiple times
            let cycle_snapshot = doc.snapshot();
            let cycle_item = cycle_snapshot
                .blocks
                .iter()
                .find(|b| b.content == nested_item.content)
                .expect("Item should still exist");

            assert_eq!(
                original_anchor_id, cycle_item.id.0,
                "ANCHOR INSTABILITY BUG: Anchor ID changed during cycle {} without content change! '{}' {} -> {}",
                cycle, nested_item.content, original_anchor_id, cycle_item.id.0
            );
        }
    }

    #[test]
    fn test_anchor_uniqueness_after_multiple_regenerations() {
        // Test that anchor regeneration doesn't create collisions
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

        // Simulate multiple anchor regenerations (as happens during UI interactions)
        for cycle in 0..5 {
            println!("Anchor regeneration cycle {}", cycle);
            doc.create_anchors_from_tree();

            let snapshot = doc.snapshot();
            let anchor_ids: Vec<u128> = snapshot.blocks.iter().map(|b| b.id.0).collect();
            let unique_ids: std::collections::HashSet<u128> = anchor_ids.iter().cloned().collect();

            println!(
                "  Total blocks: {}, Unique IDs: {}",
                anchor_ids.len(),
                unique_ids.len()
            );

            if anchor_ids.len() != unique_ids.len() {
                // Found the bug! Let's identify the duplicates
                let mut id_to_content: std::collections::HashMap<u128, String> =
                    std::collections::HashMap::new();
                let mut duplicates = Vec::new();

                for block in &snapshot.blocks {
                    if let Some(existing_content) = id_to_content.get(&block.id.0) {
                        duplicates.push((
                            block.id.0,
                            existing_content.clone(),
                            block.content.clone(),
                        ));
                    } else {
                        id_to_content.insert(block.id.0, block.content.clone());
                    }
                }

                panic!(
                    "ANCHOR COLLISION BUG FOUND in cycle {}: {} duplicates found: {:#?}",
                    cycle,
                    duplicates.len(),
                    duplicates
                );
            }
        }
    }

    #[test]
    fn test_specific_bug_scenario_from_diagnostics() {
        // Recreate the exact scenario from the diagnostic output
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Look for the specific case from diagnostics:
        // 'indented 1' and 'indented 1.2' having the same anchor ID
        let indented_1_blocks: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.content.starts_with("indented 1") && !b.content.contains("."))
            .collect();

        let indented_1_2_blocks: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.content.starts_with("indented 1.2"))
            .collect();

        println!("'indented 1' blocks:");
        for block in &indented_1_blocks {
            println!(
                "  '{}' -> {} (depth: {})",
                block.content, block.id.0, block.depth
            );
        }

        println!("'indented 1.2' blocks:");
        for block in &indented_1_2_blocks {
            println!(
                "  '{}' -> {} (depth: {})",
                block.content, block.id.0, block.depth
            );
        }

        // Check for the specific bug: same anchor ID for different content
        for block1 in &indented_1_blocks {
            for block2 in &indented_1_2_blocks {
                assert_ne!(
                    block1.id.0, block2.id.0,
                    "DIAGNOSTIC BUG REPRODUCED: '{}' and '{}' have same anchor ID: {}",
                    block1.content, block2.content, block1.id.0
                );
            }
        }

        // Also check for duplicates within each group
        let mut seen_ids = std::collections::HashSet::new();
        for block in indented_1_blocks.iter().chain(indented_1_2_blocks.iter()) {
            assert!(
                seen_ids.insert(block.id.0),
                "DUPLICATE ANCHOR ID FOUND: ID {} appears multiple times. Block: '{}'",
                block.id.0,
                block.content
            );
        }
    }
}
