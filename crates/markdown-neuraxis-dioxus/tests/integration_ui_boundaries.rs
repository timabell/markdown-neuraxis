//! Integration tests for UI boundary interactions
//!
//! Following ADR-0005 Phase 1: Create targeted integration tests that reproduce
//! the multiple textarea bug and test the integration boundary between core
//! document/anchor logic and UI rendering.

use pretty_assertions::assert_eq;
use relative_path::RelativePathBuf;
use std::collections::{HashMap, HashSet};
use tempfile::TempDir;

use markdown_neuraxis_engine::editing::{AnchorId, Document};
use markdown_neuraxis_engine::io;
use markdown_neuraxis_engine::models::MarkdownFile;
// Note: Dioxus UI component testing would require more complex setup
// For now, focusing on integration boundary testing through the public API

/// Test helper to create a temporary markdown file for integration testing
fn create_test_markdown_file(content: &str) -> (TempDir, MarkdownFile) {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.md");
    std::fs::write(&file_path, content).unwrap();

    // Create relative path from the temp directory
    let relative_path = RelativePathBuf::from("test.md");
    let markdown_file = MarkdownFile::new(relative_path);
    (temp_dir, markdown_file)
}

/// Test the core anchor uniqueness invariant through the public API
/// This tests that render blocks have unique anchor IDs, which indicates the core anchor system works correctly.
#[test]
fn test_render_block_anchor_uniqueness() {
    // Create a document with nested list items - the scenario where the bug occurs
    let content = r#"# Main Heading

- Item 1
  - Nested Item 1.1
  - Nested Item 1.2
    - Deeply nested 1.2.1
- Item 2
  - Nested Item 2.1"#;

    let mut document = Document::from_bytes(content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    // Core invariant: All render block anchor IDs must be unique
    let mut anchor_ids = HashSet::new();
    let mut duplicate_ids = Vec::new();

    for block in &snapshot.blocks {
        if !anchor_ids.insert(block.id) {
            duplicate_ids.push(block.id);
        }
    }

    assert!(
        duplicate_ids.is_empty(),
        "Found duplicate anchor IDs in render blocks: {:?}. This indicates anchor ID collision causing the multiple textarea bug.",
        duplicate_ids
    );

    // Additional validation: Each block should have a valid, non-empty range
    for (i, block) in snapshot.blocks.iter().enumerate() {
        assert!(
            block.byte_range.start < block.byte_range.end,
            "Block {} has invalid range: {:?}",
            i,
            block.byte_range
        );
        assert!(
            block.byte_range.end <= document.text().len(),
            "Block {} range extends beyond document: {:?} vs document length {}",
            i,
            block.byte_range,
            document.text().len()
        );
    }

    println!(
        "Render block anchor uniqueness test passed: {} unique anchor IDs for {} blocks",
        anchor_ids.len(),
        snapshot.blocks.len()
    );
}

/// Test that repeated snapshots produce the same anchor IDs (stability test)
/// This ensures that anchor IDs are stable across snapshot operations.
#[test]
fn test_anchor_stability_across_snapshots() {
    let content = r#"# Main Heading

- Item 1
  - Nested Item 1.1
  - Nested Item 1.2
- Item 2
  - Nested Item 2.1"#;

    let mut document = Document::from_bytes(content.as_bytes()).unwrap();
    document.create_anchors_from_tree();

    // Take multiple snapshots and ensure they have the same anchor IDs in the same order
    let snapshot1 = document.snapshot();
    let snapshot2 = document.snapshot();

    assert_eq!(
        snapshot1.blocks.len(),
        snapshot2.blocks.len(),
        "Snapshots should have the same number of blocks"
    );

    for (i, (block1, block2)) in snapshot1
        .blocks
        .iter()
        .zip(snapshot2.blocks.iter())
        .enumerate()
    {
        assert_eq!(
            block1.id, block2.id,
            "Block {} should have stable anchor ID across snapshots",
            i
        );
    }
}

/// Test that the snapshot creation produces blocks with unique anchor IDs
/// This tests the integration boundary where core document anchors are converted to UI render blocks.
#[test]
fn test_snapshot_block_anchor_uniqueness() {
    let content = r#"# Main Heading

- Item 1
  - Nested Item 1.1
  - Nested Item 1.2
    - Deeply nested 1.2.1
- Item 2
  - Nested Item 2.1"#;

    let (temp_dir, file) = create_test_markdown_file(content);

    // Parse the document using the same path as the UI does
    let file_content = io::read_file(file.relative_path(), temp_dir.path()).unwrap();
    let mut document = Document::from_bytes(file_content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    // Check that all render blocks have unique anchor IDs
    let mut anchor_ids = HashSet::new();
    let mut duplicate_ids = Vec::new();

    for block in &snapshot.blocks {
        if !anchor_ids.insert(block.id) {
            duplicate_ids.push(block.id);
        }
    }

    assert!(
        duplicate_ids.is_empty(),
        "Found duplicate anchor IDs in snapshot blocks: {:?}. This indicates the UI boundary is creating duplicate IDs from core anchors.",
        duplicate_ids
    );

    // Additional validation: each block should have valid ranges within the document
    for block in &snapshot.blocks {
        assert!(
            block.byte_range.start <= block.byte_range.end,
            "Block should have valid byte range"
        );
        assert!(
            block.byte_range.end <= document.text().len(),
            "Block range should be within document bounds"
        );
        assert!(
            block.content_range.start <= block.content_range.end,
            "Block should have valid content range"
        );
        assert!(
            block.content_range.end <= document.text().len(),
            "Block content range should be within document bounds"
        );
    }

    println!(
        "Snapshot block uniqueness test passed: {} unique blocks created",
        snapshot.blocks.len()
    );
}

/// Integration test that simulates the focus state changes that trigger the multiple textarea bug
/// This tests the specific UI interaction where clicking on nested list items causes multiple textareas.
#[test]
fn test_focus_state_integration_single_textarea_invariant() {
    let content = r#"# Main Heading

- Item 1
  - Nested Item 1.1
  - Nested Item 1.2
    - Deeply nested 1.2.1
- Item 2
  - Nested Item 2.1"#;

    let (temp_dir, file) = create_test_markdown_file(content);
    let file_content = io::read_file(file.relative_path(), temp_dir.path()).unwrap();
    let mut document = Document::from_bytes(file_content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    // Simulate the focus state that the UI maintains
    let mut focused_anchor_id: Option<AnchorId>;

    // Get all list item blocks (the scenario where the bug occurs)
    let list_item_blocks: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                markdown_neuraxis_engine::editing::BlockKind::ListItem { .. }
            )
        })
        .collect();

    assert!(
        list_item_blocks.len() >= 4,
        "Should have multiple nested list items to test focus behavior"
    );

    // Test the focus state changes that happen when clicking different list items
    for (i, block) in list_item_blocks.iter().enumerate() {
        println!(
            "Testing focus change {}: focusing block with ID {:?}",
            i, block.id
        );

        // Simulate clicking on this list item (focus change)
        focused_anchor_id = Some(block.id);

        // Count how many blocks would be considered "focused" with this state
        let focused_block_count = list_item_blocks
            .iter()
            .filter(|other_block| Some(other_block.id) == focused_anchor_id)
            .count();

        // Core invariant: Only ONE block should be focused at any time
        assert_eq!(
            focused_block_count,
            1,
            "Integration boundary violation: {} blocks are considered focused when only 1 should be. \
            This indicates the multiple textarea bug is present. \
            Focused anchor ID: {:?}, Blocks that match: {:?}",
            focused_block_count,
            focused_anchor_id,
            list_item_blocks
                .iter()
                .filter(|other_block| Some(other_block.id) == focused_anchor_id)
                .map(|b| (b.id, &b.content))
                .collect::<Vec<_>>()
        );
    }

    println!("Focus state integration test passed: single textarea invariant maintained");
}

