# ADR 12 — Replace Tree-sitter Markdown with a Pure-Rust MDNX Parser

**Status:** Accepted
**Date:** 2026-02-09
**Scope:** Core editor parsing layer (shared by all frontends)

---

## Context

[ADR-0004](0004-editing-code-architecture.md) established `xi_rope::Rope` as the single source of truth, with edits compiled to `Delta` and a parse overlay driving snapshots/render blocks.

We encountered limitations with Tree-sitter Markdown (design explored in [ChatGPT discussion](https://chatgpt.com/g/g-p-6895277320088191a3b6b66f9374e11b-markdown-neuraxis/c/698a6321-f63c-8394-86a6-70947e313b87)):

1. **Inline extensibility** — Adding `[[wikilinks]]` and other MDNX-specific constructs as first-class nodes with precise escaping/raw behavior proved difficult
2. **Toolchain friction** — JS grammar sources and generated C code conflict with our Rust+Kotlin codebase
3. **Overkill for our UX** — MDNX's non-WYSIWYG model means we can reparse entire blocks rather than perform fine-grained AST surgery

---

## Decision

Implement a **pure-Rust MDNX parser** replacing Tree-sitter. The parser is lossless and span-based, operating directly over `xi_rope::Rope`.

### Core Principles

1. **Lossless, span-based parsing** — All parsed structures store byte spans into the rope. The rope remains the single source of truth. Slicing any node's span reproduces exact source text.

2. **Two-phase block parsing**:
   - **Line classification** — Each line is classified independently (indentation, blockquote depth, fence detection, blank/non-blank)
   - **Block construction** — A container stack maintains structural wrappers (blockquotes, lists) while emitting leaf blocks (paragraphs, fenced code)

3. **Cursor-based inline parsing** — Operates over full block content spans as a character stream with explicit "raw zones" where code spans suppress all other parsing

4. **Knowledge ownership** — All syntax delimiters live with the types that represent them (`BlockQuote::PREFIX`, `WikiLink::OPEN`, etc.). Classifier/builder/parser code calls these helpers, never hardcodes delimiters.

5. **Block-level incrementality** — On edit, only impacted blocks are reparsed. No fine-grained tree mutation.

### ContentView Projection

For nested line-prefix containers (blockquotes, lists), content is represented as per-line spans rather than a single contiguous span:

```rust
pub struct ContentLine {
    pub raw_line: Span,   // full physical line
    pub prefix: Span,     // container prefix region
    pub content: Span,    // remainder after stripping prefixes
}

pub enum ContentView {
    Contiguous(Span),        // no per-line prefix semantics
    Lines(Vec<ContentLine>), // content is non-contiguous
}
```

This enables GUI editing modes (with/without visible prefixes) and correct reconstruction when adding lines to deeply nested blocks.

### Implementation

The parser is implemented in `crates/markdown-neuraxis-engine/src/parsing/`. See the [module documentation](../../crates/markdown-neuraxis-engine/src/parsing/mod.rs) for detailed architecture and usage examples.

**Current support:**
- **Blocks:** Paragraphs, fenced code blocks (``` and ~~~), blockquotes (nested)
- **Inline:** Plain text, code spans (raw zones), wikilinks (`[[target]]` and `[[target|alias]]`)

---

## Options Considered

### A) Keep Tree-sitter; fork inline grammar

Rejected — Toolchain friction (JS+C) and ongoing grammar maintenance misaligned with "Rust-first contributor ergonomics".

### B) Use [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) or [comrak](https://comrak.ee/)

Rejected — Rendering-oriented, not editor-precise. Extending inline syntax devolves into custom scanning anyway.

### C) Adopt [Rowan](https://docs.rs/rowan)-style lossless CST

Rejected — Disproportionate complexity for MDNX's block-replace editing model.

### D) Use [logos](https://docs.rs/logos) lexer generator

Deferred — May simplify inline tokenization later, but not required for v1.

---

## Consequences

### Positive

- **Rust-only contributor experience** for the parsing layer
- **MDNX-specific precision** — Escaping rules, raw modes, wikilink semantics encoded directly
- **Predictable performance** — Reparse only changed blocks

### Negative

- We own correctness and maintenance of a "Markdown+" parser
- Must maintain robust test suite to avoid regressions

### Non-goals

- No parser generators or grammar DSLs
- No JS/C toolchains
- No fine-grained incremental tree mutation
- No requirement to fully match CommonMark edge semantics where they conflict with clarity or editor needs

---

## Testing Strategy

Parsing behavior is defined by **snapshot tests**:

- Markdown fixtures are parsed and normalized to a stable, human-readable structure
- Snapshots assert block kinds, spans, inline nodes, and key sub-spans (e.g., wikilink target/alias)
- Invariant tests verify: spans within bounds, child spans contained in parents, raw zones produce no inline nodes

See `crates/markdown-neuraxis-engine/src/parsing/tests/` for the test suite.

---

## Normative Examples

These examples define intended behavior. Implementation must match these; they are covered by snapshot tests.

### Nested blockquote with fenced code

````md
> foo
> ```rust
> const x = 1;
> ```
> bar
````

Produces: BlockQuote containing Paragraph("foo"), FencedCode, Paragraph("bar"). Fence detection occurs after stripping quote prefixes.

### Wikilinks and raw zones

```md
Text [[target|alias]] and `[[not a link]]`.
```

Produces: Text, WikiLink(target, alias), Text, CodeSpan (no wikilink inside), Text. Code spans suppress all inline parsing.

### Deep nesting (Logseq-style)

````md
- a
  - b
    > c
    > - d
    >   ```txt
    >   e
    >   ```
````

Produces: List > ListItem("a") > List > ListItem("b") > BlockQuote > Paragraph("c"), List > ListItem("d") > FencedCode("e"). Containers are managed via a stack; nesting depth is unbounded with no special-case logic for specific combinations.

### ContentView for nested blocks

For a paragraph inside a blockquote, `ContentView::Lines` stores each line's prefix (`> `) and content separately. Inline parsing operates on joined content spans. Editing "without prefix" reconstructs raw text by reattaching prefixes.

---

## References

- [ADR-0004: Editing Code Architecture](0004-editing-code-architecture.md)
- [Implementation: parsing module](../../crates/markdown-neuraxis-engine/src/parsing/mod.rs)

