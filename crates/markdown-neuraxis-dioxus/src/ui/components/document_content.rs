use crate::ui::components::block::BlockRenderer;
use dioxus::prelude::*;
use markdown_neuraxis_engine::editing::{AnchorId, Block, BlockKind, Cmd, Document, Snapshot};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// Component for document content rendering
#[component]
pub fn DocumentContent(
    snapshot: Snapshot,
    source: String,
    notes_path: PathBuf,
    document: Arc<Document>,
    focused_anchor_id: Signal<Option<AnchorId>>,
    collapsed_ids: Signal<HashSet<AnchorId>>,
    on_file_select: Option<Callback<PathBuf>>,
    on_command: Callback<Cmd>,
    on_wikilink_click: Callback<String>,
) -> Element {
    // Compute which block indices are hidden due to heading collapse
    let hidden_indices = compute_hidden_heading_sections(&snapshot.blocks, &collapsed_ids.read());

    rsx! {
        div {
            class: "document-content",
            for (block_index, block) in snapshot.blocks.iter().enumerate() {
                if !hidden_indices.contains(&block_index) {
                    BlockRenderer {
                        key: "{block_index}",
                        block: block.clone(),
                        source: source.clone(),
                        focused_anchor_id,
                        collapsed_ids,
                        on_command,
                        on_wikilink_click
                    }
                }
            }
        }
    }
}

/// Compute indices of blocks hidden by collapsed headings.
/// A collapsed heading hides all following blocks until a heading of same or higher level.
fn compute_hidden_heading_sections(
    blocks: &[Block],
    collapsed_ids: &HashSet<AnchorId>,
) -> HashSet<usize> {
    let mut hidden = HashSet::new();
    let mut i = 0;

    while i < blocks.len() {
        let block = &blocks[i];

        // Check if this is a collapsed heading
        if let BlockKind::Heading { level } = &block.kind
            && collapsed_ids.contains(&block.id)
        {
            // Hide all following blocks until same or higher level heading
            let collapse_level = *level;
            let mut j = i + 1;
            while j < blocks.len() {
                if let BlockKind::Heading { level: next_level } = &blocks[j].kind
                    && *next_level <= collapse_level
                {
                    break;
                }
                hidden.insert(j);
                j += 1;
            }
            i = j;
            continue;
        }
        i += 1;
    }

    hidden
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown_neuraxis_engine::editing::BlockContent;

    /// Create a test block with given kind and ID
    fn make_block(id: u128, kind: BlockKind) -> Block {
        Block {
            id: AnchorId(id),
            kind,
            node_range: 0..10,
            segments: vec![],
            content: BlockContent::Leaf,
        }
    }

    fn heading(id: u128, level: u8) -> Block {
        make_block(id, BlockKind::Heading { level })
    }

    fn paragraph(id: u128) -> Block {
        make_block(id, BlockKind::Paragraph)
    }

    #[test]
    fn test_empty_blocks() {
        let blocks: Vec<Block> = vec![];
        let collapsed = HashSet::new();
        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);
        assert!(hidden.is_empty());
    }

    #[test]
    fn test_no_collapsed_headings() {
        let blocks = vec![heading(1, 1), paragraph(2), heading(3, 2), paragraph(4)];
        let collapsed = HashSet::new();
        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);
        assert!(hidden.is_empty());
    }

    #[test]
    fn test_collapse_h1_hides_all_following() {
        // # H1 (collapsed)
        // paragraph
        // ## H2
        // paragraph
        let blocks = vec![heading(1, 1), paragraph(2), heading(3, 2), paragraph(4)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        // All blocks after H1 should be hidden
        assert!(!hidden.contains(&0)); // H1 visible
        assert!(hidden.contains(&1)); // paragraph hidden
        assert!(hidden.contains(&2)); // H2 hidden
        assert!(hidden.contains(&3)); // paragraph hidden
    }

    #[test]
    fn test_collapse_h2_stops_at_same_level() {
        // # H1
        // ## H2 (collapsed)
        // paragraph
        // ## H2
        // paragraph
        let blocks = vec![
            heading(1, 1),
            heading(2, 2),
            paragraph(3),
            heading(4, 2),
            paragraph(5),
        ];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(2));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        assert!(!hidden.contains(&0)); // H1 visible
        assert!(!hidden.contains(&1)); // collapsed H2 visible
        assert!(hidden.contains(&2)); // paragraph hidden
        assert!(!hidden.contains(&3)); // next H2 visible (same level)
        assert!(!hidden.contains(&4)); // paragraph visible
    }

    #[test]
    fn test_collapse_h2_stops_at_higher_level() {
        // ## H2 (collapsed)
        // paragraph
        // # H1
        // paragraph
        let blocks = vec![heading(1, 2), paragraph(2), heading(3, 1), paragraph(4)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        assert!(!hidden.contains(&0)); // H2 visible
        assert!(hidden.contains(&1)); // paragraph hidden
        assert!(!hidden.contains(&2)); // H1 visible (higher level)
        assert!(!hidden.contains(&3)); // paragraph visible
    }

    #[test]
    fn test_collapse_h2_includes_lower_level_headings() {
        // ## H2 (collapsed)
        // ### H3
        // #### H4
        // ## H2
        let blocks = vec![heading(1, 2), heading(2, 3), heading(3, 4), heading(4, 2)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        assert!(!hidden.contains(&0)); // collapsed H2 visible
        assert!(hidden.contains(&1)); // H3 hidden
        assert!(hidden.contains(&2)); // H4 hidden
        assert!(!hidden.contains(&3)); // next H2 visible
    }

    #[test]
    fn test_multiple_collapsed_headings() {
        // # H1 (collapsed)
        // paragraph
        // # H1 (collapsed)
        // paragraph
        let blocks = vec![heading(1, 1), paragraph(2), heading(3, 1), paragraph(4)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));
        collapsed.insert(AnchorId(3));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        assert!(!hidden.contains(&0)); // first H1 visible
        assert!(hidden.contains(&1)); // first paragraph hidden
        assert!(!hidden.contains(&2)); // second H1 visible
        assert!(hidden.contains(&3)); // second paragraph hidden
    }

    #[test]
    fn test_only_headings_no_content() {
        // # H1 (collapsed)
        // ## H2
        let blocks = vec![heading(1, 1), heading(2, 2)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        assert!(!hidden.contains(&0)); // H1 visible
        assert!(hidden.contains(&1)); // H2 hidden
    }

    #[test]
    fn test_collapsed_heading_at_end_of_document() {
        // # H1
        // ## H2 (collapsed)
        let blocks = vec![heading(1, 1), heading(2, 2)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(2));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        // Nothing to hide - H2 is at end
        assert!(!hidden.contains(&0));
        assert!(!hidden.contains(&1));
    }

    #[test]
    fn test_single_collapsed_heading_only() {
        // # H1 (collapsed)
        let blocks = vec![heading(1, 1)];
        let mut collapsed = HashSet::new();
        collapsed.insert(AnchorId(1));

        let hidden = compute_hidden_heading_sections(&blocks, &collapsed);

        // Nothing to hide - H1 is alone
        assert!(!hidden.contains(&0));
    }
}