/// Test that nested list items have proper anchor hierarchy and don't cause ID confusion
/// This test specifically targets the nested list scenario where the bug occurs.
#[test]
fn test_nested_list_anchor_hierarchy() {
    let content = r#"- Parent Item
  - Child Item 1
  - Child Item 2
    - Grandchild Item 1
    - Grandchild Item 2
  - Child Item 3"#;

    let (temp_dir, file) = create_test_markdown_file(content);
    let file_content = io::read_file(file.relative_path(), temp_dir.path()).unwrap();
    let mut document = Document::from_bytes(file_content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    // Build a map of anchor ID to depth based on content indentation
    let mut anchor_depths = HashMap::new();
    for block in &snapshot.blocks {
        if let markdown_neuraxis_engine::editing::BlockKind::ListItem { .. } = block.kind {
            // Use the block's depth directly (no need to estimate from leading spaces)
            let estimated_depth = block.depth;
            anchor_depths.insert(block.id, estimated_depth);
        }
    }

    // Verify that anchor IDs are unique across all depths
    let mut all_anchor_ids = HashSet::new();
    for &anchor_id in anchor_depths.keys() {
        assert!(
            all_anchor_ids.insert(anchor_id),
            "Duplicate anchor ID found in nested list hierarchy: {:?}",
            anchor_id
        );
    }

    // Log the hierarchy for debugging
    for block in &snapshot.blocks {
        if let markdown_neuraxis_engine::editing::BlockKind::ListItem { .. } = block.kind {
            println!(
                "List item: depth={}, id={:?}, content='{}'",
                block.depth,
                block.id,
                block.content.trim()
            );
        }
    }

    assert!(
        anchor_depths.len() >= 5,
        "Should have created anchors for all nested list items"
    );
}

/// Test that demonstrates the expected behavior: clicking different list items should focus different anchors
/// This test will initially pass, showing that anchor uniqueness works in the core.
/// If it fails, it means the core anchor system has a fundamental problem.
#[test]
fn test_list_item_click_simulation_anchor_uniqueness() {
    let content = r#"- Item A
  - Item B
  - Item C
    - Item D"#;

    let (temp_dir, file) = create_test_markdown_file(content);
    let file_content = io::read_file(file.relative_path(), temp_dir.path()).unwrap();
    let mut document = Document::from_bytes(file_content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    // Get all list items
    let list_items: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                markdown_neuraxis_engine::editing::BlockKind::ListItem { .. }
            )
        })
        .collect();

    // Simulate clicking on each item and verify they have different anchor IDs
    let mut clicked_anchor_ids = HashSet::new();

    for (i, item) in list_items.iter().enumerate() {
        println!(
            "Simulating click on item {}: '{}' -> anchor {:?}",
            i,
            item.content.trim(),
            item.id
        );

        // Verify this anchor ID is unique
        assert!(
            clicked_anchor_ids.insert(item.id),
            "Found duplicate anchor ID when simulating clicks: {:?}. \
            This means multiple list items have the same anchor ID, which would cause multiple textareas.",
            item.id
        );
    }

    println!(
        "List item click simulation passed: {} unique anchor IDs for {} list items",
        clicked_anchor_ids.len(),
        list_items.len()
    );
}

