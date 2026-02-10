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

## Addendum: Parser structure, nesting model, and test strategy

This addendum records clarifications and refinements made after the initial ADR decision, based on deeper analysis of planned syntax, inline edge cases, code clarity and long-term maintainability.

### Parser structure

The MDNX parser will be implemented as a **pure Rust, lossless, span-based parser** operating directly over `xi_rope::Rope`. The rope remains the single source of truth; all parsed structures store byte spans into the rope rather than reconstructed text.

#### Block parsing model

Block parsing is implemented using a **two-phase approach**:

1. **Line classification**
   Each line is classified independently into a lightweight `LineClass` containing only local facts, such as:

   * indentation level
   * blockquote prefix depth (and remainder after stripping `>` prefixes)
   * list marker detection (and remainder after stripping marker + indent)
   * fenced code open/close detection
   * blank / non-blank

2. **Block construction via container stack**
   A container stack (`Vec<ContainerFrame>`) is maintained while iterating lines. Containers represent structural wrappers such as:

   * blockquotes
   * lists
   * list items

   For each line:

   * container prefixes are compared against the current stack
   * containers are pushed or popped to match the line’s structure
   * the remaining content is used to open or continue a **leaf block** (paragraph, fenced code, heading, etc.)

This model supports **arbitrarily deep nesting** (e.g. lists inside blockquotes inside lists) without combinatorial special cases. Nesting depth is unbounded except by memory.

Fenced code blocks are treated as **absolute raw leaf blocks**: once entered, no further block or inline parsing occurs until the matching fence is closed.

#### Inline parsing model

Inline parsing is **deliberately separate from block parsing** and is applied only to inline-eligible leaf blocks (paragraphs, headings, list item text, blockquote text).

Inline parsing operates over the full block content span as a single character stream (including newlines), which allows correct handling of constructs such as multi-line link labels.

Inline parsing is implemented using a **cursor-based, mode-driven parser** (not a parser generator), with explicit “raw zones”:

* normal inline parsing
* code spans (no parsing inside)
* MDNX wikilinks and related constructs

This separation avoids mixing container logic with character-level parsing and prevents exponential edge-case growth.

### Losslessness and rope integration

* All parse results store byte spans into the rope.
* Rendering and UI use rope slicing; Markdown is not regenerated for display.
* Parsed spans are valid only for the snapshot they were derived from.
* On edits, impacted regions (typically whole blocks) are reparsed; unchanged spans are discarded rather than transformed.
* This aligns with the existing ADR editing model using `xi_rope::Delta`.

### Test strategy (“tests are the spec”)

MDNX adopts **snapshot testing** as the primary specification mechanism for parsing behavior.

* Markdown input files are stored as fixtures.
* Each fixture is parsed and normalized into a stable, human-readable structure containing:

  * block kinds and spans
  * inline node kinds and spans
  * key sub-spans (e.g. wikilink target / alias)
  * optional rope-sliced text previews for readability
* The normalized structure is snapshot-asserted (e.g. using `insta`).

In addition to snapshots, invariant tests assert:

* all spans are within rope bounds
* child spans are contained within parent spans
* spans are ordered and non-overlapping where expected
* raw zones (e.g. fenced code) never produce inline nodes
* the parser does not panic on arbitrary input

This approach keeps the implementation flexible while ensuring correctness, regressions are visible, and behavior is precisely defined by executable tests rather than prose specifications.

### Non-goals

* No external grammar DSL or parser generator.
* No separate formal syntax specification beyond tests.
* No requirement for WYSIWYG-style fine-grained incremental tree mutation.

## Addendum: Normative parsing examples (non-exhaustive)

This section provides **worked examples** that define the intended behavior of the MDNX parser. These examples are authoritative and are expected to be covered by snapshot tests. Where implementation choices are ambiguous, the behavior demonstrated here should be followed.

### Example 1: Nested blockquote with fenced code block

**Input:**

````md
> foo
> ```rust
> const x = 1;
> ```
> bar
````

**Expected block structure:**

* One top-level `BlockQuote`

  * Contains:

    * `Paragraph` spanning `foo`
    * `FencedCodeBlock` spanning `rust … `
    * `Paragraph` spanning `bar`

