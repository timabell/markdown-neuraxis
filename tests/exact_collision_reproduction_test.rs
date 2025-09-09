//! Test that reproduces the exact anchor ID collision found in diagnostic output

use markdown_neuraxis::editing::Document;

#[test]
fn test_exact_diagnostic_collision_indented_1_vs_indented_1_2() {
    // This test MUST FAIL initially - it reproduces the exact bug from diagnostic output:
    // "indented 1" and "indented 1.2" both getting anchor ID 10032346120884770342

    // Use embedded test data that reproduces the collision scenario
    // This represents the content that previously caused the collision
    let actual_file_content = r#"# fresh tab indents

- indented 1
	- indented 1.1
	- indented 1.2

# indented 2

- indented 2
    - indented 2.1
    - indented 2.2

# other content

- indented 1
    - indented 1.1 hoooooray  
    - indented 1.2
        - indented 1.2.1 - then clicked this
"#;

    let mut doc = Document::from_bytes(actual_file_content.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    // Find the specific blocks that collided in the diagnostic output
    let indented_1_blocks: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1")
        .collect();

    let indented_1_2_blocks: Vec<_> = snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1.2")
        .collect();

    println!("=== REPRODUCING EXACT DIAGNOSTIC COLLISION ===");
    println!("'indented 1' blocks found: {}", indented_1_blocks.len());
    for (i, block) in indented_1_blocks.iter().enumerate() {
        println!(
            "  [{}] '{}' anchor_id={} depth={}",
            i, block.content, block.id.0, block.depth
        );
    }

    println!("'indented 1.2' blocks found: {}", indented_1_2_blocks.len());
    for (i, block) in indented_1_2_blocks.iter().enumerate() {
        println!(
            "  [{}] '{}' anchor_id={} depth={}",
            i, block.content, block.id.0, block.depth
        );
    }

    // The specific collision from diagnostic output
    let collision_anchor_id = 10032346120884770342u128;

    let indented_1_with_collision_id = indented_1_blocks
        .iter()
        .find(|b| b.id.0 == collision_anchor_id);

    let indented_1_2_with_collision_id = indented_1_2_blocks
        .iter()
        .find(|b| b.id.0 == collision_anchor_id);

    if let (Some(block_1), Some(block_1_2)) =
        (indented_1_with_collision_id, indented_1_2_with_collision_id)
    {
        panic!(
            "EXACT DIAGNOSTIC BUG REPRODUCED: Both 'indented 1' (depth={}) and 'indented 1.2' (depth={}) have anchor ID {}. \
             This causes the multiple textarea bug when clicking either item.",
            block_1.depth, block_1_2.depth, collision_anchor_id
        );
    }

    // More general collision detection - any "indented 1" vs "indented 1.2" collision
    for block_1 in &indented_1_blocks {
        for block_1_2 in &indented_1_2_blocks {
            if block_1.id.0 == block_1_2.id.0 {
                panic!(
                    "ANCHOR ID COLLISION REPRODUCED: 'indented 1' and 'indented 1.2' share anchor ID {}. \
                     When user clicks one, both components think they're focused, causing multiple textareas.",
                    block_1.id.0
                );
            }
        }
    }

    println!("✅ No collision reproduced - this means the anchor generation algorithm was fixed!");
}

#[test]
fn test_anchor_generation_algorithm_produces_unique_ids_for_similar_content() {
    // Test the root cause: anchor generation should produce unique IDs for similar content
    // This tests the specific pattern that causes collisions

    let test_cases = vec![
        ("indented 1", "indented 1.2"),
        ("indented 1", "indented 1.1"),
        ("item 1", "item 1.2"),
        ("foo", "foo.1"),
        ("test", "test.1"),
        // Test other potential collision patterns
        ("a", "a.1"),
        ("", ".1"), // Edge case
    ];

    for (content1, content2) in test_cases {
        println!(
            "\n=== Testing collision potential: '{}' vs '{}' ===",
            content1, content2
        );

        // Create minimal documents to test anchor generation
        let doc1_markdown = format!("- {}", content1);
        let doc2_markdown = format!("- {}", content2);

        let mut doc1 = Document::from_bytes(doc1_markdown.as_bytes()).unwrap();
        let mut doc2 = Document::from_bytes(doc2_markdown.as_bytes()).unwrap();

        doc1.create_anchors_from_tree();
        doc2.create_anchors_from_tree();

        let snapshot1 = doc1.snapshot();
        let snapshot2 = doc2.snapshot();

        if let (Some(block1), Some(block2)) = (snapshot1.blocks.first(), snapshot2.blocks.first()) {
            println!("  '{}' -> anchor_id={}", content1, block1.id.0);
            println!("  '{}' -> anchor_id={}", content2, block2.id.0);

            assert_ne!(
                block1.id.0, block2.id.0,
                "ANCHOR GENERATION BUG: Different content '{}' and '{}' produced same anchor ID {}",
                content1, content2, block1.id.0
            );
        }
    }

    println!("✅ All test cases produce unique anchor IDs");
}

#[test]
fn test_combined_document_collision_reproduction() {
    // Test the collision in a combined document (more realistic scenario)
    // This simulates the actual structure that causes the collision

    let problematic_markdown = r#"
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

    let mut doc = Document::from_bytes(problematic_markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    // Build collision detection map
    let mut anchor_to_blocks: std::collections::HashMap<u128, Vec<String>> =
        std::collections::HashMap::new();

    for block in &snapshot.blocks {
        anchor_to_blocks
            .entry(block.id.0)
            .or_default()
            .push(block.content.clone());
    }

    // Find collisions (same anchor ID, different content)
    let mut collisions = Vec::new();
    for (anchor_id, contents) in &anchor_to_blocks {
        if contents.len() > 1 {
            let unique_contents: std::collections::HashSet<_> = contents.iter().collect();
            if unique_contents.len() > 1 {
                collisions.push((
                    *anchor_id,
                    unique_contents.into_iter().cloned().collect::<Vec<_>>(),
                ));
            }
        }
    }

    if !collisions.is_empty() {
        panic!(
            "COMBINED DOCUMENT COLLISION BUG: Found {} anchor ID collisions: {:#?}",
            collisions.len(),
            collisions
        );
    }

    println!("✅ No collisions found in combined document test");
}
