//! Context menu for collapse toggle actions (expand/collapse all/children)

use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockContent, BlockKind, Snapshot};
use std::collections::HashSet;

/// Context menu for collapse toggle with expand/collapse options
#[component]
pub fn CollapseContextMenu(
    position: (f64, f64),
    block_id: AnchorId,
    snapshot: Snapshot,
    mut collapsed_ids: Signal<HashSet<AnchorId>>,
    on_close: Callback<()>,
) -> Element {
    let (x, y) = position;

    // Find the block to determine its type for children computation
    let block = find_block_by_id(&snapshot.blocks, block_id);

    rsx! {
        // Overlay to capture clicks outside the menu
        div {
            class: "collapse-context-menu-overlay",
            onclick: move |_| on_close.call(()),
        }
        div {
            class: "collapse-context-menu",
            style: "left: {x}px; top: {y}px;",
            onclick: |evt| evt.stop_propagation(),
            // Collapse All - collapse this item and all nested sections
            div {
                class: "collapse-context-menu-item",
                onclick: {
                    let snapshot = snapshot.clone();
                    let block = block.clone();
                    move |_| {
                        if let Some(ref b) = block {
                            let mut children = collect_children_ids(&snapshot, b);
                            children.insert(b.id); // Include the selected item
                            let mut ids = collapsed_ids.write();
                            for child_id in children {
                                ids.insert(child_id);
                            }
                        }
                        on_close.call(());
                    }
                },
                "Collapse All"
            }
            // Expand All - expand this item and all nested sections
            div {
                class: "collapse-context-menu-item",
                onclick: {
                    let snapshot = snapshot.clone();
                    let block = block.clone();
                    move |_| {
                        if let Some(ref b) = block {
                            let mut children = collect_children_ids(&snapshot, b);
                            children.insert(b.id); // Include the selected item
                            let mut ids = collapsed_ids.write();
                            for child_id in children {
                                ids.remove(&child_id);
                            }
                        }
                        on_close.call(());
                    }
                },
                "Expand All"
            }
            // Separator
            div { class: "collapse-context-menu-separator" }
            // Collapse All in Document
            div {
                class: "collapse-context-menu-item",
                onclick: {
                    let snapshot = snapshot.clone();
                    move |_| {
                        let all_collapsible = collect_all_collapsible(&snapshot);
                        *collapsed_ids.write() = all_collapsible;
                        on_close.call(());
                    }
                },
                "Collapse All in Document"
            }
            // Expand All in Document
            div {
                class: "collapse-context-menu-item",
                onclick: move |_| {
                    collapsed_ids.write().clear();
                    on_close.call(());
                },
                "Expand All in Document"
            }
        }
    }
}

/// Find a block by its ID in the snapshot
fn find_block_by_id(blocks: &[Block], target_id: AnchorId) -> Option<Block> {
    for block in blocks {
        if block.id == target_id {
            return Some(block.clone());
        }
        if let BlockContent::Children(children) = &block.content
            && let Some(found) = find_block_by_id(children, target_id)
        {
            return Some(found);
        }
    }
    None
}

/// Collect all collapsible block IDs from the snapshot.
/// Collapsible blocks are: headings and list items with children.
fn collect_all_collapsible(snapshot: &Snapshot) -> HashSet<AnchorId> {
    let mut ids = HashSet::new();
    collect_collapsible_recursive(&snapshot.blocks, &mut ids);
    ids
}

fn collect_collapsible_recursive(blocks: &[Block], ids: &mut HashSet<AnchorId>) {
    for block in blocks {
        // List items with children are collapsible
        if matches!(block.kind, BlockKind::ListItem { .. })
            && let BlockContent::Children(children) = &block.content
            && !children.is_empty()
        {
            ids.insert(block.id);
        }
        // Headings are always collapsible
        if matches!(block.kind, BlockKind::Heading { .. }) {
            ids.insert(block.id);
        }
        // Recurse into children
        if let BlockContent::Children(children) = &block.content {
            collect_collapsible_recursive(children, ids);
        }
    }
}

/// Collect IDs of children for a given block.
/// For ListItems: all nested block IDs recursively.
/// For Headings: all headings until next same/higher level heading.
fn collect_children_ids(snapshot: &Snapshot, block: &Block) -> HashSet<AnchorId> {
    match &block.kind {
        BlockKind::ListItem { .. } => collect_list_item_children(block),
        BlockKind::Heading { level } => collect_heading_children(snapshot, block.id, *level),
        _ => HashSet::new(),
    }
}

/// Collect all nested AnchorIds from a list item's children recursively
fn collect_list_item_children(block: &Block) -> HashSet<AnchorId> {
    let mut ids = HashSet::new();
    if let BlockContent::Children(children) = &block.content {
        collect_nested_ids(children, &mut ids);
    }
    ids
}

