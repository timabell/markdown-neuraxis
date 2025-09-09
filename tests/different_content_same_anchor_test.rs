//! Test for the actual bug: different content getting same anchor ID

use markdown_neuraxis::editing::Document;

#[test]
fn test_different_content_same_anchor_id_bug() {
    // Use the actual runtime data that shows the bug
    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    // Build a map from anchor ID to all content that uses that ID
    let mut anchor_to_contents: std::collections::HashMap<u128, Vec<String>> =
        std::collections::HashMap::new();

    for block in &snapshot.blocks {
        anchor_to_contents
            .entry(block.id.0)
            .or_default()
            .push(block.content.clone());
    }

    // Look for anchor IDs that map to multiple DIFFERENT content strings
    let mut different_content_collisions = Vec::new();

    for (anchor_id, contents) in &anchor_to_contents {
        if contents.len() > 1 {
            // Multiple blocks share this anchor ID - check if they have different content
            let unique_contents: std::collections::HashSet<_> = contents.iter().collect();
            if unique_contents.len() > 1 {
                // Found the bug! Same anchor ID for different content
                different_content_collisions.push((*anchor_id, contents.clone()));
            }
        }
    }

    println!("=== TESTING FOR DIFFERENT CONTENT, SAME ANCHOR ID BUG ===");
    println!(
        "Found {} anchor IDs with different content:",
        different_content_collisions.len()
    );

    for (anchor_id, contents) in &different_content_collisions {
        println!(
            "  Anchor ID {}: {} different contents:",
            anchor_id,
            contents.len()
        );
        for (i, content) in contents.iter().enumerate() {
            println!("    [{}] '{}'", i, content);
        }
    }

    // This should fail if the bug exists
    assert!(
        different_content_collisions.is_empty(),
        "DIFFERENT CONTENT SAME ANCHOR BUG: Found {} anchor IDs shared by different content: {:#?}",
        different_content_collisions.len(),
        different_content_collisions
    );
}

#[test]
fn test_specific_indented_1_vs_indented_1_2_collision() {
    // Test the specific diagnostic case: "indented 1" vs "indented 1.2"
    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    // Find all "indented 1" blocks (exact match)
    let indented_1_blocks: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1")
        .collect();

    // Find all "indented 1.2" blocks (exact match)
    let indented_1_2_blocks: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1.2")
        .collect();

    println!("=== TESTING SPECIFIC 'indented 1' vs 'indented 1.2' COLLISION ===");
    println!("'indented 1' blocks: {}", indented_1_blocks.len());
    for (i, block) in indented_1_blocks.iter().enumerate() {
        println!("  [{}] anchor_id={} depth={}", i, block.id.0, block.depth);
    }

    println!("'indented 1.2' blocks: {}", indented_1_2_blocks.len());
    for (i, block) in indented_1_2_blocks.iter().enumerate() {
        println!("  [{}] anchor_id={} depth={}", i, block.id.0, block.depth);
    }

    // Check for the specific diagnostic collision
    let target_diagnostic_id = 10032346120884770342u128;

    let indented_1_with_target_id = indented_1_blocks
        .iter()
        .any(|b| b.id.0 == target_diagnostic_id);

    let indented_1_2_with_target_id = indented_1_2_blocks
        .iter()
        .any(|b| b.id.0 == target_diagnostic_id);

    if indented_1_with_target_id && indented_1_2_with_target_id {
        panic!(
            "DIAGNOSTIC BUG REPRODUCED: Both 'indented 1' and 'indented 1.2' have anchor ID {}",
            target_diagnostic_id
        );
    }

    // More general check: any "indented 1" and "indented 1.2" sharing an anchor ID?
    for block_1 in &indented_1_blocks {
        for block_1_2 in &indented_1_2_blocks {
            if block_1.id.0 == block_1_2.id.0 {
                panic!(
                    "ANCHOR COLLISION BUG: 'indented 1' and 'indented 1.2' share anchor ID {}",
                    block_1.id.0
                );
            }
        }
    }

    println!("✅ No collision found between 'indented 1' and 'indented 1.2' in current test");
}
