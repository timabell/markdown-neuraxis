//! Test to prove/disprove order-dependent anchor generation bug
//!
//! This test checks if anchor IDs are stable across multiple generations
//! or if they change based on traversal order

use markdown_neuraxis_engine::editing::Document;

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

        // Create content -> anchor_id mapping for this generation
        let mut mapping = std::collections::HashMap::new();
        for block in &snapshot.blocks {
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
    // Test if manipulating the tree-sitter tree affects anchor generation order
    let markdown = r#"
- indented 1
	- indented 1.1
	- indented 1.2
"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    // First generation
    doc.create_anchors_from_tree();
    let snapshot1 = doc.snapshot();
    let mapping1: std::collections::HashMap<String, u128> = snapshot1
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("=== FIRST GENERATION ===");
    for (content, id) in &mapping1 {
        println!("  '{}' -> {}", content, id);
    }

    // Simulate what might happen during UI interactions:
    // Force tree-sitter to re-parse by clearing and re-creating the tree
    let text = doc.text();
    let mut new_doc = Document::from_bytes(text.as_bytes()).unwrap();
    new_doc.create_anchors_from_tree();
    let snapshot2 = new_doc.snapshot();
    let mapping2: std::collections::HashMap<String, u128> = snapshot2
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("\n=== AFTER TREE RE-CREATION ===");
    for (content, id) in &mapping2 {
        println!("  '{}' -> {}", content, id);
    }

    // Test: Anchor IDs should be identical even after tree manipulation
    for (content, id1) in &mapping1 {
        if let Some(id2) = mapping2.get(content) {
            assert_eq!(
                id1, id2,
                "TREE MANIPULATION BUG: '{}' changed anchor_id from {} to {} after tree re-creation",
                content, id1, id2
            );
        }
    }

    println!("✅ Tree manipulation test passed");
}

#[test]
fn test_anchor_generation_with_incremental_parsing() {
    // Test if incremental parsing affects anchor generation
    let markdown = r#"
- indented 1
	- indented 1.1
	- indented 1.2
"#;

    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();
    let initial_snapshot = doc.snapshot();

    let initial_mapping: std::collections::HashMap<String, u128> = initial_snapshot
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("=== INITIAL STATE ===");
    for (content, id) in &initial_mapping {
        println!("  '{}' -> {}", content, id);
    }

    // Make a small edit that shouldn't affect the list structure
    // Insert text at the end that doesn't change any existing structure
    let doc_len = doc.text().len();
    let edit_cmd = markdown_neuraxis_engine::editing::Cmd::InsertText {
        text: "\n\n# New heading".to_string(),
        at: doc_len, // At the very end of the document
    };

    let _patch = doc.apply(edit_cmd);
    let after_edit_snapshot = doc.snapshot();

    let after_edit_mapping: std::collections::HashMap<String, u128> = after_edit_snapshot
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();

    println!("\n=== AFTER INCREMENTAL EDIT ===");
    for (content, id) in &after_edit_mapping {
        println!("  '{}' -> {}", content, id);
    }

    // Test: Anchor IDs for unchanged content should remain stable
    for (content, initial_id) in &initial_mapping {
        if let Some(after_edit_id) = after_edit_mapping.get(content) {
            assert_eq!(
                initial_id, after_edit_id,
                "INCREMENTAL PARSING BUG: '{}' changed anchor_id from {} to {} after incremental edit",
                content, initial_id, after_edit_id
            );
        }
    }

    println!("✅ Incremental parsing test passed");
}

#[test]
fn test_specific_collision_scenario() {
    // Test the exact scenario from the diagnostic output:
    // "indented 1" should get AnchorId(10032346120884770342)
    // "indented 1.1" should get AnchorId(1159858299485389006) initially
    // But after some operation, "indented 1.1" gets the same ID as "indented 1"

    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    doc.create_anchors_from_tree();
    let snapshot = doc.snapshot();

    // Find the specific items from diagnostic output
    let indented_1_items: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1")
        .collect();

    let indented_1_1_items: Vec<_> = snapshot
        .blocks
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

    // Test: Check if we can reproduce the specific collision IDs
    let target_collision_id = 10032346120884770342u128;
    let target_original_id = 1159858299485389006u128;

    let has_collision_id = indented_1_items
        .iter()
        .any(|item| item.id.0 == target_collision_id)
        || indented_1_1_items
            .iter()
            .any(|item| item.id.0 == target_collision_id);

    let has_original_id = indented_1_1_items
        .iter()
        .any(|item| item.id.0 == target_original_id);

    if has_collision_id {
        println!(
            "⚠️ Found collision ID {} in current generation",
            target_collision_id
        );
    }

    if has_original_id {
        println!(
            "✅ Found original ID {} for 'indented 1.1'",
            target_original_id
        );
    }

    // The key test: No two different contents should share an anchor ID
    let mut id_to_content = std::collections::HashMap::new();
    for block in &snapshot.blocks {
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
