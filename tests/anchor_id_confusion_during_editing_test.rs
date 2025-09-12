//! Test to reproduce the exact anchor ID confusion bug during editing
//!
//! This test reproduces the bug where after editing a list item, other list items
//! get assigned the wrong anchor IDs when the snapshot is recreated.

use markdown_neuraxis::editing::{Cmd, Document};
use std::collections::HashMap;

#[test]
fn test_anchor_id_confusion_after_editing() {
    // Create the exact same structure that triggers the bug
    let markdown = r#"# fresh tab indents

- indented 1
	- indented 1.1
	- indented 1.2

# indented 2

- indented 2
    - indented 2.1
    - indented 2.2
"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    // Take initial snapshot and record anchor IDs
    let initial_snapshot = doc.snapshot();
    let mut initial_anchor_mapping = HashMap::new();
    for block in &initial_snapshot.blocks {
        initial_anchor_mapping.insert(block.content.clone(), block.id);
    }

    println!("=== INITIAL ANCHOR MAPPING ===");
    for (content, anchor_id) in &initial_anchor_mapping {
        if content.contains("indented 1") {
            println!("  '{}' -> {:?}", content, anchor_id);
        }
    }

    // Find the "indented 1.2" block to simulate editing it
    let indented_1_2_block = initial_snapshot
        .blocks
        .iter()
        .find(|b| b.content == "indented 1.2")
        .expect("Should find 'indented 1.2' block");

    println!("\n=== SIMULATING EDIT ON 'indented 1.2' ===");
    println!(
        "Original 'indented 1.2' anchor_id: {:?}",
        indented_1_2_block.id
    );

    // Simulate editing "indented 1.2" by inserting some text
    // This is what happens when user clicks on "indented 1.2" and types
    let edit_position = indented_1_2_block.byte_range.start + "indented 1.2".len();
    let edit_cmd = Cmd::InsertText {
        text: " EDITED".to_string(),
        at: edit_position,
    };

    // Apply the edit - this should trigger anchor rebinding and snapshot recreation
    let _patch = doc.apply(edit_cmd);

    // Take snapshot after editing
    let after_edit_snapshot = doc.snapshot();
    let mut after_edit_anchor_mapping = HashMap::new();
    for block in &after_edit_snapshot.blocks {
        after_edit_anchor_mapping.insert(block.content.clone(), block.id);
    }

    println!("\n=== AFTER EDIT ANCHOR MAPPING ===");
    for (content, anchor_id) in &after_edit_anchor_mapping {
        if content.contains("indented 1") {
            println!("  '{}' -> {:?}", content, anchor_id);
        }
    }

    // THE BUG: After editing, "indented 1.2" should still have its original anchor ID
    // But due to the find_anchor_for_range bug, it gets assigned "indented 1"'s anchor ID

    let original_indented_1_anchor = initial_anchor_mapping["indented 1"];
    let original_indented_1_2_anchor = initial_anchor_mapping["indented 1.2"];

    // Find the edited content - it might be malformed due to wrong edit position
    let edited_content = after_edit_anchor_mapping
        .keys()
        .find(|content| content.contains("EDITED"))
        .expect("Should find edited content");

    let after_edit_indented_1_anchor = after_edit_anchor_mapping["indented 1"];
    let after_edit_indented_1_2_anchor = after_edit_anchor_mapping[edited_content];

    println!("\n=== ANCHOR ID ANALYSIS ===");
    println!(
        "Original 'indented 1' anchor: {:?}",
        original_indented_1_anchor
    );
    println!(
        "Original 'indented 1.2' anchor: {:?}",
        original_indented_1_2_anchor
    );
    println!(
        "After edit 'indented 1' anchor: {:?}",
        after_edit_indented_1_anchor
    );
    println!(
        "After edit 'indented 1.2' anchor: {:?}",
        after_edit_indented_1_2_anchor
    );

    // TEST 1: "indented 1" should keep its anchor ID
    assert_eq!(
        original_indented_1_anchor, after_edit_indented_1_anchor,
        "BUG: 'indented 1' should keep its anchor ID after editing 'indented 1.2'"
    );

    // TEST 2: "indented 1.2" should keep its anchor ID even after editing
    // The anchor represents the identity of the content block, not just its exact text
    assert_eq!(
        original_indented_1_2_anchor, after_edit_indented_1_2_anchor,
        "BUG: 'indented 1.2' should keep its anchor ID after editing - anchor identity should be preserved"
    );

    // TEST 3: No two different contents should share the same anchor ID
    let mut id_to_content = HashMap::new();
    for (content, anchor_id) in &after_edit_anchor_mapping {
        if let Some(existing_content) = id_to_content.get(anchor_id) {
            panic!(
                "ANCHOR COLLISION BUG: Anchor ID {:?} is shared by '{}' and '{}'",
                anchor_id, existing_content, content
            );
        }
        id_to_content.insert(anchor_id, content);
    }

    println!("✅ All anchor identity tests passed");
}

#[test]
fn test_multiple_edit_cycles_preserve_anchor_identity() {
    // Test that anchor identity is preserved across multiple edit cycles
    let markdown = r#"- item 1
	- item 1.1
	- item 1.2
- item 2"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    // Record initial anchor mappings
    let initial_snapshot = doc.snapshot();
    let initial_mappings: HashMap<String, _> = initial_snapshot
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id))
        .collect();

    // Perform multiple edit cycles
    for cycle in 0..3 {
        println!("\n=== EDIT CYCLE {} ===", cycle);

        // Edit different items in each cycle
        let (target_content, edit_text) = match cycle {
            0 => ("item 1", " CYCLE0"),
            1 => ("item 1.1", " CYCLE1"),
            2 => ("item 1.2", " CYCLE2"),
            _ => unreachable!(),
        };

        // Find the target block
        let current_snapshot = doc.snapshot();
        let target_block = current_snapshot
            .blocks
            .iter()
            .find(|b| b.content.contains(target_content) && !b.content.contains("CYCLE"))
            .unwrap_or_else(|| panic!("Should find '{}' block in cycle {}", target_content, cycle));

        // Edit the target
        let edit_position = target_block.byte_range.start + target_block.content.len();
        let edit_cmd = Cmd::InsertText {
            text: edit_text.to_string(),
            at: edit_position,
        };

        let _patch = doc.apply(edit_cmd);

        // Check that anchor identity is preserved
        let after_edit_snapshot = doc.snapshot();
        for block in &after_edit_snapshot.blocks {
            // Extract the original content (remove CYCLE suffixes)
            let original_content = block
                .content
                .replace(" CYCLE0", "")
                .replace(" CYCLE1", "")
                .replace(" CYCLE2", "");

            if let Some(&expected_anchor_id) = initial_mappings.get(&original_content) {
                assert_eq!(
                    block.id, expected_anchor_id,
                    "ANCHOR IDENTITY LOST: '{}' (originally '{}') changed anchor ID from {:?} to {:?} in cycle {}",
                    block.content, original_content, expected_anchor_id, block.id, cycle
                );
            }
        }
    }

    println!("✅ Anchor identity preserved across {} edit cycles", 3);
}