**Key rules illustrated:**

* Blockquotes are containers; fenced code blocks are leaf blocks.
* Fence open/close is detected *after* stripping blockquote prefixes.
* While inside a fenced code block, no further block or inline parsing occurs.
* The fenced code block remains nested inside the blockquote container.

---

### Example 2: Deeply nested containers (Logseq-style)

**Input:**

````md
- a
  - b
    > c
    > - d
    >   ```txt
    >   e
    >   ```
````

**Expected structure (simplified):**

* List

  * ListItem “a”

    * List

      * ListItem “b”

        * BlockQuote

          * Paragraph “c”
          * List

            * ListItem “d”

              * FencedCodeBlock “e”

**Key rules illustrated:**

* Containers are managed via a stack; depth is unbounded.
* Lists, blockquotes, and fenced code blocks can nest arbitrarily.
* No special-case logic exists for specific nesting combinations.
* Structure is derived mechanically from container prefixes and indentation.

---

### Example 3: Multiline inline link inside a paragraph

**Input:**

```md
This is a [multiline
inline
link](https://example.org).
```

**Expected block structure:**

* One `Paragraph` block spanning all three lines.

**Expected inline structure (within the paragraph):**

* Text “This is a ”
* Link:

  * full span covers `[multiline\ninline\nlink](https://example.org)`
  * label span covers `multiline\ninline\nlink`
  * destination span covers `https://example.org`
* Text “.”

**Key rules illustrated:**

* Paragraphs may span multiple lines.
* Inline parsing operates over the full paragraph span, including newlines.
* Inline constructs may be multiline if their delimiters allow it.
* Block parsing does not need to understand inline delimiters.

---

### Example 4: Wikilinks and raw zones

**Input:**

```md
Text [[target|alias]] and `[[not a link]]`.
```

**Expected inline structure:**

* Text “Text ”
* WikiLink:

  * full span `[[target|alias]]`
  * target span `target`
  * alias span `alias`
* Text “ and ”
* CodeSpan:

  * full span `` `[[not a link]]` ``
  * no inline parsing inside
* Text “.”

**Key rules illustrated:**

* Wikilinks are inline constructs parsed only in inline-eligible regions.
* Code spans are raw zones and suppress all inline parsing inside them.
* Raw zones take precedence over other inline constructs.

---

### Example 5: Lossless span behavior

**Input:**

```md
Hello [[world]]!
```

**Expected invariant:**

* Every parsed node stores only byte spans into the rope.
* Slicing the rope with any node’s span reproduces the exact source text.
* No Markdown is regenerated during rendering.

**Key rules illustrated:**

* The rope is the source of truth.
* Parsing is lossless and reversible by slicing.
* Rendering and UI operate on spans, not reconstructed strings.

---

### Example 6: Editing and reparsing scope

**Scenario:**
User edits text inside a paragraph containing inline nodes.

**Expected behavior:**

* The edit produces a `Delta` applied to the rope.
* The containing block (paragraph) is reparsed in full.
* Inline parsing is rerun only for that block.
* Other blocks are discarded or reused as appropriate; no attempt is made to surgically update inline nodes.

**Key rules illustrated:**

* Block-level reparsing is the unit of incrementality.
* Fine-grained inline node mutation is explicitly out of scope.

---

### Guidance for implementation and testing

* Each example above should correspond to one or more snapshot fixtures.
* Snapshot output should assert:

  * block kinds and spans
  * inline kinds and spans
  * key sub-spans (e.g. wikilink target/alias)
* If an implementation choice conflicts with an example in this section, the example takes precedence.

---

## Addendum: Implementation guidance for parser code (normative)

This section provides **specific guidance on how the parser code should be structured and written**. Deviations should be intentional and justified.

### Overall architecture

* Implement a **pure Rust, lossless parser** over `xi_rope::Rope`.
* The rope is the **single source of truth**. Parsed structures store **byte spans only**.
* Parsing is split into:

  1. **Block parsing** (line/container based)
  2. **Inline parsing** (character based, inside inline-eligible blocks)
* Incrementality is achieved by **reparsing whole blocks**, not by mutating fine-grained trees.

### Knowledge ownership (critical)

