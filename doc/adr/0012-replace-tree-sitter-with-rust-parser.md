# ADR 12 — Replace Tree-sitter Markdown overlay with a custom pure-Rust “MDNX Markdown+” parser

**Status:** Proposed  
**Date:** 2026-02-09  
**Scope:** Core editor parsing layer (shared by all frontends)

---

## Context

[ADR-0004](0004-editing-code-architecture.md) set the current direction: a single `xi_rope::Rope` buffer as source of truth, edits compiled to `Delta`, and a parse overlay using Tree-sitter Markdown (block + inline) to drive snapshots/render blocks.

In practice, we’ve hit limits with Tree-sitter Markdown inline extensibility (e.g. wanting `[[wikilinks]]` and other MDNX-specific inline constructs as first-class nodes, with precise escaping/raw behavior). The Tree-sitter approach also carries non-Rust toolchain friction (JS grammar sources, generated C, build-time feature toggles), which is especially undesirable given the project already spans Rust + Kotlin.

MDNX’s UX model (not WYSIWYG) reduces the need for “perfect incremental AST surgery”: editing is of raw text, and it’s acceptable to replace entire blocks and reparse those blocks deterministically.

Generated following [discussion with ChatGPT (private url)](https://chatgpt.com/g/g-p-6895277320088191a3b6b66f9374e11b-markdown-neuraxis/c/698a6321-f63c-8394-86a6-70947e313b87)

---

## Decision

Implement a **pure-Rust MDNX parser** and remove Tree-sitter from the core parsing overlay.

### Parsing model

1. **Block phase (line/indent aware, deterministic):**

   * Scan the rope by lines to produce block spans and block kinds (heading, paragraph, list item, code fence, etc.).
   * Emit a block list with **byte ranges** into the rope (to preserve round-trip and enable fast slicing).

2. **Inline phase (per inline-eligible block):**

   * Parse only the content ranges of blocks that allow inline syntax (paragraphs, headings, list item bodies, etc.).
   * Use a **hand-rolled inline parser** (recursive descent / Pratt-style) to implement MDNX-specific constructs precisely:

     * `[[wikilink]]` variants (aliases, embeds, etc.)
     * escapes and “raw” modes
     * clear “no-parse zones” (e.g. code spans dominate; nothing inside parses)

3. **Incrementality strategy:**

   * On edit, identify impacted blocks (cheap via line scanning around the edit).
   * Recompute block structure locally and re-run inline parse only for changed blocks.
   * Continue using `xi_rope::Delta` for selection/anchor transforms as per ADR-0004.

### Output contract

The parser produces the data needed for `Snapshot` / `RenderBlock` generation (as described in ADR-0004), but now backed by MDNX’s own block+inline structures instead of Tree-sitter nodes.

---

## Options considered

### A) Keep Tree-sitter; enable/fork inline grammar for wikilinks

Pros:

* Existing incremental parse machinery.
  Cons:
* Toolchain friction (JS grammar + generated C) remains.
* Inline grammar changes likely become nuanced and ongoing; forking means maintenance burden anyway.

Rejected because the toolchain + grammar iteration cost is misaligned with “Rust-first contributor ergonomics”.

### B) Use an existing Rust Markdown parser (pulldown-cmark / comrak)

* `pulldown-cmark` is an efficient CommonMark pull-parser emitting an event stream. ([pulldown-cmark][2])
* Comrak is a CommonMark/GFM parser+renderer with an AST and rendering-focused extension points. ([comrak.ee][3])

Pros:

* Pure Rust, mature.
  Cons:
* Primarily oriented toward rendering, not editor-precise, MDNX-specific inline semantics.
* Extending inline syntax to MDNX’s needs is awkward and tends to devolve into custom scanning anyway.

Rejected because it doesn’t provide the precise, editor-oriented control we need without significant invasive changes.

### C) Adopt a Rowan-style lossless CST (rust-analyzer approach)

* `rowan` is a generic library for lossless syntax trees. ([docs.rs][4])

Pros:

* Very strong tooling story; lossless trees.
  Cons:
* High upfront complexity; effectively building a full language infrastructure.
* Overkill given MDNX’s block-replace editing model.

Rejected as disproportionate to current needs.

### D) Use a lexer generator + hand-written parser

* `logos` provides fast, DFA-based tokenization suitable for language tooling. ([sdiehl.github.io][5])

Pros:

* Could simplify inline tokenization.
  Cons:
* Still need the parser; not a full solution by itself.

Deferred: we may use `logos` later, but it’s not required for v1.

---

## Consequences

### Positive

* **Rust-only contributor experience** for the parsing layer (no JS/C toolchain to modify grammar).
* **MDNX-specific precision**: escaping rules, raw modes, and wikilink semantics can be encoded directly and tested.
* **Predictable performance** aligned with our UX: reparse only changed blocks/inline ranges.

### Negative

* We own correctness and long-term maintenance of a “Markdown+” parser.
* Must design and maintain a robust test suite (goldens + property tests) to avoid regressions.

---

## Implementation notes

* Keep the [ADR-0004][1] edit loop intact (commands → delta → rope), and swap only the parse overlay component.
* Start with a minimal block set + minimal inline set (`code spans`, `escapes`, `wikilinks`), then expand.
* Add “nasty fixtures” early (nested lists, backticks inside wikilinks, escaped delimiters) and lock them with golden tests.

---

## Follow-ups

1. Define the MDNX inline syntax spec (wikilinks, aliasing, embed markers, escape rules, raw rules).
2. Implement v1 block scanner emitting `RenderBlock`-compatible spans.
3. Implement v1 inline parser for `[[...]]`, escapes, and code spans.
4. Replace Tree-sitter dependency in core; update snapshot generation accordingly.
5. Add golden fixtures and property tests around round-trip invariants and span correctness.

[1]: 0004-editing-code-architecture.md "raw.githubusercontent.com"
[2]: https://github.com/pulldown-cmark/pulldown-cmark "GitHub - pulldown-cmark/pulldown-cmark: An efficient, reliable parser for CommonMark, a standard dialect of Markdown"
[3]: https://comrak.ee/ "Comrak · Markdown parser and renderer"
[4]: https://docs.rs/rowan "rowan - Rust"
[5]: https://sdiehl.github.io/compiler-crates/logos.html "logos - Compiler Crates"
