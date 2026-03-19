# ADR-13: Recursive Enum for Nested Inline Formatting

Status: Proposed
Date: 2026-03-19

## Context

Markdown supports nested inline formatting: `**Bold with *nested italic* text**`. The current flat representation loses this structure:

```rust
// Current: loses nesting
enum SegmentKind {
    Strong(String),    // "Bold with *nested italic* text"
    Emphasis(String),
}
```

We need a tree structure to preserve nested formatting for accurate rendering.

## Options Considered

### Option A: Uniform struct with children field

```rust
struct InlineNode {
    kind: InlineKind,
    children: Vec<InlineNode>,
}

enum InlineKind {
    Text(String),
    Strong,
    Emphasis,
    Code(String),
}
```

Cons:
- Leaf nodes carry empty `children` vecs
- Rendering requires accessing both `kind` and `children` in every match arm
- Kind and content are separated

### Option B: Direct recursive enum

```rust
enum InlineNode {
    Text(String),
    Strong(Vec<InlineNode>),
    Emphasis(Vec<InlineNode>),
    Code(String),
}
```

Pros:
- Each variant holds exactly what it needs semantically
- Leaves hold content, containers hold children
- Clean recursive rendering:

```rust
fn render(node: &InlineNode) -> Element {
    match node {
        InlineNode::Text(s) => rsx! { "{s}" },
        InlineNode::Strong(children) => rsx! {
            strong { for c in children { {render(c)} } }
        },
        InlineNode::Emphasis(children) => rsx! {
            em { for c in children { {render(c)} } }
        },
        InlineNode::Code(s) => rsx! { code { "{s}" } },
    }
}
```

## Decision

Use Option B: direct recursive enum.

The non-uniformity (some variants hold `String`, some hold `Vec`) reflects semantic truth. This yields the cleanest rendering code and avoids carrying empty vectors on leaf nodes.

## Implementation

1. Parser: make `emphasis_or_strong()` recurse into `inline_element()` for content
2. Snapshot: replace `SegmentKind` with recursive `InlineNode` enum
3. FFI: add `children: Vec<TextSegmentDto>` to `TextSegmentDto`
4. UI: recursive `render_inline()` function