* **All syntax knowledge (delimiters, prefixes, markers) MUST live with the type that represents it.**
* Do **not** embed magic strings or characters (`">"`, `"```"`, `"[[")`) inside generic classifier or builder logic.

#### Required pattern

* Each block or inline construct has a dedicated type/module that owns:

  * its delimiter characters/strings (as `const`s)
  * helper functions for recognizing its opening/closing form

Example intent (not exact code):

* `BlockQuote::PREFIX = '>'`
* `CodeFence::{ BACKTICKS = "```", TILDES = "~~~" }`
* `WikiLink::{ OPEN = "[[", CLOSE = "]]", ALIAS = '|' }`

Classifier/builder/parser code **must call these helpers**, never re-implement checks.

### Block parsing rules

* Block parsing uses a **two-phase approach**:

  1. **Line classification** (local facts only)
  2. **Block construction** using a **container stack**
* Containers (e.g. blockquotes, lists) are represented by a stack (`Vec<ContainerFrame>`).
* Nesting depth is unbounded; no special-case logic for specific combinations.
* Fenced code blocks are **leaf blocks and raw zones**:

  * once entered, no inline or block parsing occurs until closed
* Block openers are detected via a **single dispatch point** (e.g. `try_open_leaf`), which delegates to block-type helpers.

### Inline parsing rules

* Inline parsing is **only** applied to inline-eligible block content spans.
* Inline parsing operates over the **entire block content span**, including newlines.
* Inline parser is cursor-based with explicit “raw zones”:

  * code spans suppress all other inline parsing
* Inline constructs (wikilinks, code spans, etc.) must own their delimiters and parsing helpers.

### Separation of concerns (do not blur)

* Classifier: *what does this line look like?*
  (indentation, container prefixes, remainder span, possible block open signals)
* Block builder: *how do blocks/containers open, continue, and close?*
* Inline parser: *what inline nodes exist inside a given span?*
* Snapshot/normalization: *how do we expose a stable, testable view of the parse?*

### Text regeneration (future-proofing)

* Even though current rendering is span-based, **syntax ownership must support future structure→text operations**.
* Any future formatter/rewriter must be able to reuse the same delimiter definitions.
* Do not hardcode textual representations outside the owning type.

### Testing requirements

* Parsing behavior is defined by **snapshot tests**:

  * Markdown in → normalized parsed structure out
* Snapshots assert:

  * block kinds and spans
  * inline kinds and spans
  * key sub-spans (e.g. wikilink target/alias)
* Invariants must be enforced in tests:

  * spans are within rope bounds
  * child spans are contained within parent spans
  * raw zones never produce inline nodes
* Tests are the specification; no separate formal grammar is required.

### Non-goals (explicit)

* No parser generators or grammar DSLs.
* No JS/C toolchains.
* No fine-grained incremental tree mutation.
* No requirement to fully match CommonMark edge semantics where they conflict with clarity or editor needs.

---

## Suggested folder layout (engine)

```text
crates/markdown-neuraxis-engine/src/parsing/
  mod.rs

  rope/
    mod.rs
    span.rs
    slice.rs
    lines.rs

  blocks/
    mod.rs
    types.rs
    containers.rs
    classify.rs
    open.rs
    builder.rs
    kinds/
      mod.rs
      block_quote.rs
      code_fence.rs
      paragraph.rs

  inline/
    mod.rs
    types.rs
    cursor.rs
    parser.rs
    kinds/
      mod.rs
      code_span.rs
      wikilink.rs

  snapshot/
    mod.rs
    normalize.rs
    invariants.rs
```

---

## Rope primitives

### `rope/span.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn len(self) -> usize { self.end.saturating_sub(self.start) }
}
```

### `rope/slice.rs`

```rust
use xi_rope::Rope;

use super::span::Span;

pub fn slice_to_string(rope: &Rope, sp: Span) -> String {
    rope.slice_to_string(sp.start..sp.end)
}

pub fn preview(rope: &Rope, sp: Span, max: usize) -> String {
    let mut s = slice_to_string(rope, sp);
    if s.len() > max {
        s.truncate(max);
        s.push_str("…");
    }
    s
}
```

### `rope/lines.rs` (scaffold; replace with zero-copy later)

```rust
use xi_rope::Rope;

use super::span::Span;

#[derive(Debug, Clone)]
pub struct LineRef {
    pub span: Span,   // includes newline if present
    pub text: String, // scaffold
}

pub trait LineSource {
    fn lines(&self) -> Box<dyn Iterator<Item = LineRef> + '_>;
}

impl LineSource for Rope {
    fn lines(&self) -> Box<dyn Iterator<Item = LineRef> + '_> {
        let s = self.to_string();
        let mut offset = 0usize;
        Box::new(s.split_inclusive('\n').map(move |line| {
            let start = offset;
            offset += line.len();
            LineRef { span: Span { start, end: offset }, text: line.to_string() }
        }))
    }
}
```

---

## Block kinds own syntax (no magic strings elsewhere)

### `blocks/kinds/block_quote.rs`

```rust
pub struct BlockQuote;

impl BlockQuote {
    pub const PREFIX: char = '>';

    /// Returns (depth, byte index into `s` after stripping prefixes).
    /// Intentionally small and self-contained.
    pub fn strip_prefixes(s: &str) -> (u8, usize) {
        let b = s.as_bytes();
        let mut i = 0usize;
        let mut depth = 0u8;

        loop {
            while i < b.len() && b[i] == b' ' { i += 1; }
            if i < b.len() && b[i] == (Self::PREFIX as u8) {
                depth = depth.saturating_add(1);
                i += 1;
                if i < b.len() && b[i] == b' ' { i += 1; }
            } else {
                break;
            }
        }
        (depth, i)
    }
}
```

### `blocks/kinds/code_fence.rs`

````rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceSig { Backticks, Tildes }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceKind { Backticks, Tildes }

pub struct CodeFence;

impl CodeFence {
    pub const BACKTICKS: &'static str = "```";
    pub const TILDES: &'static str = "~~~";

    pub fn sig(remainder: &str) -> Option<FenceSig> {
        let t = remainder.trim_end_matches(['\r', '\n']);
        if t.starts_with(Self::BACKTICKS) {
            Some(FenceSig::Backticks)
        } else if t.starts_with(Self::TILDES) {
            Some(FenceSig::Tildes)
        } else {
            None
        }
    }

    pub fn kind(sig: FenceSig) -> FenceKind {
        match sig {
            FenceSig::Backticks => FenceKind::Backticks,
            FenceSig::Tildes => FenceKind::Tildes,
        }
    }

    pub fn closes(kind: FenceKind, sig: Option<FenceSig>) -> bool {
        match (kind, sig) {
            (FenceKind::Backticks, Some(FenceSig::Backticks)) => true,
            (FenceKind::Tildes, Some(FenceSig::Tildes)) => true,
            _ => false,
        }
    }
}
````

