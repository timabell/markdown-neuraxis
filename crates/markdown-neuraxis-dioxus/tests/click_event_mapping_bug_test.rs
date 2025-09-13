//! Test for UI click event mapping bug - clicks getting wrong anchor IDs

use markdown_neuraxis_engine::editing::Document;

#[test]
fn test_ui_block_to_anchor_mapping_correctness() {
    // The bug: UI shows "indented 1.2" but click event gets anchor ID of "indented 1"
    // This suggests the UI rendering and click handling are using different block mappings

    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    // This simulates what the UI rendering does vs what click handling does
    // If they're different, we found the bug

    println!("=== TESTING UI BLOCK-TO-ANCHOR MAPPING CONSISTENCY ===");

    // Build what the UI rendering should show: content → anchor mapping
    let mut content_to_anchor_rendering = std::collections::HashMap::new();
    let mut anchor_to_content_clicks = std::collections::HashMap::new();

    for block in &snapshot.blocks {
        // This simulates UI rendering: "what anchor ID should this content show?"
        content_to_anchor_rendering.insert(block.content.clone(), block.id.0);

        // This simulates click handling: "what content does this anchor ID belong to?"
        anchor_to_content_clicks
            .entry(block.id.0)
            .or_insert_with(Vec::new)
            .push(block.content.clone());
    }

    // Check for the specific diagnostic case
    let target_anchor = 10032346120884770342u128;

    if let Some(contents_for_anchor) = anchor_to_content_clicks.get(&target_anchor) {
        println!(
            "Anchor ID {} maps to content: {:?}",
            target_anchor, contents_for_anchor
        );

        if contents_for_anchor.len() > 1 {
            // Multiple content strings map to same anchor - this IS the bug!
            let unique_contents: std::collections::HashSet<_> =
                contents_for_anchor.iter().collect();
            if unique_contents.len() > 1 {
                panic!(
                    "CLICK EVENT MAPPING BUG FOUND: Anchor ID {} maps to multiple different contents: {:?}",
                    target_anchor,
                    unique_contents.into_iter().collect::<Vec<_>>()
                );
            }
        }
    }

    // More general test: look for any anchor ID that maps to multiple contents
    for (anchor_id, contents) in &anchor_to_content_clicks {
        if contents.len() > 1 {
            let unique_contents: std::collections::HashSet<_> = contents.iter().collect();
            if unique_contents.len() > 1 {
                panic!(
                    "GENERAL MAPPING BUG: Anchor ID {} maps to different contents: {:?}. \
                     When user clicks content showing '{}', the click handler might get triggered by '{}'",
                    anchor_id,
                    unique_contents.into_iter().collect::<Vec<_>>(),
                    contents[0],
                    contents[1]
                );
            }
        }
    }

    println!("✅ No click event mapping inconsistencies found");
}

#[test]
fn test_snapshot_block_order_vs_ui_rendering_order() {
    // Another hypothesis: maybe the snapshot block order doesn't match UI rendering order
    // This could cause click events to get mapped to wrong blocks

    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    println!("=== TESTING SNAPSHOT BLOCK ORDER ===");
    println!("Blocks in snapshot order:");

    for (index, block) in snapshot.blocks.iter().enumerate() {
        println!(
            "  [{}] '{}' anchor_id={} depth={}",
            index,
            block.content.chars().take(30).collect::<String>(),
            block.id.0,
            block.depth
        );
    }

    // Look for cases where blocks with similar content are not sequential
    // This might indicate ordering issues that could confuse click handling

    let indented_blocks: Vec<(usize, &str, u128)> = snapshot
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.content.starts_with("indented 1"))
        .map(|(i, block)| (i, block.content.as_str(), block.id.0))
        .collect();

    println!("\n'indented 1*' blocks in order:");
    for (index, content, anchor_id) in &indented_blocks {
        println!(
            "  snapshot[{}] '{}' anchor_id={}",
            index, content, anchor_id
        );
    }

    // Check if there are duplicate anchor IDs in this subset
    let anchor_ids: Vec<u128> = indented_blocks.iter().map(|(_, _, id)| *id).collect();
    let unique_ids: std::collections::HashSet<u128> = anchor_ids.iter().cloned().collect();

    if anchor_ids.len() != unique_ids.len() {
        // Found duplicates in the "indented 1*" family
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = Vec::new();

        for (index, content, anchor_id) in &indented_blocks {
            if !seen.insert(*anchor_id) {
                duplicates.push((index, content, anchor_id));
            }
        }

        panic!(
            "SNAPSHOT ORDER BUG: Found duplicate anchor IDs in 'indented 1*' blocks: {:#?}",
            duplicates
        );
    }

    println!("✅ No snapshot ordering issues found");
}

#[test]
fn test_hierarchical_list_item_click_confusion() {
    // Final hypothesis: The bug might be in hierarchical list rendering
    // Maybe nested items inherit or share anchor IDs with their parents

    let markdown = include_str!("../test_data/actual_runtime_bug_repro.md");
    let mut doc = Document::from_bytes(markdown.as_bytes()).unwrap();
    doc.create_anchors_from_tree();

    let snapshot = doc.snapshot();

    println!("=== TESTING HIERARCHICAL LIST STRUCTURE ===");

    // Build parent-child relationships based on content and depth
    let mut potential_parent_child_issues = Vec::new();

    for (i, block) in snapshot.blocks.iter().enumerate() {
        if block.content == "indented 1" && block.depth == 0 {
            // This is a parent "indented 1" - look for child "indented 1.2"
            for (j, child_block) in snapshot.blocks.iter().enumerate() {
                if child_block.content == "indented 1.2" && child_block.depth > block.depth {
                    // Found potential parent-child relationship
                    println!(
                        "Potential parent-child: parent[{}]='{}' (id={}) child[{}]='{}' (id={})",
                        i, block.content, block.id.0, j, child_block.content, child_block.id.0
                    );

                    if block.id.0 == child_block.id.0 {
                        potential_parent_child_issues.push((
                            (i, block.content.as_str(), block.id.0),
                            (j, child_block.content.as_str(), child_block.id.0),
                        ));
                    }
                }
            }
        }
    }

    if !potential_parent_child_issues.is_empty() {
        panic!(
            "HIERARCHICAL CLICK BUG FOUND: Parent and child blocks have same anchor ID: {:#?}",
            potential_parent_child_issues
        );
    }

    println!("✅ No hierarchical anchor ID conflicts found");
}
