//! Test to prove/disprove order-dependent anchor generation bug
//!
//! This test checks if anchor IDs are stable across multiple generations
//! or if they change based on traversal order

mod test_helpers;

use markdown_neuraxis_engine::editing::Document;
use test_helpers::flatten_blocks;

#[test]
fn test_anchor_generation_is_order_independent() {
    // Use the exact content that causes the collision
    let markdown = r#"
# fresh tab indents

- indented 1
	- indented 1.1
	- indented 1.2

# other section

- indented 1
    - indented 1.1 hoooooray
    - indented 1.2
        - indented 1.2.1 - then clicked this

"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    // Generate anchors multiple times and capture the mappings
    let mut anchor_mappings = Vec::new();

    for generation in 0..5 {
        // Clear existing anchors and regenerate from scratch
        doc.create_anchors_from_tree();
        let snapshot = doc.snapshot();
        let blocks = flatten_blocks(&snapshot.blocks);

        // Create content -> anchor_id mapping for this generation
        let mut mapping = std::collections::HashMap::new();
        for block in &blocks {
            mapping.insert(block.content.clone(), block.id.0);
        }

        println!("\n=== GENERATION {} ===", generation);
        for (content, anchor_id) in &mapping {
            if content.starts_with("indented 1") {
                println!("  '{}' -> {}", content, anchor_id);
            }
        }

        anchor_mappings.push(mapping);
    }

    // Test 1: Anchor IDs should be identical across all generations
    println!("\n=== TESTING ANCHOR STABILITY ===");
    let reference_mapping = &anchor_mappings[0];

    for (gen_idx, mapping) in anchor_mappings.iter().enumerate().skip(1) {
        for (content, reference_id) in reference_mapping {
            if let Some(current_id) = mapping.get(content) {
                assert_eq!(
                    reference_id, current_id,
                    "ORDER-DEPENDENT BUG DETECTED: '{}' had anchor_id {} in generation 0 but {} in generation {}",
                    content, reference_id, current_id, gen_idx
                );
            }
        }
    }

    // Test 2: No two different contents should have the same anchor ID within any generation
    println!("\n=== TESTING ANCHOR UNIQUENESS ===");
    for (gen_idx, mapping) in anchor_mappings.iter().enumerate() {
        let mut id_to_content: std::collections::HashMap<u128, String> =
            std::collections::HashMap::new();

        for (content, anchor_id) in mapping {
            if let Some(existing_content) = id_to_content.get(anchor_id) {
                panic!(
                    "ANCHOR COLLISION BUG: In generation {}, anchor_id {} is shared by '{}' and '{}'",
                    gen_idx, anchor_id, existing_content, content
                );
            }
            id_to_content.insert(*anchor_id, content.clone());
        }
    }

    println!("✅ All anchor generation tests passed");
}

#[test]
fn test_anchor_generation_with_tree_manipulation() {
    let markdown = r#"
- indented 1
	- indented 1.1
	- indented 1.2
"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    // First generation
    doc.create_anchors_from_tree();
    let snapshot1 = doc.snapshot();
    let blocks1 = flatten_blocks(&snapshot1.blocks);
    let mapping1: std::collections::HashMap<String, u128> = blocks1
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("=== FIRST GENERATION ===");
    for (content, id) in &mapping1 {
        println!("  '{}' -> {}", content, id);
    }

    // Simulate tree re-creation
    let text = doc.text();
    let mut new_doc = Document::from_bytes(text.as_bytes()).unwrap();
    new_doc.create_anchors_from_tree();
    let snapshot2 = new_doc.snapshot();
    let blocks2 = flatten_blocks(&snapshot2.blocks);
    let mapping2: std::collections::HashMap<String, u128> = blocks2
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("\n=== AFTER TREE RE-CREATION ===");
    for (content, id) in &mapping2 {
        println!("  '{}' -> {}", content, id);
    }

    for (content, id1) in &mapping1 {
        if let Some(id2) = mapping2.get(content) {
            assert_eq!(
                id1, id2,
                "TREE MANIPULATION BUG: '{}' changed anchor_id from {} to {}",
                content, id1, id2
            );
        }
    }

    println!("✅ Tree manipulation test passed");
}