### `blocks/kinds/paragraph.rs`

```rust
pub struct Paragraph;
// No delimiters; it’s the default leaf when nothing else opens.
```

### `blocks/kinds/mod.rs`

```rust
pub mod block_quote;
pub mod code_fence;
pub mod paragraph;

pub use block_quote::BlockQuote;
pub use code_fence::{CodeFence, FenceKind, FenceSig};
pub use paragraph::Paragraph;
```

---

## Blocks: types, containers, classification, opener dispatch, builder

### `blocks/types.rs`

```rust
use crate::parsing::rope::span::Span;

use super::kinds::FenceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerFrame {
    BlockQuote { depth: u8 },
    // Later: List, ListItem, etc.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    FencedCode { kind: FenceKind },
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub containers: Vec<ContainerFrame>,
    pub kind: BlockKind,
    pub span: Span,
    pub content_span: Span, // what inline parser should see
}
```

### `blocks/containers.rs`

```rust
use super::types::ContainerFrame;

#[derive(Debug, Default, Clone)]
pub struct ContainerPath(pub Vec<ContainerFrame>);

impl ContainerPath {
    pub fn set_blockquote_depth(&mut self, depth: u8) {
        self.0.retain(|f| !matches!(f, ContainerFrame::BlockQuote { .. }));
        if depth > 0 {
            self.0.push(ContainerFrame::BlockQuote { depth });
        }
    }
}
```

### `blocks/classify.rs`