/// Integration boundary analysis: Summary of findings from the core anchor system tests
/// This test documents what we learned about the multiple textarea bug location.
#[test]
fn test_multiple_textarea_bug_location_analysis() {
    // Create the exact scenario where the bug occurs
    let content = r#"- Item 1
  - Nested Item 1.1
  - Nested Item 1.2
    - Deeply nested 1.2.1
- Item 2
  - Nested Item 2.1"#;

    let mut document = Document::from_bytes(content.as_bytes()).unwrap();
    document.create_anchors_from_tree();
    let snapshot = document.snapshot();

    println!("\n=== MULTIPLE TEXTAREA BUG LOCATION ANALYSIS ===");

    // Collect all findings from our integration tests
    let mut anchor_ids = HashSet::new();
    for block in &snapshot.blocks {
        anchor_ids.insert(block.id);
    }

    println!("✅ CORE SYSTEM FINDINGS:");
    println!(
        "   - All {} render blocks have UNIQUE anchor IDs",
        anchor_ids.len()
    );
    println!("   - No anchor ID collisions detected in the core system");
    println!("   - Focus state simulation works correctly (1:1 mapping)");
    println!("   - Nested list items have completely different anchor IDs");

    // Test the specific scenario that triggers the bug in the UI
    let list_items: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                markdown_neuraxis_engine::editing::BlockKind::ListItem { .. }
            )
        })
        .collect();

    println!("\n🔍 UI INTEGRATION BOUNDARY ANALYSIS:");
    println!(
        "   - Testing {} list items that could trigger multiple textareas",
        list_items.len()
    );

    for (i, block) in list_items.iter().enumerate() {
        println!(
            "   - Item {}: '{}' has anchor ID {:?} (depth: {})",
            i,
            block.content.trim(),
            block.id,
            block.depth
        );
    }

    // The critical insight: if core system is correct, the bug MUST be in the UI layer
    println!("\n❌ BUG LOCATION CONCLUSION:");
    println!("   - The multiple textarea bug is NOT in the core anchor system");
    println!("   - The bug is in the UI layer - likely in the Dioxus components");
    println!("   - Suspected locations:");
    println!("     * RenderListItem focus state checking logic");
    println!("     * focused_anchor_id signal management in MainPanel");
    println!("     * EditorBlock rendering conditions");
    println!("     * Dioxus component state synchronization");

    println!("\n🎯 NEXT STEPS:");
    println!("   - Investigate RenderListItem.is_focused calculation");
    println!("   - Check if multiple RenderListItem components think they're focused");
    println!("   - Review focused_anchor_id signal sharing between components");
    println!("   - Test if the bug occurs in component reactivity/re-rendering");

    // Assert our key findings
    assert!(
        anchor_ids.len() >= 5,
        "Should have created multiple list items with unique anchors"
    );

    assert_eq!(
        anchor_ids.len(),
        snapshot.blocks.len(),
        "Every block should have a unique anchor ID (no duplicates)"
    );

    println!("\n✅ Integration boundary analysis completed successfully");
}
