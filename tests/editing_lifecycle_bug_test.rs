//! Test that simulates the runtime editing lifecycle that causes anchor ID instability

use markdown_neuraxis::editing::Document;

#[test]
fn test_editing_lifecycle_causes_anchor_instability() {
    // Use the actual runtime data
    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    // Step 1: Initial document load (like opening the file)
    doc.create_anchors_from_tree();
    let initial_snapshot = doc.snapshot();

    // Capture initial anchor mappings
    let mut anchor_history = Vec::new();
    let initial_mapping: std::collections::HashMap<String, u128> = initial_snapshot
        .blocks
        .iter()
        .map(|b| (b.content.clone(), b.id.0))
        .collect();
    anchor_history.push(("initial_load", initial_mapping));

    // Step 2: Simulate UI interactions that might trigger anchor regeneration
    // This mimics what happens when you click on different items

    let target_contents = vec!["indented 1", "indented 1.2", "indented 1.1"];

    for (cycle, target_content) in target_contents.iter().enumerate() {
        println!(
            "\n=== EDITING CYCLE {}: Simulating click on '{}' ===",
            cycle, target_content
        );

        // This simulates what happens in the UI when you click an item:
        // 1. Focus event triggered
        // 2. UI might regenerate anchors for consistency
        // 3. Document state changes

        doc.create_anchors_from_tree(); // This is what might cause instability

        let cycle_snapshot = doc.snapshot();
        let cycle_mapping: std::collections::HashMap<String, u128> = cycle_snapshot
            .blocks
            .iter()
            .map(|b| (b.content.clone(), b.id.0))
            .collect();

        anchor_history.push((format!("cycle_{}", cycle).leak(), cycle_mapping));

        // Check for the bug: same anchor ID for different content
        let mut anchor_to_contents: std::collections::HashMap<u128, Vec<String>> =
            std::collections::HashMap::new();

        for block in &cycle_snapshot.blocks {
            anchor_to_contents
                .entry(block.id.0)
                .or_insert_with(Vec::new)
                .push(block.content.clone());
        }

        // Look for anchor collisions (same ID, different content)
        for (anchor_id, contents) in &anchor_to_contents {
            if contents.len() > 1 {
                let unique_contents: std::collections::HashSet<_> = contents.iter().collect();
                if unique_contents.len() > 1 {
                    // Found the bug!
                    panic!(
                        "EDITING LIFECYCLE BUG FOUND in cycle {}: Anchor ID {} shared by different content: {:?}",
                        cycle,
                        anchor_id,
                        unique_contents.into_iter().collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    // Step 3: Compare anchor stability across editing cycles
    println!("\n=== CHECKING ANCHOR STABILITY ACROSS EDITING CYCLES ===");

    if anchor_history.len() >= 2 {
        let (_, initial) = &anchor_history[0];

        for (cycle_name, cycle_mapping) in anchor_history.iter().skip(1) {
            for (content, initial_id) in initial {
                if let Some(current_id) = cycle_mapping.get(content) {
                    if initial_id != current_id {
                        panic!(
                            "ANCHOR INSTABILITY DETECTED: '{}' had anchor_id {} initially but {} in {}",
                            content, initial_id, current_id, cycle_name
                        );
                    }
                }
            }
        }
    }

    println!("✅ No anchor instability detected in editing lifecycle simulation");
}

#[test]
fn test_rapid_focus_changes_cause_collision() {
    // Simulate rapid clicking between items (like a user quickly clicking different items)
    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();

    doc.create_anchors_from_tree();
    let initial_snapshot = doc.snapshot();

    // Find items that could collide based on diagnostic output
    let indented_1_items: Vec<_> = initial_snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1")
        .collect();

    let indented_1_2_items: Vec<_> = initial_snapshot
        .blocks
        .iter()
        .filter(|b| b.content == "indented 1.2")
        .collect();

    println!("=== RAPID FOCUS SIMULATION ===");
    println!("Found {} 'indented 1' items", indented_1_items.len());
    println!("Found {} 'indented 1.2' items", indented_1_2_items.len());

    // Simulate rapid focus changes (like clicking back and forth)
    for rapid_cycle in 0..10 {
        // This simulates the sequence: click item → focus → anchor regeneration
        doc.create_anchors_from_tree();

        let rapid_snapshot = doc.snapshot();

        // Check if the rapid regeneration caused the diagnostic bug
        let current_indented_1: Vec<_> = rapid_snapshot
            .blocks
            .iter()
            .filter(|b| b.content == "indented 1")
            .collect();

        let current_indented_1_2: Vec<_> = rapid_snapshot
            .blocks
            .iter()
            .filter(|b| b.content == "indented 1.2")
            .collect();

        // Check for collision between "indented 1" and "indented 1.2"
        for item_1 in &current_indented_1 {
            for item_1_2 in &current_indented_1_2 {
                if item_1.id.0 == item_1_2.id.0 {
                    panic!(
                        "RAPID FOCUS BUG FOUND in cycle {}: 'indented 1' and 'indented 1.2' both have anchor_id {}",
                        rapid_cycle, item_1.id.0
                    );
                }
            }
        }

        // Special check for the specific diagnostic ID
        let diagnostic_id = 10032346120884770342u128;
        let items_with_diagnostic_id: Vec<_> = rapid_snapshot
            .blocks
            .iter()
            .filter(|b| b.id.0 == diagnostic_id)
            .collect();

        if items_with_diagnostic_id.len() > 1 {
            let contents: Vec<&str> = items_with_diagnostic_id
                .iter()
                .map(|b| b.content.as_str())
                .collect();
            if contents
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 1
            {
                panic!(
                    "DIAGNOSTIC BUG REPRODUCED in rapid cycle {}: Anchor ID {} shared by different content: {:?}",
                    rapid_cycle, diagnostic_id, contents
                );
            }
        }
    }

    println!("✅ No rapid focus collision detected");
}