```rust
use crate::parsing::rope::{lines::LineRef, span::Span};

use super::kinds::{BlockQuote, CodeFence, FenceSig};

#[derive(Debug, Clone)]
pub struct LineClass {
    pub line: Span,
    pub is_blank: bool,

    pub quote_depth: u8,
    pub remainder_span: Span,     // bytes in rope after stripping quote prefixes
    pub remainder_text: String,   // scaffold: remainder string

    pub fence_sig: Option<FenceSig>, // “looks like a fence” on remainder
}

pub struct MarkdownLineClassifier;

impl MarkdownLineClassifier {
    pub fn classify(&self, lr: &LineRef) -> LineClass {
        let trimmed = lr.text.trim_end_matches(['\r','\n']);
        let is_blank = trimmed.trim().is_empty();

        let (qd, idx) = BlockQuote::strip_prefixes(trimmed);
        let remainder = &trimmed[idx..];
        let remainder_span = Span { start: lr.span.start + idx, end: lr.span.end };

        LineClass {
            line: lr.span,
            is_blank,
            quote_depth: qd,
            remainder_span,
            remainder_text: remainder.to_string(),
            fence_sig: CodeFence::sig(remainder),
        }
    }
}
```

### `blocks/open.rs` (single, explicit precedence point)

```rust
use super::kinds::{CodeFence, FenceKind, FenceSig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOpen {
    FencedCode { kind: FenceKind },
    // Later: Heading, ThematicBreak, HtmlBlock...
}

pub fn try_open_leaf(remainder: &str) -> Option<BlockOpen> {
    // Precedence: fence beats everything else.
    if let Some(sig) = CodeFence::sig(remainder) {
        return Some(BlockOpen::FencedCode { kind: CodeFence::kind(sig) });
    }
    None
}
```

### `blocks/builder.rs` (small methods; patterns pop)

```rust
use crate::parsing::rope::span::Span;

use super::{
    classify::LineClass,
    containers::ContainerPath,
    kinds::CodeFence,
    open::{try_open_leaf, BlockOpen},
    types::{BlockKind, BlockNode, ContainerFrame},
};

#[derive(Debug, Clone, Copy)]
enum LeafState {
    None,
    Paragraph { start: Span, content_start: Span },
    Fence { kind: super::kinds::FenceKind, start: Span },
}

pub struct BlockBuilder {
    containers: ContainerPath,
    leaf: LeafState,
    out: Vec<BlockNode>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self { containers: ContainerPath::default(), leaf: LeafState::None, out: vec![] }
    }

    pub fn push(&mut self, c: &LineClass) {
        self.containers.set_blockquote_depth(c.quote_depth);

        if self.in_fence() {
            self.consume_fence_line(c);
            return;
        }

        if c.is_blank {
            self.flush_paragraph(c.line);
            return;
        }

        if let Some(open) = try_open_leaf(&c.remainder_text) {
            self.flush_paragraph(c.line);
            self.open_leaf(open, c.line);
            return;
        }

        self.extend_paragraph(c.line, c.remainder_span);
    }

    pub fn finish(mut self, end: Span) -> Vec<BlockNode> {
        // EOF flush
        self.flush_paragraph(end);
        // NOTE: in a real impl, also flush an unterminated fence as a fence block.
        self.out
    }

    fn in_fence(&self) -> bool {
        matches!(self.leaf, LeafState::Fence { .. })
    }

    fn open_leaf(&mut self, open: BlockOpen, line: Span) {
        match open {
            BlockOpen::FencedCode { kind } => self.leaf = LeafState::Fence { kind, start: line },
        }
    }

    fn consume_fence_line(&mut self, c: &LineClass) {
        let (kind, start) = match self.leaf {
            LeafState::Fence { kind, start } => (kind, start),
            _ => return,
        };

        // Close if this line “looks like fence” with same sig.
        if CodeFence::closes(kind, c.fence_sig) {
            self.out.push(BlockNode {
                containers: self.containers.0.clone(),
                kind: BlockKind::FencedCode { kind },
                span: Span { start: start.start, end: c.line.end },
                content_span: Span { start: start.start, end: c.line.end }, // refine later
            });
            self.leaf = LeafState::None;
        }
        // else: remain in fence; span finalised on close (or EOF flush)
    }

    fn extend_paragraph(&mut self, line: Span, content_start: Span) {
        match self.leaf {
            LeafState::Paragraph { .. } => {
                // continue; span finalised on flush
            }
            _ => {
                self.leaf = LeafState::Paragraph { start: line, content_start };
            }
        }
    }

    fn flush_paragraph(&mut self, end: Span) {
        let prev = std::mem::replace(&mut self.leaf, LeafState::None);
        if let LeafState::Paragraph { start, content_start } = prev {
            self.out.push(BlockNode {
                containers: self.containers.0.clone(),
                kind: BlockKind::Paragraph,
                span: Span { start: start.start, end: end.end },
                content_span: Span { start: content_start.start, end: end.end },
            });
        } else {
            self.leaf = prev; // put back non-paragraph leaf (e.g. fence)
        }
    }
}
```

