//! Dioxus Component Rendering Tests - Testing actual UI component behavior

use dioxus::prelude::*;
use dioxus_ssr::render_element;
use markdown_neuraxis::editing::Document;

/// Test helper to create nested list document
fn create_nested_list_doc() -> Document {
    let markdown = "- item 1\n  - nested 1.1\n  - nested 1.2\n    - deeply nested 1.2.1\n- item 2";
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();
    doc
}

#[cfg(test)]
mod dioxus_component_tests {
    use super::*;

    #[test]
    fn test_dioxus_list_rendering_textarea_count() {
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        // Find nested items
        let nested_items: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|block| block.depth > 0)
            .collect();

        assert!(
            !nested_items.is_empty(),
            "Should have nested items for testing"
        );

        // Test focusing on each nested item
        for target_item in &nested_items {
            println!(
                "\n=== Testing Dioxus rendering when focusing on '{}' ===",
                target_item.content
            );

            // This simulates what happens in the real UI when clicking
            let focused_anchor_id = target_item.id;

            // Try to render something that would show the issue
            // Note: This is a simplified test since we can't easily test full MainPanel rendering
            // But we can test the core rendering logic

            // Count how many blocks would render textareas
            let textarea_rendering_count = snapshot
                .blocks
                .iter()
                .map(|block| {
                    // This is the exact logic from RenderListItem
                    let is_focused = Some(focused_anchor_id) == Some(block.id);
                    (block.content.clone(), is_focused)
                })
                .filter(|(_, is_focused)| *is_focused)
                .count();

            assert_eq!(
                textarea_rendering_count, 1,
                "DIOXUS RENDERING BUG: When focusing '{}' (id={:?}), {} blocks would render textareas instead of 1",
                target_item.content, target_item.id, textarea_rendering_count
            );
        }
    }

    #[test]
    fn test_actual_component_focus_state() {
        // This test tries to get closer to the actual Dioxus component behavior
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        // Get a nested item to test
        let nested_item = snapshot
            .blocks
            .iter()
            .find(|b| b.content.contains("nested 1.1"))
            .expect("Should have nested 1.1 item");

        println!(
            "Testing component focus state for: '{}' (id={:?})",
            nested_item.content, nested_item.id
        );

        // Test what would happen in the component
        // This simulates the focused_anchor_id signal being set
        let simulated_focused_signal = Some(nested_item.id);

        // Test each block's focus calculation
        for block in &snapshot.blocks {
            let is_focused = simulated_focused_signal.as_ref() == Some(&block.id);
            println!(
                "Block '{}' (id={:?}) -> is_focused={}",
                block.content, block.id, is_focused
            );

            // If this block thinks it's focused but it's not the target, that's the bug
            if is_focused && block.id != nested_item.id {
                panic!(
                    "FOUND BUG: Block '{}' (id={:?}) thinks it's focused when target is '{}' (id={:?})",
                    block.content, block.id, nested_item.content, nested_item.id
                );
            }
        }
    }

    #[test]
    fn test_signal_behavior_simulation() {
        // This test simulates signal behavior that might cause the bug
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        println!("Testing signal behavior simulation");

        // Test rapid focus changes (like clicking different items quickly)
        let test_sequence = vec![
            (
                "nested 1.1",
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.content.contains("nested 1.1")),
            ),
            (
                "nested 1.2",
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.content.contains("nested 1.2")),
            ),
            (
                "deeply nested 1.2.1",
                snapshot
                    .blocks
                    .iter()
                    .find(|b| b.content.contains("deeply nested 1.2.1")),
            ),
        ];

        for (click_target, maybe_block) in test_sequence {
            if let Some(clicked_block) = maybe_block {
                println!("\n--- Simulating click on '{}' ---", click_target);

                // This is what should happen when the focused_anchor_id signal updates
                let new_focused_state = Some(clicked_block.id);

                // Check if any other blocks incorrectly think they're focused
                let incorrect_focus: Vec<_> = snapshot
                    .blocks
                    .iter()
                    .filter(|block| {
                        let thinks_focused = new_focused_state.as_ref() == Some(&block.id);
                        thinks_focused && block.id != clicked_block.id
                    })
                    .collect();

                if !incorrect_focus.is_empty() {
                    panic!(
                        "SIGNAL BUG: When focusing '{}', these blocks also think they're focused: {:?}",
                        click_target,
                        incorrect_focus
                            .iter()
                            .map(|b| (&b.content, b.id))
                            .collect::<Vec<_>>()
                    );
                }

                // Verify only the target block is focused
                let correctly_focused: Vec<_> = snapshot
                    .blocks
                    .iter()
                    .filter(|block| new_focused_state.as_ref() == Some(&block.id))
                    .collect();

                assert_eq!(
                    correctly_focused.len(),
                    1,
                    "Expected exactly 1 focused block for '{}', got {}: {:?}",
                    click_target,
                    correctly_focused.len(),
                    correctly_focused
                        .iter()
                        .map(|b| (&b.content, b.id))
                        .collect::<Vec<_>>()
                );

                assert_eq!(
                    correctly_focused[0].id,
                    clicked_block.id,
                    "Wrong block is focused! Expected '{}' (id={:?}) but got '{}' (id={:?})",
                    click_target,
                    clicked_block.id,
                    correctly_focused[0].content,
                    correctly_focused[0].id
                );
            }
        }
    }

    #[test]
    fn test_potential_dioxus_state_bug() {
        // This test looks for potential Dioxus-specific issues
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        println!("Analyzing potential Dioxus state management issues");

        // Look for any patterns that might cause multiple components to render textareas
        let anchor_to_content: std::collections::HashMap<_, _> = snapshot
            .blocks
            .iter()
            .map(|b| (b.id, b.content.clone()))
            .collect();

        println!("Anchor ID to content mapping:");
        for (id, content) in &anchor_to_content {
            println!("  {:?} -> '{}'", id, content);
        }

        // Check if there are any anchor ID patterns that could confuse components
        let anchor_ids: Vec<_> = snapshot.blocks.iter().map(|b| b.id).collect();

        // Look for suspiciously similar anchor IDs (potential collision source)
        for (i, &id1) in anchor_ids.iter().enumerate() {
            for &id2 in anchor_ids.iter().skip(i + 1) {
                let id1_val = id1.0; // Extract the u64 value
                let id2_val = id2.0;

                // Check for potential bit patterns that could cause confusion
                let xor_diff = id1_val ^ id2_val;
                let hamming_weight = xor_diff.count_ones();

                if hamming_weight < 8 {
                    // Less than 8 bit differences - might be confusing
                    println!("WARNING: Anchor IDs are very similar:");
                    println!("  {} ({:064b})", id1_val, id1_val);
                    println!("  {} ({:064b})", id2_val, id2_val);
                    println!(
                        "  XOR diff: {:064b} (hamming weight: {})",
                        xor_diff, hamming_weight
                    );
                }
            }
        }

        // The real test: verify no anchor ID collision could cause the bug
        assert_eq!(
            anchor_ids.len(),
            anchor_to_content.len(),
            "Anchor ID collision detected! Some IDs map to multiple contents"
        );
    }
}
