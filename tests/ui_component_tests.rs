//! UI Component TDD Tests - Narrowing down textarea bug location

use dioxus::prelude::*;
use markdown_neuraxis::editing::{AnchorId, Document};
use markdown_neuraxis::ui::components::main_panel::RenderListItem;
use std::collections::HashSet;

/// Test helper to create a nested list document
fn create_nested_list_doc() -> Document {
    let markdown = "- item 1\n  - nested 1.1\n  - nested 1.2\n    - deeply nested 1.2.1\n- item 2";
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();
    doc
}

#[cfg(test)]
mod ui_layer_textarea_bug_tests {
    use super::*;

    #[test]
    fn test_focus_state_calculation_is_exclusive() {
        // Test the core issue: only ONE item should calculate is_focused=true at a time
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        // Get all block anchor IDs
        let anchor_ids: Vec<AnchorId> = snapshot.blocks.iter().map(|b| b.id).collect();
        println!("Available anchor IDs: {:?}", anchor_ids);

        // For each possible focused anchor ID, verify focus exclusivity
        for focused_anchor in &anchor_ids {
            println!("\nTesting focus on anchor: {:?}", focused_anchor);

            // Simulate what each RenderListItem component would calculate
            let focus_results: Vec<(AnchorId, bool)> = anchor_ids
                .iter()
                .map(|&item_anchor| {
                    // This is the exact logic from RenderListItem
                    let is_focused = Some(*focused_anchor) == Some(item_anchor);
                    (item_anchor, is_focused)
                })
                .collect();

            // Count how many items think they're focused
            let focused_count = focus_results.iter().filter(|(_, focused)| *focused).count();

            println!("Focus results: {:?}", focus_results);
            assert_eq!(
                focused_count,
                1,
                "Expected exactly 1 item to be focused when focused_anchor={:?}, but {} items are focused: {:?}",
                focused_anchor,
                focused_count,
                focus_results
                    .iter()
                    .filter(|(_, focused)| *focused)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_multiple_render_items_with_same_focus_signal() {
        // This test simulates the bug: multiple RenderListItem components sharing focus signal
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        // Find nested items (the ones causing the bug)
        let nested_items: Vec<_> = snapshot
            .blocks
            .iter()
            .filter(|block| block.depth > 0)
            .collect();

        assert!(
            !nested_items.is_empty(),
            "Should have nested items for testing"
        );

        // Pick one nested item to focus
        let target_item = nested_items[0];
        println!(
            "Focusing on nested item: content='{}' id={:?} depth={}",
            target_item.content, target_item.id, target_item.depth
        );

        // Simulate what happens when each item checks if it's focused
        let all_focus_checks: Vec<_> = snapshot
            .blocks
            .iter()
            .map(|block| {
                let is_focused = block.id == target_item.id;
                (block.content.clone(), block.id, is_focused)
            })
            .collect();

        println!("Focus check results:");
        for (content, id, focused) in &all_focus_checks {
            println!("  '{}' (id={:?}) -> focused={}", content, id, focused);
        }

        // This should pass - only the target item should be focused
        let focused_items: Vec<_> = all_focus_checks
            .iter()
            .filter(|(_, _, focused)| *focused)
            .collect();

        assert_eq!(
            focused_items.len(),
            1,
            "Expected exactly 1 focused item, got {}: {:?}",
            focused_items.len(),
            focused_items
        );
    }

    #[test]
    fn test_ui_component_focus_signal_behavior() {
        // Test that reproduces the exact bug scenario from UI perspective
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        // This test will help us understand if the issue is:
        // 1. Signal state management (same signal shared incorrectly)
        // 2. Component rendering logic (multiple components rendering textareas)
        // 3. Event handling (clicks setting wrong anchor IDs)

        // Simulate clicking on different nested items
        let click_scenarios = vec![
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

        for (click_target, maybe_block) in click_scenarios {
            if let Some(clicked_block) = maybe_block {
                println!(
                    "\n=== Simulating click on '{}' (id={:?}) ===",
                    click_target, clicked_block.id
                );

                // This simulates the focused_anchor_id signal being set to clicked_block.id
                let simulated_focused_signal = Some(clicked_block.id);

                // Now check what each component would render
                let rendering_results: Vec<_> = snapshot
                    .blocks
                    .iter()
                    .map(|block| {
                        // This is the exact logic from RenderListItem component
                        let is_focused = simulated_focused_signal.as_ref() == Some(&block.id);
                        (
                            block.content.clone(),
                            block.id,
                            is_focused,
                            if is_focused {
                                "RENDERS TEXTAREA"
                            } else {
                                "no textarea"
                            },
                        )
                    })
                    .collect();

                println!("Rendering results:");
                for (content, id, focused, renders) in &rendering_results {
                    println!(
                        "  '{}' (id={:?}) focused={} -> {}",
                        content, id, focused, renders
                    );
                }

                // Count textareas that would be rendered
                let textarea_count = rendering_results
                    .iter()
                    .filter(|(_, _, focused, _)| *focused)
                    .count();

                assert_eq!(
                    textarea_count,
                    1,
                    "MULTIPLE TEXTAREA BUG: Expected 1 textarea when clicking '{}', got {}: {:?}",
                    click_target,
                    textarea_count,
                    rendering_results
                        .iter()
                        .filter(|(_, _, focused, _)| *focused)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn test_anchor_id_collision_hypothesis() {
        // This test specifically checks if there are any anchor ID collisions
        // that could cause the multiple textarea bug
        let doc = create_nested_list_doc();
        let snapshot = doc.snapshot();

        let anchor_ids: Vec<AnchorId> = snapshot.blocks.iter().map(|b| b.id).collect();
        let unique_ids: HashSet<AnchorId> = anchor_ids.iter().cloned().collect();

        println!("Total blocks: {}", anchor_ids.len());
        println!("Unique anchor IDs: {}", unique_ids.len());
        println!("All anchor IDs: {:?}", anchor_ids);

        // This should pass based on our integration tests, but let's verify at UI level
        assert_eq!(
            anchor_ids.len(),
            unique_ids.len(),
            "ANCHOR ID COLLISION DETECTED: Some blocks have duplicate anchor IDs: {:?}",
            anchor_ids
        );

        // Additional check: make sure each content string maps to unique anchor ID
        let mut content_to_id = std::collections::HashMap::new();
        for block in &snapshot.blocks {
            if let Some(existing_id) = content_to_id.get(&block.content) {
                panic!(
                    "DUPLICATE CONTENT MAPPING: Content '{}' maps to multiple anchor IDs: {:?} and {:?}",
                    block.content, existing_id, block.id
                );
            }
            content_to_id.insert(block.content.clone(), block.id);
        }

        println!("Content to anchor ID mapping:");
        for (content, id) in content_to_id {
            println!("  '{}' -> {:?}", content, id);
        }
    }
}