### `blocks/mod.rs`

```rust
pub mod kinds;
pub mod types;
pub mod containers;
pub mod classify;
pub mod open;
pub mod builder;

pub use builder::BlockBuilder;
pub use classify::{LineClass, MarkdownLineClassifier};
pub use types::{BlockKind, BlockNode, ContainerFrame};
```

---

## Inline kinds own syntax + parser uses them (no magic strings)

### `inline/kinds/wikilink.rs`

```rust
pub struct WikiLink;

impl WikiLink {
    pub const OPEN: &[u8; 2] = b"[[";
    pub const CLOSE: &[u8; 2] = b"]]";
    pub const ALIAS: u8 = b'|';
}
```

### `inline/kinds/code_span.rs`

```rust
pub struct CodeSpan;

impl CodeSpan {
    pub const TICK: u8 = b'`';
}
```

### `inline/kinds/mod.rs`

```rust
pub mod wikilink;
pub mod code_span;

pub use wikilink::WikiLink;
pub use code_span::CodeSpan;
```

### `inline/types.rs`

```rust
use crate::parsing::rope::span::Span;

#[derive(Debug, Clone)]
pub enum InlineNode {
    Text(Span),
    CodeSpan { full: Span, inner: Span },
    WikiLink { full: Span, target: Span, alias: Option<Span> },
}
```

### `inline/cursor.rs`

```rust
#[derive(Clone)]
pub struct Cursor<'a> {
    pub s: &'a str,
    pub base: usize,
    pub i: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(s: &'a str, base: usize) -> Self { Self { s, base, i: 0 } }
    pub fn pos(&self) -> usize { self.base + self.i }
    pub fn eof(&self) -> bool { self.i >= self.s.len() }
    pub fn peek(&self) -> Option<u8> { self.s.as_bytes().get(self.i).copied() }
    pub fn starts_with(&self, pat: &[u8]) -> bool {
        self.s.as_bytes()[self.i..].starts_with(pat)
    }
    pub fn bump(&mut self) -> Option<u8> {
        let b = self.s.as_bytes().get(self.i).copied()?;
        self.i += 1;
        Some(b)
    }
    pub fn bump_n(&mut self, n: usize) { self.i += n; }
}
```

### `inline/parser.rs` (composed `try_parse_*`, short + readable)

```rust
use crate::parsing::rope::span::Span;

use super::{cursor::Cursor, kinds::{CodeSpan, WikiLink}, types::InlineNode};

pub fn parse_inline(base: usize, s: &str) -> Vec<InlineNode> {
    let mut cur = Cursor::new(s, base);
    let mut out = vec![];
    let mut text_start = cur.pos();

    fn flush_text(out: &mut Vec<InlineNode>, start: usize, end: usize) {
        if end > start { out.push(InlineNode::Text(Span { start, end })); }
    }

    while !cur.eof() {
        if let Some(node) = try_parse_code_span(&mut cur) {
            flush_text(&mut out, text_start, span_of(&node).start);
            text_start = span_of(&node).end;
            out.push(node);
            continue;
        }
        if let Some(node) = try_parse_wikilink(&mut cur) {
            flush_text(&mut out, text_start, span_of(&node).start);
            text_start = span_of(&node).end;
            out.push(node);
            continue;
        }
        cur.bump();
    }

    flush_text(&mut out, text_start, cur.pos());
    out
}

