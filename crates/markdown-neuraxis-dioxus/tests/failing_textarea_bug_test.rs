//! Failing test that reproduces the exact multiple textarea bug

use markdown_neuraxis_engine::editing::Document;

#[cfg(test)]
mod failing_bug_reproduction {
    use super::*;

    #[test]
    fn test_prefix_content_collision_bug() {
        // This test SHOULD FAIL initially, reproducing the exact bug from the screenshot
        // The bug: "indented 1" and "indented 1.2" get same anchor ID due to prefix collision

        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Find the exact problematic items from the screenshot
        let indented_1_items: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.content == "indented 1" && b.depth == 0)
            .collect();

        let indented_1_2_items: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.content == "indented 1.2" && b.depth > 0)
            .collect();

        println!("=== REPRODUCING MULTIPLE TEXTAREA BUG ===");
        println!("Found {} 'indented 1' items:", indented_1_items.len());
        for (i, item) in indented_1_items.iter().enumerate() {
            println!(
                "  [{}] '{}' -> anchor_id={} depth={}",
                i, item.content, item.id.0, item.depth
            );
        }

        println!("Found {} 'indented 1.2' items:", indented_1_2_items.len());
        for (i, item) in indented_1_2_items.iter().enumerate() {
            println!(
                "  [{}] '{}' -> anchor_id={} depth={}",
                i, item.content, item.id.0, item.depth
            );
        }

        // Test the bug scenario: when one of these items is clicked,
        // do multiple items think they're focused?

        // Simulate clicking on any "indented 1.2" item
        if let Some(clicked_item) = indented_1_2_items.first() {
            println!(
                "\n=== SIMULATING CLICK ON '{}' (id={}) ===",
                clicked_item.content, clicked_item.id.0
            );

            // This simulates what happens in the UI when focused_anchor_id signal is set
            let focused_anchor_id = clicked_item.id;

            // Check which items would render textareas (think they're focused)
            let mut textarea_items = Vec::new();

            // Check all "indented 1" items
            for item in &indented_1_items {
                if item.id == focused_anchor_id {
                    textarea_items.push(("indented 1", item.id.0, item.depth));
                }
            }

            // Check all "indented 1.2" items
            for item in &indented_1_2_items {
                if item.id == focused_anchor_id {
                    textarea_items.push(("indented 1.2", item.id.0, item.depth));
                }
            }

            println!("Items that would render textareas:");
            for (content, id, depth) in &textarea_items {
                println!("  TEXTAREA: '{}' id={} depth={}", content, id, depth);
            }

            // THE BUG: This should fail because multiple items have the same anchor ID
            assert_eq!(
                textarea_items.len(),
                1,
                "MULTIPLE TEXTAREA BUG REPRODUCED: Expected 1 textarea, got {}. \
                 Clicking '{}' causes {} items to render textareas: {:#?}",
                textarea_items.len(),
                clicked_item.content,
                textarea_items.len(),
                textarea_items
            );

            // Additional check: verify the content matches what we clicked
            if let Some((textarea_content, _, _)) = textarea_items.first() {
                assert_eq!(
                    *textarea_content, clicked_item.content,
                    "WRONG CONTENT BUG: Clicked '{}' but textarea shows '{}'",
                    clicked_item.content, textarea_content
                );
            }
        } else {
            panic!("Test data should contain 'indented 1.2' items");
        }
    }

    #[test]
    fn test_anchor_generation_algorithm_collision() {
        // Test to understand WHY the anchor generation creates collisions
        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

        // Test anchor generation multiple times to see if it's consistent
        let mut anchor_mappings = Vec::new();

        for iteration in 0..3 {
            doc.create_anchors_from_tree();
            let snapshot = doc.snapshot();

            println!("\n=== ANCHOR GENERATION ITERATION {} ===", iteration);

            let mut iteration_map = std::collections::HashMap::new();

            for block in &snapshot.blocks {
                // Look specifically for prefix-related content
                if block.content.starts_with("indented 1") {
                    println!(
                        "  '{}' (depth={}) -> anchor_id={}",
                        block.content, block.depth, block.id.0
                    );
                    iteration_map.insert(block.content.clone(), block.id.0);
                }
            }

            anchor_mappings.push(iteration_map);
        }

        // Check if anchor IDs are stable across iterations
        if anchor_mappings.len() >= 2 {
            let first_mapping = &anchor_mappings[0];
            for (iteration, mapping) in anchor_mappings.iter().enumerate().skip(1) {
                for (content, first_id) in first_mapping {
                    if let Some(current_id) = mapping.get(content) {
                        assert_eq!(
                            first_id, current_id,
                            "ANCHOR INSTABILITY: '{}' had anchor_id {} in iteration 0 but {} in iteration {}",
                            content, first_id, current_id, iteration
                        );
                    }
                }
            }
        }

        // The critical test: look for anchor ID collisions
        if let Some(mapping) = anchor_mappings.first() {
            let mut id_to_content: std::collections::HashMap<u128, String> =
                std::collections::HashMap::new();
            let mut collisions = Vec::new();

            for (content, anchor_id) in mapping {
                if let Some(existing_content) = id_to_content.get(anchor_id) {
                    collisions.push((anchor_id, existing_content.clone(), content.clone()));
                } else {
                    id_to_content.insert(*anchor_id, content.clone());
                }
            }

            if !collisions.is_empty() {
                println!("\n🐛 ANCHOR COLLISION DETECTED:");
                for (id, content1, content2) in &collisions {
                    println!(
                        "  anchor_id {} shared by '{}' and '{}'",
                        id, content1, content2
                    );
                }

                panic!(
                    "ANCHOR GENERATION BUG: {} anchor ID collisions found: {:#?}",
                    collisions.len(),
                    collisions
                );
            } else {
                println!("\n✅ No anchor ID collisions found in this test");
                // This might mean the test data doesn't reproduce the bug,
                // or the bug is more subtle than simple content collision
            }
        }
    }

    #[test]
    fn test_real_world_bug_scenario_from_diagnostic() {
        // Reproduce the exact diagnostic output scenario:
        // CLICK EVENT: 'indented 1.2' id=AnchorId(10032346120884770342)
        // RenderListItem RENDER: 'indented 1' id=AnchorId(10032346120884770342) is_focused=true
        // RenderListItem RENDER: 'indented 1.2' id=AnchorId(10032346120884770342) is_focused=true

        let markdown = include_str!("../test_data/nested_lists_bug_repro.md");
        let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
        doc.create_anchors_from_tree();

        let snapshot = doc.snapshot();

        // Look for the specific collision: same anchor ID for different content
        let target_anchor_id = 10032346120884770342u128;

        let blocks_with_target_id: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|b| b.id.0 == target_anchor_id)
            .collect();

        println!("=== SEARCHING FOR DIAGNOSTIC BUG SCENARIO ===");
        println!("Looking for anchor_id {}", target_anchor_id);
        println!(
            "Found {} blocks with this anchor ID:",
            blocks_with_target_id.len()
        );

        for (i, block) in blocks_with_target_id.iter().enumerate() {
            println!(
                "  [{}] '{}' depth={} id={}",
                i, block.content, block.depth, block.id.0
            );
        }

        if blocks_with_target_id.len() > 1 {
            // Found the exact bug!
            let contents: Vec<&str> = blocks_with_target_id
                .iter()
                .map(|b| b.content.as_str())
                .collect();

            panic!(
                "DIAGNOSTIC BUG REPRODUCED: Anchor ID {} is shared by {} blocks with different content: {:?}",
                target_anchor_id,
                blocks_with_target_id.len(),
                contents
            );
        } else {
            println!(
                "Note: Specific diagnostic anchor ID {} not found in current test data",
                target_anchor_id
            );
            println!(
                "This might mean the bug is environment-dependent or our test data differs from runtime"
            );

            // Still test for any collisions in the current data
            let mut anchor_to_blocks = std::collections::HashMap::new();

            for block in &snapshot.blocks {
                anchor_to_blocks
                    .entry(block.id.0)
                    .or_insert_with(Vec::new)
                    .push(block);
            }

            let collisions: Vec<_> = anchor_to_blocks
                .iter()
                .filter(|(_, blocks)| blocks.len() > 1)
                .collect();

            if !collisions.is_empty() {
                println!("\n🐛 FOUND ANCHOR COLLISIONS IN CURRENT DATA:");
                for (anchor_id, blocks) in &collisions {
                    println!("  anchor_id {}: {} blocks", anchor_id, blocks.len());
                    for (i, block) in blocks.iter().enumerate() {
                        println!("    [{}] '{}'", i, block.content);
                    }
                }

                // Pick the first collision to demonstrate the bug
                let (collision_id, collision_blocks) = collisions[0];
                let collision_contents: Vec<&str> = collision_blocks
                    .iter()
                    .map(|b| b.content.as_str())
                    .collect();

                panic!(
                    "ANCHOR COLLISION BUG FOUND: Anchor ID {} shared by {} blocks: {:?}",
                    collision_id,
                    collision_blocks.len(),
                    collision_contents
                );
            }
        }
    }
}
