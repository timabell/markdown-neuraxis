# ADR-12: Adopt Rowan + Logos as the Core Markdown+ Parser Architecture

Status: Proposed
Date: 2026-02-23
Supersedes: None (new branch; previous ADR-12 disregarded)

## Context

Markdown-Neuraxis is intended to become a high-performance, structurally aware Markdown notes outliner with:

* Arbitrarily deep nesting (lists, blockquotes, mixed containers)
* Lossless source preservation
* Structural editing
* Plugin support
* Acceptance and rendering of almost any Markdown
* Future HTML block parsing
* MDNX-specific syntax extensions (e.g. wikilinks)

Markdown is not context-free, especially around inline constructs and container interactions (see [https://clehaxze.tw/gemlog/2022/03-31-markdown-is-not-context-free.gmi](https://clehaxze.tw/gemlog/2022/03-31-markdown-is-not-context-free.gmi)). This implies:

* Mode-driven parsing
* Ambiguity handling
* Error tolerance
* Non-trivial container rules

The parser must therefore be designed as a long-term language frontend, not an ad-hoc span processor.

Rowan provides a lossless concrete syntax tree infrastructure:
[https://docs.rs/rowan/latest/rowan/](https://docs.rs/rowan/latest/rowan/)

Rust-analyzer’s syntax architecture provides the reference model:
[https://rust-analyzer.github.io/book/contributing/syntax.html](https://rust-analyzer.github.io/book/contributing/syntax.html)

Logos provides a fast lexer framework:
[https://docs.rs/logos/latest/logos/](https://docs.rs/logos/latest/logos/)

---

## Decision

We will:

1. Create a new crate: `markdown-neuraxis-syntax`
2. Use Logos for tokenization
3. Use Rowan for lossless CST construction
4. Implement a manual event-based grammar
5. Build projection layers for engine/UI
6. Support a broad Markdown surface from the start
7. Treat HTML blocks as raw leaf nodes initially
8. Design for extension and plugin traversal

This establishes a proper language frontend architecture.

---

## Crate Structure

New crate:

```
crates/markdown-neuraxis-syntax/
  src/
    lib.rs
    syntax_kind.rs
    lexer.rs
    parser/
      mod.rs
      event.rs
      sink.rs
      grammar/
        mod.rs
        root.rs
        block.rs
        inline.rs
    ast/
      mod.rs
      blocks.rs
      inline.rs
```

Engine depends on this crate and derives editor projections from the CST.

---

## SyntaxKind Definition

All tokens and nodes share a single enum:

```rust
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxKind {
    // Tokens
    WHITESPACE,
    NEWLINE,
    TEXT,
    GT,
    DASH,
    STAR,
    PLUS,
    BACKTICK,
    TILDE,
    LBRACKET,
    RBRACKET,
    PIPE,
    LPAREN,
    RPAREN,
    HASH,
    HTML_TEXT,
    EOF,

    // Nodes
    ROOT,
    BLOCK_QUOTE,
    LIST,
    LIST_ITEM,
    PARAGRAPH,
    HEADING,
    THEMATIC_BREAK,
    FENCED_CODE,
    HTML_BLOCK,
    INLINE,
    WIKILINK,
    CODE_SPAN,
    LINK,
    EMPHASIS,
    STRONG,

    ERROR,
}
```

Every byte in the source must appear as a token in the tree. No implicit reconstruction.

---

## Lexer (Logos)

The lexer must:

* Preserve whitespace
* Preserve newlines
* Emit container prefix tokens (`>`, list markers, indentation)
* Emit raw HTML as tokens
* Never discard trivia

Example skeleton:

````rust
#[derive(Logos, Debug, Clone, PartialEq)]
pub enum TokenKind {
    #[regex(r"[ \t]+")]
    Whitespace,

    #[token("\n")]
    Newline,

    #[token(">")]
    Gt,

    #[token("-")]
    Dash,

    #[token("```")]
    TripleBacktick,

    #[regex(r"[^\s\[\]()`*>#-]+")]
    Text,
}
````

The actual implementation will be more detailed and must cover all required token categories.

---

## Parser Architecture

Parser will follow the rust-analyzer event model:

* `start_node(kind)`
* `token(kind)`
* `finish_node()`

Grammar modules:

* `root.rs`
* `block.rs`
* `inline.rs`

Block parsing responsibilities:

* Container recognition (blockquote, lists)
* Nested container stacking
* Leaf block detection (paragraphs, headings, fences)
* HTML block detection
* Error tolerance

Inline parsing responsibilities:

* Code spans
* Wikilinks
* Standard links
* Emphasis / strong
* Nested inline structures
* Multiline link labels
* Fallback to text on malformed constructs

Fenced code and HTML blocks suppress inline parsing.

---

## Supported Markdown Surface (Initial Scope)

### Containers

* Blockquotes (`>`)
* Nested blockquotes
* Unordered lists (`-`, `*`, `+`)
* Nested lists (arbitrary depth)
* Blockquote inside list
* List inside blockquote

### Leaf Blocks

* Paragraph
* ATX headings (`#`)
* Thematic break (`---`, `***`)
* Fenced code blocks (``` and ~~~)
* HTML blocks (raw)

### Inline

* Code spans
* Wikilinks (`[[target|alias]]`)
* Standard links `[text](url)`
* Emphasis `*em*`
* Strong `**strong**`
* Nested inline constructs
* Multiline link labels

---

## HTML Block Handling

Initially:

* Detect HTML block start according to CommonMark-style heuristics
* Treat as raw leaf node
* Preserve all internal tokens

Later:

* Optional HTML subtree parsing
* Possible integration with HTML parser

---

## Projection Layer (Engine-Facing)

The engine must not manipulate Rowan internals directly.

Instead:

* Derive outline blocks
* Derive prefix-aware line views
* Provide content-without-prefix views
* Provide raw spans for replacement edits

Prefix stripping becomes a projection over token sequences, not primary parser output.

---

## Editing Model

Two editing modes:

1. Raw span replacement (replace subtree span directly)
2. Prefix-aware editing (derive content view, reconstruct prefixes on write-back)

Rowan enables subtree replacement safely.

Write-back of prefix-aware edits must:

* Preserve container tokens
* Reapply appropriate prefix tokens per line
* Replace only the intended subtree

---

## Required Test Coverage

### Snapshot Parsing Tests

* Deeply nested lists with blockquotes
* Fenced code inside nested blockquotes
* Multiline inline links
* Wikilinks inside lists
* Raw HTML blocks
* Malformed emphasis
* Unclosed fences
* Mixed nesting

### Invariant Tests

* Every byte in input appears in tree
* Tree reserialization equals original input
* Raw zones contain no inline nodes
* Nested containers correctly structured

### Round-Trip Tests

* Parse → serialize → parse equality
* Prefix-aware edit reconstruction correctness
* Subtree replacement preserves surrounding structure

### Fuzz Testing

* Random markdown input does not panic
* Tree remains structurally valid

---

## Why This Design

* Lossless CST guarantees correctness.
* Clear separation between syntax and projection layers.
* Plugins can traverse structured nodes instead of reverse-engineering spans.
* HTML blocks integrate naturally as raw nodes.
* Deep nesting scales structurally.
* Error tolerance is inherent to CST-based architecture.
* Future extensions are incremental.

This design prioritizes long-term structural integrity over short-term minimalism.
