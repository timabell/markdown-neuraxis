# ADR-0003: Test Framework for Tree Structure Comparison

## Status
Proposed

## Context

The markdown-neuraxis parser produces hierarchical `OutlineItem` tree structures from markdown input. Testing these trees with standard Rust assertions (`assert_eq!`) provides unhelpful error messages like "assertion `left == right` failed" without indicating where in the tree hierarchy differences occur.

For a parser that needs to verify complex nested structures like:
```
- Parent item
  - Child item
    - Grandchild
  - Another child
- Second parent
```

We need better error reporting that shows exactly where structural differences occur.

## Problem Statement

Need better error information when testing tree structure equality, specifically:
1. Clear indication of where in the hierarchy differences occur
2. Readable output for complex nested structures
3. Efficient test authoring for multiple markdown input → expected tree output cases

## Decision

Adopt a **two-pronged testing approach**:

### Primary: [insta](https://crates.io/crates/insta) for comprehensive tree structure testing
- Use `insta` with YAML snapshots for testing complete tree structures
- Provides detailed hierarchical diffs when structures change
- Excellent for regression testing and understanding complex trees

### Secondary: [pretty_assertions](https://crates.io/crates/pretty_assertions) for direct property testing  
- Use `pretty_assertions` for specific property assertions
- Provides colorized diffs for individual comparisons
- Maintains familiar assertion syntax

### Foundation: [rstest](https://crates.io/crates/rstest) for parameterized testing
- Use `rstest` to eliminate boilerplate for "markdown X → tree Y" test cases
- Clean separation of test data from test logic

## Rationale

### Why not custom tree diff implementation?
Research showed existing solutions that solve our core problem better than custom code:
- `insta` provides exactly the hierarchical diff visibility we need
- `pretty_assertions` is battle-tested and widely adopted
- Building custom tree diff is reinventing the wheel

### Why insta over pure pretty_assertions?
- `pretty_assertions` still shows flat textual diff, doesn't understand tree structure well
- `insta` YAML snapshots are perfect for hierarchical data like our `OutlineItem` trees
- Snapshot testing is ideal for parser validation - captures the complete expected output

### Why the combination approach?
- `insta` excels at "shape correctness" - is the entire tree structure right?
- `pretty_assertions` excels at "property correctness" - are specific values/counts right?
- Together they provide comprehensive coverage with clear error messages

## Implementation

```toml
[dev-dependencies]
rstest = "0.23"
insta = { version = "1.34", features = ["yaml"] }  
pretty_assertions = "1.4"
```

### Tree Structure Tests
```rust
use insta::assert_yaml_snapshot;

#[rstest]
#[case("- Parent\n  - Child", "nested_bullet_list")]
#[case("# Heading\n- Item", "heading_with_list")]
fn test_outline_parsing(#[case] markdown: &str, #[case] name: &str) {
    let doc = parse_markdown_outline(markdown);
    assert_yaml_snapshot!(name, doc.outline);
}
```

### Property Tests
```rust
#[cfg(test)]
use pretty_assertions::assert_eq;

#[test]
fn test_specific_outline_properties() {
    let doc = parse_markdown_outline("- Item 1\n- Item 2");
    assert_eq!(doc.outline.len(), 2); // Clear diff if wrong count
}
```

## Alternatives Considered

1. **Only [pretty_assertions](https://crates.io/crates/pretty_assertions)**: Insufficient for deep hierarchies - still shows flat diff
2. **Custom tree diff helpers**: Unnecessary when existing solutions solve the problem
3. **Only [insta](https://crates.io/crates/insta)**: Snapshot-only testing obscures explicit property requirements
4. **[similar-asserts](https://crates.io/crates/similar-asserts)**: Similar to pretty_assertions but with more complexity
5. **[treediff](https://crates.io/crates/treediff)**: Too low-level, requires significant setup for basic testing needs

## Consequences

### Positive
- Detailed tree structure diffs show exactly where parsing differences occur
- YAML snapshots make complex tree structures human-readable in tests
- `rstest` parameterization eliminates boilerplate for parser testing
- Leverages battle-tested, widely-adopted Rust testing ecosystem
- Interactive snapshot review with `cargo insta review`

### Negative  
- Adds external snapshot files to repository
- Snapshot tests require discipline to review changes meaningfully
- Two different assertion paradigms to learn (snapshots + traditional)

### Neutral
- Incremental adoption - can add to existing tests gradually
- Builds on standard Rust testing foundation rather than wholesale replacement