fn collect_nested_ids(blocks: &[Block], ids: &mut HashSet<AnchorId>) {
    for block in blocks {
        // Only collect collapsible items (list items with children, headings)
        if matches!(block.kind, BlockKind::ListItem { .. })
            && let BlockContent::Children(children) = &block.content
            && !children.is_empty()
        {
            ids.insert(block.id);
        }
        if matches!(block.kind, BlockKind::Heading { .. }) {
            ids.insert(block.id);
        }
        if let BlockContent::Children(children) = &block.content {
            collect_nested_ids(children, ids);
        }
    }
}

/// Collect all heading IDs that are children of a given heading.
/// Children are all blocks until next same/higher level heading.
fn collect_heading_children(
    snapshot: &Snapshot,
    heading_id: AnchorId,
    heading_level: u8,
) -> HashSet<AnchorId> {
    let mut ids = HashSet::new();

    // Find the index of our heading
    let Some(heading_index) = snapshot.blocks.iter().position(|b| b.id == heading_id) else {
        return ids;
    };

    // Collect all headings after this one until we hit same/higher level
    for block in snapshot.blocks.iter().skip(heading_index + 1) {
        if let BlockKind::Heading { level } = &block.kind {
            if *level <= heading_level {
                // Same or higher level heading - stop collecting
                break;
            }
            // Lower level heading - this is a child
            ids.insert(block.id);
        }
        // Collect collapsible items within this block's children (e.g., nested lists)
        if let BlockContent::Children(children) = &block.content {
            collect_nested_ids(children, &mut ids);
        }
        // Also check if this top-level block itself is a collapsible list item
        // (collect_nested_ids only looks at children, not the block itself)
        if matches!(block.kind, BlockKind::ListItem { .. })
            && let BlockContent::Children(children) = &block.content
            && !children.is_empty()
        {
            ids.insert(block.id);
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test block with given kind and ID
    fn make_block(id: u128, kind: BlockKind, content: BlockContent) -> Block {
        Block {
            id: AnchorId(id),
            kind,
            node_range: 0..10,
            segments: vec![],
            content,
        }
    }

    fn heading(id: u128, level: u8) -> Block {
        make_block(id, BlockKind::Heading { level }, BlockContent::Leaf)
    }

    fn paragraph(id: u128) -> Block {
        make_block(id, BlockKind::Paragraph, BlockContent::Leaf)
    }

    fn list_item_leaf(id: u128) -> Block {
        make_block(
            id,
            BlockKind::ListItem {
                marker: "- ".to_string(),
            },
            BlockContent::Leaf,
        )
    }

    fn list_item_with_children(id: u128, children: Vec<Block>) -> Block {
        make_block(
            id,
            BlockKind::ListItem {
                marker: "- ".to_string(),
            },
            BlockContent::Children(children),
        )
    }

    fn list(id: u128, children: Vec<Block>) -> Block {
        make_block(
            id,
            BlockKind::List { ordered: false },
            BlockContent::Children(children),
        )
    }

    // ============ find_block_by_id tests ============

    #[test]
    fn test_find_block_by_id_empty_blocks() {
        let blocks: Vec<Block> = vec![];
        assert!(find_block_by_id(&blocks, AnchorId(1)).is_none());
    }

    #[test]
    fn test_find_block_by_id_not_found() {
        let blocks = vec![heading(1, 1), paragraph(2)];
        assert!(find_block_by_id(&blocks, AnchorId(999)).is_none());
    }

    #[test]
    fn test_find_block_by_id_found_at_top_level() {
        let blocks = vec![heading(1, 1), paragraph(2)];
        let found = find_block_by_id(&blocks, AnchorId(2));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, AnchorId(2));
    }

    #[test]
    fn test_find_block_by_id_found_nested() {
        let nested_item = list_item_leaf(3);
        let parent_item = list_item_with_children(2, vec![list(10, vec![nested_item])]);
        let top_list = list(1, vec![parent_item]);
        let blocks = vec![top_list];

        let found = find_block_by_id(&blocks, AnchorId(3));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, AnchorId(3));
    }

    // ============ collect_all_collapsible tests ============

    #[test]
    fn test_collect_all_collapsible_empty() {
        let snapshot = Snapshot { blocks: vec![] };
        let ids = collect_all_collapsible(&snapshot);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_collect_all_collapsible_headings_only() {
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1), heading(2, 2), paragraph(3)],
        };
        let ids = collect_all_collapsible(&snapshot);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&AnchorId(1)));
        assert!(ids.contains(&AnchorId(2)));
        assert!(!ids.contains(&AnchorId(3))); // paragraphs not collapsible
    }

    #[test]
    fn test_collect_all_collapsible_list_items_without_children() {
        let snapshot = Snapshot {
            blocks: vec![list(1, vec![list_item_leaf(2), list_item_leaf(3)])],
        };
        let ids = collect_all_collapsible(&snapshot);
        // Leaf list items are not collapsible
        assert!(ids.is_empty());
    }

    #[test]
    fn test_collect_all_collapsible_list_items_with_children() {
        let nested_item = list_item_leaf(4);
        let parent_item = list_item_with_children(2, vec![list(3, vec![nested_item])]);
        let snapshot = Snapshot {
            blocks: vec![list(1, vec![parent_item])],
        };
        let ids = collect_all_collapsible(&snapshot);
        // Only parent_item (id=2) is collapsible because it has children
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&AnchorId(2)));
    }

    #[test]
    fn test_collect_all_collapsible_mixed() {
        // H1 + list with nested items
        let nested_item = list_item_leaf(5);
        let parent_item = list_item_with_children(3, vec![list(4, vec![nested_item])]);
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1), list(2, vec![parent_item])],
        };
        let ids = collect_all_collapsible(&snapshot);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&AnchorId(1))); // heading
        assert!(ids.contains(&AnchorId(3))); // list item with children
    }

    // ============ collect_children_ids tests ============

    #[test]
    fn test_collect_children_ids_paragraph_returns_empty() {
        let snapshot = Snapshot {
            blocks: vec![paragraph(1)],
        };
        let para = &snapshot.blocks[0];
        let ids = collect_children_ids(&snapshot, para);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_collect_children_ids_list_item_without_children() {
        let snapshot = Snapshot {
            blocks: vec![list(1, vec![list_item_leaf(2)])],
        };
        let list_block = &snapshot.blocks[0];
        if let BlockContent::Children(children) = &list_block.content {
            let ids = collect_children_ids(&snapshot, &children[0]);
            assert!(ids.is_empty());
        }
    }

    #[test]
    fn test_collect_children_ids_list_item_with_nested() {
        // Parent item with two nested child items
        let nested1 = list_item_leaf(4);
        let nested2 = list_item_with_children(5, vec![list(6, vec![list_item_leaf(7)])]);
        let parent_item = list_item_with_children(2, vec![list(3, vec![nested1, nested2])]);
        let snapshot = Snapshot {
            blocks: vec![list(1, vec![parent_item.clone()])],
        };

        let ids = collect_children_ids(&snapshot, &parent_item);
        // nested2 (id=5) has children so it should be in the set
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&AnchorId(5)));
    }

    // ============ collect_heading_children tests ============

    #[test]
    fn test_collect_heading_children_not_found() {
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(999), 1);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_collect_heading_children_h1_with_h2_children() {
        // # H1
        // ## H2
        // ## H2
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1), heading(2, 2), heading(3, 2)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(1), 1);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&AnchorId(2)));
        assert!(ids.contains(&AnchorId(3)));
    }

    #[test]
    fn test_collect_heading_children_h2_stops_at_same_level() {
        // ## H2 (id=1)
        // ### H3 (id=2)
        // ## H2 (id=3)
        // ### H3 (id=4)
        let snapshot = Snapshot {
            blocks: vec![heading(1, 2), heading(2, 3), heading(3, 2), heading(4, 3)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(1), 2);
        // Only H3 (id=2) is a child, H2 (id=3) stops collection
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&AnchorId(2)));
    }

    #[test]
    fn test_collect_heading_children_h2_stops_at_higher_level() {
        // ## H2 (id=1)
        // ### H3 (id=2)
        // # H1 (id=3)
        // ### H3 (id=4)
        let snapshot = Snapshot {
            blocks: vec![heading(1, 2), heading(2, 3), heading(3, 1), heading(4, 3)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(1), 2);
        // Only H3 (id=2) is a child, H1 (id=3) stops collection
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&AnchorId(2)));
    }

    #[test]
    fn test_collect_heading_children_deeply_nested() {
        // # H1 (id=1)
        // ## H2 (id=2)
        // ### H3 (id=3)
        // #### H4 (id=4)
        // # H1 (id=5)
        let snapshot = Snapshot {
            blocks: vec![
                heading(1, 1),
                heading(2, 2),
                heading(3, 3),
                heading(4, 4),
                heading(5, 1),
            ],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(1), 1);
        // H2, H3, H4 are all children of first H1
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&AnchorId(2)));
        assert!(ids.contains(&AnchorId(3)));
        assert!(ids.contains(&AnchorId(4)));
    }

    #[test]
    fn test_collect_heading_children_at_end_of_document() {
        // # H1 (id=1)
        // ## H2 (id=2)
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1), heading(2, 2)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(2), 2);
        // H2 is at end, no children
        assert!(ids.is_empty());
    }

    #[test]
    fn test_collect_heading_children_includes_nested_list_items() {
        // # H1 (id=1)
        // List with nested items
        // # H1 (id=5)
        let nested_item = list_item_leaf(4);
        let parent_item = list_item_with_children(3, vec![list(10, vec![nested_item])]);
        let snapshot = Snapshot {
            blocks: vec![heading(1, 1), list(2, vec![parent_item]), heading(5, 1)],
        };
        let ids = collect_heading_children(&snapshot, AnchorId(1), 1);
        // Should include the collapsible list item (id=3)
        assert!(ids.contains(&AnchorId(3)));
    }
}
