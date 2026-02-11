use markdown_neuraxis_engine::parsing::{parse_document, snapshot};

#[test]
fn fixture_simple_paragraph() {
    assert_fixture("combos/simple_paragraph");
}

#[test]
fn fixture_simple_code_fence() {
    assert_fixture("combos/simple_code_fence");
}

#[test]
fn fixture_nested_quote_fence() {
    assert_fixture("combos/nested_quote_fence");
}

#[test]
fn fixture_wikilinks_raw_zones() {
    assert_fixture("combos/wikilinks_raw_zones");
}

#[test]
fn fixture_lossless_spans() {
    assert_fixture("combos/lossless_spans");
}

fn assert_fixture(name: &str) {
    let md = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let rope = xi_rope::Rope::from(md.as_str());

    let doc = parse_document(&rope);
    snapshot::invariants(&rope, &doc.blocks);

    let snap = snapshot::normalize(&rope, &doc.blocks);
    insta::assert_yaml_snapshot!(name, snap);
}

/// Test that slicing any span from the rope reproduces exact text
#[test]
fn lossless_span_invariant() {
    let md = "Hello [[world]]!";
    let rope = xi_rope::Rope::from(md);

    let doc = parse_document(&rope);

    // Every block span should slice back to valid text
    for block in &doc.blocks {
        let text = rope.slice_to_cow(block.span.start..block.span.end);
        assert!(!text.is_empty());
    }
}

/// Test that raw zones (code spans) don't produce wikilinks
#[test]
fn raw_zones_suppress_inline_parsing() {
    use markdown_neuraxis_engine::parsing::{
        blocks::BlockKind, inline::InlineNode, parse_inline_for_block,
    };

    let md = "`[[not a link]]`";
    let rope = xi_rope::Rope::from(md);

    let doc = parse_document(&rope);
    assert_eq!(doc.blocks.len(), 1);
    assert!(matches!(doc.blocks[0].kind, BlockKind::Paragraph));

    let inlines = parse_inline_for_block(&rope, &doc.blocks[0]);

    // Should be a CodeSpan, not a WikiLink
    assert_eq!(inlines.len(), 1);
    assert!(matches!(inlines[0], InlineNode::CodeSpan { .. }));
}
