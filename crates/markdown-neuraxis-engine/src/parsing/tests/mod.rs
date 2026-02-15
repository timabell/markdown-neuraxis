//! Integration tests for the parsing module.
//!
//! Uses snapshot testing with RON format for readability.
//! Fixtures (.md) and snapshots (.snap) are co-located in `fixtures/`.

mod invariants;
mod normalize;

use crate::parsing::{
    blocks::BlockKind, inline::InlineNode, parse_document, parse_inline_for_block,
};

// Fixture-based snapshot tests

#[test]
fn fixture_simple_paragraph() {
    assert_fixture("simple_paragraph");
}

#[test]
fn fixture_simple_code_fence() {
    assert_fixture("simple_code_fence");
}

#[test]
fn fixture_nested_quote_fence() {
    assert_fixture("nested_quote_fence");
}

#[test]
fn fixture_wikilinks_raw_zones() {
    assert_fixture("wikilinks_raw_zones");
}

#[test]
fn fixture_lossless_spans() {
    assert_fixture("lossless_spans");
}

fn assert_fixture(name: &str) {
    let fixtures_dir = format!("{}/src/parsing/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    let md = std::fs::read_to_string(format!("{fixtures_dir}/{name}.md")).unwrap();
    let rope = xi_rope::Rope::from(md.as_str());

    let doc = parse_document(&rope);
    invariants::check(&rope, &doc.blocks);

    let snap = normalize::normalize(&rope, &doc.blocks);
    insta::with_settings!({
        snapshot_path => fixtures_dir.as_str(),
        prepend_module_to_snapshot => false,
    }, {
        insta::assert_ron_snapshot!(name, snap);
    });
}

// Invariant tests

/// Test that slicing any span from the rope reproduces exact text.
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

/// Test that raw zones (code spans) don't produce wikilinks.
#[test]
fn raw_zones_suppress_inline_parsing() {
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

/// Test unclosed constructs become plain text.
#[test]
fn unclosed_constructs_become_text() {
    let md = "[[unclosed and `also unclosed";
    let rope = xi_rope::Rope::from(md);

    let doc = parse_document(&rope);
    let inlines = parse_inline_for_block(&rope, &doc.blocks[0]);

    // Everything should be text since nothing closes
    assert_eq!(inlines.len(), 1);
    assert!(matches!(inlines[0], InlineNode::Text(_)));
}

/// Test empty document produces no blocks.
#[test]
fn empty_document() {
    let rope = xi_rope::Rope::from("");
    let doc = parse_document(&rope);
    assert!(doc.blocks.is_empty());
}

/// Test blank lines don't produce blocks.
#[test]
fn blank_lines_only() {
    let rope = xi_rope::Rope::from("\n\n\n");
    let doc = parse_document(&rope);
    assert!(doc.blocks.is_empty());
}