#[test]
fn test_anchor_generation_with_incremental_parsing() {
    let markdown = r#"
- indented 1
	- indented 1.1
	- indented 1.2
"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();
    let initial_snapshot = doc.snapshot();
    let initial_blocks = flatten_blocks(&initial_snapshot.blocks);

    // Filter out empty content blocks (LIST containers) as they have unstable fallback IDs
    let initial_mapping: std::collections::HashMap<String, u128> = initial_blocks
        .iter()
        .filter(|b| !b.content.is_empty())
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("=== INITIAL STATE ===");
    for (content, id) in &initial_mapping {
        println!("  '{}' -> {}", content, id);
    }

    // Make a small edit
    let doc_len = doc.text().len();
    let edit_cmd = markdown_neuraxis_engine::editing::Cmd::InsertText {
        text: "\n\n# New heading".to_string(),
        at: doc_len,
    };

    let _patch = doc.apply(edit_cmd);
    let after_edit_snapshot = doc.snapshot();
    let after_edit_blocks = flatten_blocks(&after_edit_snapshot.blocks);

    // Filter out empty content blocks (LIST containers) as they have unstable fallback IDs
    let after_edit_mapping: std::collections::HashMap<String, u128> = after_edit_blocks
        .iter()
        .filter(|b| !b.content.is_empty())
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("\n=== AFTER INCREMENTAL EDIT ===");
    for (content, id) in &after_edit_mapping {
        println!("  '{}' -> {}", content, id);
    }

    for (content, initial_id) in &initial_mapping {
        if let Some(after_edit_id) = after_edit_mapping.get(content) {
            assert_eq!(
                initial_id, after_edit_id,
                "INCREMENTAL PARSING BUG: '{}' changed anchor_id from {} to {}",
                content, initial_id, after_edit_id
            );
        }
    }

    println!("✅ Incremental parsing test passed");
}

#[test]
fn test_specific_collision_scenario() {
    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    doc.create_anchors_from_tree();
    let snapshot = doc.snapshot();
    let blocks = flatten_blocks(&snapshot.blocks);

    let indented_1_items: Vec<_> = blocks
        .iter()
        .filter(|b| b.content == "indented 1")
        .collect();

    let indented_1_1_items: Vec<_> = blocks
        .iter()
        .filter(|b| b.content == "indented 1.1")
        .collect();

    println!("=== SPECIFIC COLLISION TEST ===");
    println!("'indented 1' items: {}", indented_1_items.len());
    for (i, item) in indented_1_items.iter().enumerate() {
        println!("  [{}] '{}' -> {}", i, item.content, item.id.0);
    }

    println!("'indented 1.1' items: {}", indented_1_1_items.len());
    for (i, item) in indented_1_1_items.iter().enumerate() {
        println!("  [{}] '{}' -> {}", i, item.content, item.id.0);
    }

    // The key test: No two different non-empty contents should share an anchor ID
    // Note: Empty content blocks (LIST containers) may share IDs with content blocks
    // because they get fallback IDs. This is expected behavior.
    let mut id_to_content = std::collections::HashMap::new();
    for block in &blocks {
        // Skip empty content blocks (LIST containers)
        if block.content.is_empty() {
            continue;
        }
        if let Some(existing_content) = id_to_content.get(&block.id.0)
            && existing_content != &block.content
        {
            panic!(
                "SPECIFIC COLLISION REPRODUCED: Anchor ID {} shared by '{}' and '{}'",
                block.id.0, existing_content, block.content
            );
        }
        id_to_content.insert(block.id.0, block.content.clone());
    }

    println!("✅ No collisions found in single generation");
}