fn span_of(n: &InlineNode) -> Span {
    match n {
        InlineNode::Text(sp) => *sp,
        InlineNode::CodeSpan { full, .. } => *full,
        InlineNode::WikiLink { full, .. } => *full,
    }
}

fn try_parse_code_span(cur: &mut Cursor<'_>) -> Option<InlineNode> {
    if cur.peek() != Some(CodeSpan::TICK) { return None; }

    let start = cur.pos();
    cur.bump(); // `
    let inner_start = cur.pos();

    while !cur.eof() {
        if cur.peek() == Some(CodeSpan::TICK) { break; }
        cur.bump();
    }
    let inner_end = cur.pos();

    if cur.peek() != Some(CodeSpan::TICK) { return None; }
    cur.bump(); // closing `
    let end = cur.pos();

    Some(InlineNode::CodeSpan {
        full: Span { start, end },
        inner: Span { start: inner_start, end: inner_end },
    })
}

fn try_parse_wikilink(cur: &mut Cursor<'_>) -> Option<InlineNode> {
    if !cur.starts_with(WikiLink::OPEN) { return None; }

    let start = cur.pos();
    cur.bump_n(WikiLink::OPEN.len());
    let target_start = cur.pos();

    while !cur.eof() {
        if cur.peek() == Some(WikiLink::ALIAS) { break; }
        if cur.starts_with(WikiLink::CLOSE) { break; }
        cur.bump();
    }
    let target_end = cur.pos();

    let mut alias = None;
    if cur.peek() == Some(WikiLink::ALIAS) {
        cur.bump(); // |
        let alias_start = cur.pos();
        while !cur.eof() {
            if cur.starts_with(WikiLink::CLOSE) { break; }
            cur.bump();
        }
        let alias_end = cur.pos();
        alias = Some(Span { start: alias_start, end: alias_end });
    }

    if !cur.starts_with(WikiLink::CLOSE) { return None; }
    cur.bump_n(WikiLink::CLOSE.len());
    let end = cur.pos();

    Some(InlineNode::WikiLink {
        full: Span { start, end },
        target: Span { start: target_start, end: target_end },
        alias,
    })
}
```

### `inline/mod.rs`

```rust
pub mod kinds;
pub mod types;
pub mod cursor;
pub mod parser;

pub use types::InlineNode;
pub use parser::parse_inline;
```

---

## Top-level parse function (glue only)

### `parsing/mod.rs`

```rust
pub mod rope;
pub mod blocks;
pub mod inline;
pub mod snapshot;

use xi_rope::Rope;

use rope::{lines::LineSource, span::Span, slice::slice_to_string};

#[derive(Debug)]
pub struct ParsedDoc {
    pub blocks: Vec<blocks::BlockNode>,
}

pub fn parse_document(rope: &Rope) -> ParsedDoc {
    let classifier = blocks::MarkdownLineClassifier;
    let mut builder = blocks::BlockBuilder::new();

    for lr in rope.lines() {
        let lc = classifier.classify(&lr);
        builder.push(&lc);
    }

    let end = Span { start: rope.len(), end: rope.len() };
    ParsedDoc { blocks: builder.finish(end) }
}

/// Convenience: inline parse for a given block node (paragraphs only in this skeleton).
pub fn parse_inline_for_block(rope: &Rope, b: &blocks::BlockNode) -> Vec<inline::InlineNode> {
    if !matches!(b.kind, blocks::BlockKind::Paragraph) {
        return vec![];
    }
    let s = slice_to_string(rope, b.content_span);
    inline::parse_inline(b.content_span.start, &s)
}
```

---

## Snapshot testing: assert structure, not internals

### `snapshot/normalize.rs`

```rust
use std::collections::BTreeMap;

use xi_rope::Rope;

use crate::parsing::{
    blocks::{BlockKind, BlockNode, ContainerFrame},
    rope::{span::Span, slice::preview},
    parse_inline_for_block,
};

#[derive(serde::Serialize)]
pub struct Snap {
    pub blocks: Vec<BlockSnap>,
}

#[derive(serde::Serialize)]
pub struct BlockSnap {
    pub kind: String,
    pub span: (usize, usize),
    pub containers: Vec<String>,
    pub text: String,
    pub inline: Vec<InlineSnap>,
}

#[derive(serde::Serialize)]
pub struct InlineSnap {
    pub kind: String,
    pub span: (usize, usize),
    pub text: String,
    pub parts: BTreeMap<String, (usize, usize)>,
}

pub fn normalize(rope: &Rope, blocks: &[BlockNode]) -> Snap {
    let blocks = blocks.iter().map(|b| {
        let kind = match &b.kind {
            BlockKind::Paragraph => "Paragraph".to_string(),
            BlockKind::FencedCode { kind } => format!("FencedCode({kind:?})"),
        };

        let containers = b.containers.iter().map(|c| match c {
            ContainerFrame::BlockQuote { depth } => format!("Quote({depth})"),
        }).collect::<Vec<_>>();

        let inline_nodes = parse_inline_for_block(rope, b);
        let inline = inline_nodes.into_iter().map(|n| {
            let mut parts = BTreeMap::new();
            match n {
                crate::parsing::inline::InlineNode::Text(sp) => InlineSnap {
                    kind: "Text".into(),
                    span: (sp.start, sp.end),
                    text: preview(rope, sp, 60),
                    parts,
                },
                crate::parsing::inline::InlineNode::CodeSpan { full, inner } => {
                    parts.insert("inner".into(), (inner.start, inner.end));
                    InlineSnap {
                        kind: "CodeSpan".into(),
                        span: (full.start, full.end),
                        text: preview(rope, full, 60),
                        parts,
                    }
                }
                crate::parsing::inline::InlineNode::WikiLink { full, target, alias } => {
                    parts.insert("target".into(), (target.start, target.end));
                    if let Some(a) = alias {
                        parts.insert("alias".into(), (a.start, a.end));
                    }
                    InlineSnap {
                        kind: "WikiLink".into(),
                        span: (full.start, full.end),
                        text: preview(rope, full, 60),
                        parts,
                    }
                }
            }
        }).collect();

        BlockSnap {
            kind,
            span: (b.span.start, b.span.end),
            containers,
            text: preview(rope, b.span, 80),
            inline,
        }
    }).collect();

    Snap { blocks }
}
```

### `snapshot/invariants.rs`

```rust
use xi_rope::Rope;

use crate::parsing::blocks::{BlockKind, BlockNode};

pub fn check(rope: &Rope, blocks: &[BlockNode]) {
    let n = rope.len();
    for b in blocks {
        assert!(b.span.start <= b.span.end && b.span.end <= n);
        assert!(b.content_span.start <= b.content_span.end && b.content_span.end <= n);
        assert!(b.content_span.start >= b.span.start && b.content_span.end <= b.span.end);

        // Raw zone rule: fenced code should not produce inline nodes (enforced by caller).
        if matches!(b.kind, BlockKind::FencedCode { .. }) {
            // nothing here; kept as a reminder invariant
        }
    }
}
```

### `snapshot/mod.rs`

```rust
pub mod normalize;
pub mod invariants;

pub use normalize::{Snap, normalize};
pub use invariants::check as invariants;
```

### Test harness example

```rust
// crates/markdown-neuraxis-engine/src/tests/parsing_snapshots.rs (or tests/)
use markdown_neuraxis_engine::parsing::{parse_document, snapshot};

#[test]
fn fixture_nested_quote_fence() {
    assert_fixture("combos/nested_quote_fence");
}

fn assert_fixture(name: &str) {
    let md = std::fs::read_to_string(format!("tests/fixtures/{name}.md")).unwrap();
    let rope = xi_rope::Rope::from(md.as_str());

    let doc = parse_document(&rope);
    snapshot::invariants(&rope, &doc.blocks);

    let snap = snapshot::normalize(&rope, &doc.blocks);
    insta::assert_yaml_snapshot!(name, snap);
}
```

---

### Key point this demonstrates

* **Delimiter checks are only in** `blocks/kinds/*` and `inline/kinds/*`.
* Classifier/builder/parser call those helpers; they do not hardcode `"```"` etc.
* Builder is readable because it’s a small state machine with small methods.
* Snapshots assert a stable, human reviewable structure.
