# ADR 4 — Editor Core & UI Architecture for the next iteration

**Status:** Accepted
**Date:** 2025-08-31
**Scope:** Desktop/Web (Dioxus) first; future TUI (Ratatui)

---

## Context

The exising editing approach is showing its weaknesses, an attempt to add bullet editing failed miserably due to poor abstractions. This ADR is the result of a discussion with ChatGPT about a proper approach to building such an editor. Full conversation: <https://chatgpt.com/share/68b44769-14ec-8006-8655-48be654cbe30>

We’re building a Markdown outliner with high quality UX and exact round-trip fidelity. The current prototype gets tangled because UI concerns leak into the model. We want:

* **Lossless** persistence (no formatting drift; byte-for-byte round-trip).
* **Single source of truth** for the entire file (no re-rendering from an abstract model).
* **Fast edits** that scale to large docs.
* **Clean, testable core** with a tiny API that multiple frontends can share (Dioxus now, TUI later).
* **Pretty render of the full document**, but when a block is focused it flips to **raw Markdown editing** (users like Markdown).
* **Simple now; powerful later:** Undo/redo DAG (branching) and IME are future work, but the design must not preclude them.

Key pain points we’re addressing:

* Ad-hoc mutations from the UI into the document.
* Lack of stable identifiers for blocks.
* Cursor/selection drift after edits.
* Fear that Dioxus can’t keep things smooth (it can, if we separate concerns).

---

## Decision

### 1) Source of truth: one **xi-rope** buffer for the whole file

* Store the entire document in **`xi_rope::Rope`**, which gives us efficient inserts/deletes and a **Delta** representation of edits. ([Docs.rs][1])
* We **never** regenerate Markdown from a model. Saving writes the rope bytes verbatim → **lossless round-trip**.

### 2) Edits as **commands** compiled to **Delta**; apply on every input event

* The UI turns each input into a **command**, which compiles to a **Delta** and is applied immediately to the rope.
* (Undo layering is deferred; we will add transaction coalescing and a DAG of revisions later without changing this loop.)

```rust
// Core edit algebra (initial set)
pub enum Cmd {
  InsertText { at: usize, text: String },           // absolute byte offset
  DeleteRange { range: std::ops::Range<usize> },
  SplitListItem { at: usize },                      // newline + copy indent/marker
  IndentLines { range: std::ops::Range<usize> },    // add spaces at line starts
  OutdentLines { range: std::ops::Range<usize> },
  ToggleMarker { line_start: usize, to: Marker },   // "-", "*", "1.", etc.
}
```

**Editing loop (per input):** prevent the default DOM edit, apply `Cmd`, repaint from the new snapshot. For IME we temporarily let the browser compose and commit on `compositionend` (see Future Work). MDN’s `beforeinput` and composition events exist explicitly to support this pattern. ([MDN Web Docs][2])

### 3) Parse overlay: **Tree-sitter Markdown** (incremental)

* Use **Tree-sitter** for an **incremental** parse over the rope buffer. We feed edits (`tree.edit`), then re-parse, which updates only changed regions. ([tree-sitter.github.io][3], [GitHub][4])
* Grammar: **`tree-sitter-markdown`** (block + inline grammars, via `tree_sitter_md` crate). We can relax/extend rules later for “Markdown+” (e.g., headings inside bullets) by post-processing nodes or forking the grammar. ([GitHub][5], [Docs.rs][6])

### 4) **Anchors**: stable IDs over byte ranges (v1 simple)

See [../anchors.md](../anchors.md)

We need stable IDs for blocks that survive edits.

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct AnchorId(pub u128);

pub struct Anchor {
  pub id: AnchorId,
  pub range: std::ops::Range<usize>, // byte range in the rope
  // (v2 will add bias/stickiness and kind hints; see Future Work)
}
```

* On every applied Delta, **transform all anchor ranges** through the delta (xi-rope supports interval transforms), so IDs “slide” with text. ([Docs.rs][7])
* After the incremental parse, **only for changed regions**, re-associate affected anchors to the best-overlap node (e.g., list\_item). Most edits need no rebind.

### 5) Read API: **Snapshot** of render blocks (model → view)

The core exposes an immutable **Snapshot** describing how to render and where to edit, without exposing the rope directly:

```rust
pub struct Snapshot {
  pub version: u64,
  pub blocks: Vec<RenderBlock>,
}

pub struct RenderBlock {
  pub id: AnchorId,
  pub kind: BlockKind,               // Paragraph | Heading{level} | ListItem{marker, depth} | CodeFence{lang} | ...
  pub byte_range: Range<usize>,      // full line bytes
  pub content_range: Range<usize>,   // editable text (after indent/marker)
  pub depth: usize,                  // indent level (for padding in UI)
  // optional: precomputed spans for styling
}
```

Snapshots are produced from the Tree-sitter CST plus the anchors. The UI renders from this structure and never mutates the rope.

### 6) UI pattern: **Pretty everywhere, raw Markdown in the focused block**

* The Dioxus UI renders a list of **RenderBlock**s (keyed by **AnchorId**) in a “pretty” style.
* When a block is focused, it switches to an **EditorBlock** that shows a **controlled `<textarea>`** with the **exact bytes** of `content_range` and a gutter for indent/marker so it visually aligns with bullets.
* Input handling:

  * Use `beforeinput` → **preventDefault** → send a `Cmd` → `apply()` → update textarea value + selection from the rope.
  * Special keys: Tab/Shift+Tab (indent/outdent), Enter (split list item), Cut/Paste (convert to commands).
  * Composition (IME) is deferred (see Future Work). Dioxus supports controlled inputs, event handlers, and fine-grained preventDefault in 0.6. ([dioxuslabs.com][8])
* Selection/caret: tracked as **byte ranges** and transformed through each Delta; mapped to local offsets for the textarea per frame.
* DOM selection APIs (Selection/Range) may be used for click-to-caret mapping in pretty view; we rely on MDN-documented APIs for precise mapping. ([MDN Web Docs][9])

### 7) Frontend frameworks

* **Dioxus** (desktop/web) is our first UI. We use **signals**, **memos**, and **keyed lists** to keep render efficient. ([dioxuslabs.com][10], [Docs.rs][11])
* **Ratatui** (TUI) later: render the same **RenderBlock**s with a text UI; dispatch the same commands. Ratatui is well-documented and stable. ([Docs.rs][12], [Ratatui][13])

---

## Detailed Specification

### Core types (headless crate)

```rust
pub type Buffer = xi_rope::Rope;
pub type Delta  = xi_rope::delta::Delta<xi_rope::RopeInfo>;

pub struct Doc {
  buf: Buffer,
  // parsing
  ts_parser: tree_sitter::Parser,
  ts_tree: Option<tree_sitter::Tree>,
  // anchors
  anchors: Vec<Anchor>, // interned + index by range or id
  // selection
  selection: std::ops::Range<usize>, // absolute bytes
  version: u64,
}

pub struct Patch {
  pub changed: Vec<std::ops::Range<usize>>,
  pub new_selection: std::ops::Range<usize>,
  pub version: u64,
  // optionally expose the applied Delta for clients that coalesce
  pub applied_delta: Option<Delta>,
}
```

**Core API**

```rust
impl Doc {
  pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;
  pub fn to_bytes(&self) -> Vec<u8>; // exact rope bytes

  pub fn apply(&mut self, cmd: Cmd) -> Patch;

  pub fn snapshot(&self) -> Snapshot;

  // hit testing helpers (optional now, useful for pretty-mode caret mapping)
  pub fn locate_in_block(&self, id: AnchorId, line: usize, col_utf16: usize) -> usize;
  pub fn describe_point(&self, byte: usize) -> (AnchorId, usize /*line*/, usize /*col*/);
}
```

**Apply loop (high level):**

1. Build `Delta` from `Cmd`.
2. `buf = delta.apply(&buf)`.
3. Transform `selection` and all `anchors` through the delta.
4. Call `tree.edit(...)` with the byte changes and **incrementally parse** only the changed region. ([tree-sitter.github.io][14])
5. Re-associate anchors whose old ranges overlapped changed regions.
6. Bump `version`, compute `changed` byte ranges, and return `Patch`.

**Commands compilation (initial rules):**

* `InsertText` / `DeleteRange`: straightforward byte edits.
* `SplitListItem`: insert `\n` at caret; then insert `indent + marker + (space)` if appropriate; if current line was empty then “un-bullet” on second enter (common Markdown ergonomics).
* `IndentLines` / `OutdentLines`: operate on full line starts within the provided range, add/remove 2 or 4 spaces.
* `ToggleMarker`: replace the list marker token at the given line start; numbering normalization is **not** done in v1.

### UI contract (Dioxus app)

* Keep `Signal<Snapshot>` and `Signal<Option<AnchorId>>` (focused block).
* Render blocks as pretty components keyed by `AnchorId`. When active, replace with `EditorBlock`.
* **EditorBlock**:

  * `<textarea value=… onbeforeinput=… oncompositionstart=… oncompositionend=…>`
  * Left gutter for indent/marker; textarea holds **only** `content_range` slice.
  * On `beforeinput` (non-IME): `event.prevent_default();` → map event to `Cmd` → `apply` → set new `.value` and selection.
  * On `paste`/`cut`: `prevent_default()` and issue `InsertText`/`DeleteRange`.
  * On `compositionstart/update`: let the browser compose; on `compositionend`: compute/insert the finalized text and refresh from rope (see Future Work for full IME plan). ([MDN Web Docs][2])

---

## Trade-offs

* **Tree-sitter vs. custom scanner**: we’re choosing Tree-sitter for cleaner incremental updates and future extensibility. It’s more complex up front than a line scanner but avoids bespoke parsing edge cases and scales better on large files. (We can still post-process or fork the Markdown grammar for “Markdown+” features like headings inside bullets.) ([tree-sitter.github.io][3], [GitHub][5])
* **Per-event apply vs. batch on blur**: per-event apply keeps the **model authoritative** and enables immediate reflow; batching would complicate selection transforms and IME. MDN’s `beforeinput` exists to support per-event interception. ([MDN Web Docs][2])
* **Raw-block editor instead of contenteditable**: far fewer edge cases, predictable selection and IME handling, simpler multi-frontend story. Prior art in ProseMirror/CodeMirror favors model-first command handling and default-prevented inputs. ([ProseMirror][15], [CodeMirror][16])
* **Anchors as simple ranges (v1)**: simplest that works. If anchor churn becomes noticeable during drastic rewrites, we’ll add bias/stickiness and a smarter rebind (v2).

---

## Non-Goals (for this iteration)

* Undo/redo (linear or DAG) — **deferred**, but the design is compatible with a revision DAG later (store `Delta`s and parents).
* Viewport virtualization (only if perf requires it later).
* Automatic list renumbering and advanced markdown normalization (later).

---

## Testing Strategy

### Unit tests (core)

* **Delta compilation:** each `Cmd` creates the expected `Delta` over small fixtures.
* **Selection transform:** after each apply, caret/selection byte ranges are transformed correctly by the applied delta.
* **Anchor transform:** anchors move correctly through inserts/deletes; no panics on edge overlaps.
* **Tree-sitter incremental update:** edits confined to a region update only that region’s nodes; unchanged regions keep their identity (where observable).

### Property tests

* **Round-trip invariants:** arbitrary sequences of random `Cmd`s → `to_bytes()` always matches the starting bytes plus the logical edits (no invalid UTF-8, no truncations).
* **Idempotence of snapshot:** repeated calls to `snapshot()` with no edits produce identical structures (by `AnchorId` and ranges).

### Golden tests (render blocks)

* For representative fixtures (nested lists, headings, code fences, “Markdown+” cases):

  * Assert `Snapshot.blocks` (kind, depth, byte ranges) match stored JSON “goldens.”
  * After specific edits, assert the minimal set of blocks change.

### Integration tests (UI harness)

* Simulate `beforeinput` sequences (type, backspace, enter, tab/shift+tab, paste) and assert rope bytes + snapshot updates + caret mapping.
* **No flicker**: textarea value and selection after apply match expected substring and offsets.

### Performance checks

* Measure apply+incremental parse+snapshot time on 1MB and 5MB fixtures; assert within target (e.g., p95 < 10ms on dev machine).
* Memory sanity: no growth after 10k random edits.

### (Future) IME tests

* Composition sessions for CJK and diacritics; ensure only final insertion reaches rope on `compositionend`.

---

## Migration / Implementation Plan

1. **Core crate skeleton**

   * `Doc`, `Cmd`, `Patch`, `Snapshot`, `RenderBlock`, `Anchor`.
   * Load/save bytes; `apply()` support for `InsertText`, `DeleteRange`, `SplitListItem`, `IndentLines`, `OutdentLines`, `ToggleMarker`.
   * Tree-sitter setup (block+inline grammars) with incremental edits. ([Docs.rs][6])

2. **Dioxus app**

   * Render pretty list from `Snapshot`. Key by `AnchorId`. Signals for snapshot and active block. ([dioxuslabs.com][10])
   * `EditorBlock` with controlled `<textarea>`:

     * implement `beforeinput` path with `prevent_default()` (0.6 API). ([dioxuslabs.com][17])
     * map events → `Cmd` and call `apply()`.
     * initial selection mapping byte↔local offsets.

3. **Stabilize anchor generation**

   * Assign anchors to block-level nodes; transform on delta; minimal rebind in changed regions.

4. **Basic autocomplete hook points**

   * Wire a simple provider for `[[`/`#`/`@` triggers (can return static suggestions initially).

---

## Future Work

* **Undo/redo DAG & timeline UI**: Store each applied Delta as a **revision node** with a parent pointer. Typing after undo creates branches; provide a timeline/branch picker. xi-rope’s Delta model is purpose-built for this. ([Docs.rs][7])
* **IME support (full)**: During `compositionstart/update`, temporarily let the browser own the textarea; at `compositionend` compute/insert the finalized text and resync to rope. (MDN: CompositionEvent, compositionend.) ([MDN Web Docs][18])
* **Viewport virtualization**: Render only visible blocks; query `Snapshot` for byte or block ranges.
* **Markdown+ grammar**: Loosen rules for “headings inside bullets”, custom fenced blocks, etc. via grammar fork or post-processing over the CST. ([GitHub][5])
* **TUI frontend (Ratatui)**: Render `RenderBlock`s with widgets; textarea analogue for the active block; reuse the same commands. ([Docs.rs][12])
* **Numbered list renumbering** (optional) and structural helpers.
* **Search/replace, link autocomplete, backlinks** as providers that propose `Cmd::InsertText`.

---

## Risks & Mitigations

* **Tree-sitter markdown quirks** (CommonMark vs GFM vs Markdown+): mitigate by post-processing nodes and forking the grammar if needed. ([GitHub][19])
* **Selection mapping mismatches** across proportional fonts: in edit mode we use a monospace textarea and align with a gutter; in pretty mode we can use DOM Selection/Range APIs for hit-testing. ([MDN Web Docs][9])
* **Performance on huge files**: xi-rope + incremental parse are designed to scale; add virtualization if needed. ([tree-sitter.github.io][3])

---

## References

* **xi-rope (Delta, interval transforms)** — docs.rs. ([Docs.rs][1])
* **Tree-sitter (incremental parsing)** — site and repo. ([tree-sitter.github.io][3], [GitHub][4])
* **Tree-sitter Markdown grammars / crate** — repos & crate. ([GitHub][5], [Docs.rs][6])
* **MDN `beforeinput` / `input` / composition events** — event semantics. ([MDN Web Docs][2])
* **DOM Selection / Range APIs** — caret/selection mapping. ([MDN Web Docs][9])
* **Dioxus 0.6 SIG/controlled input docs** — signals, state, prevent\_default in 0.6. ([dioxuslabs.com][10])
* **Ratatui** — docs and site. ([Docs.rs][12], [Ratatui][13])
* **Prior art (model-first editors):** ProseMirror & CodeMirror docs (commands, transactions, preventing default). ([ProseMirror][15], [CodeMirror][16])

---

## Glossary

* **IME (Input Method Editor):** OS/browser facility for composing complex characters (CJK, diacritics).
* **CST (Concrete Syntax Tree):** Tree-sitter’s parsed structure; we use it as a **read-only overlay**.
* **Delta:** A compact representation of text edits that can apply to a rope and transform intervals (selections/anchors).
* **Anchor:** Stable ID + byte range that tracks a logical block through edits.

---

## Acceptance Criteria (for this iteration)

* Load/save preserves file bytes exactly.
* Typing, backspace, Enter, Tab/Shift+Tab, paste operate via `Cmd` → `apply()` with immediate re-render.
* Full document is pretty-rendered; focused block flips to raw Markdown textarea aligned to its indent/marker.
* `Snapshot` returns stable `AnchorId`s for blocks; selection and anchors remain consistent through edits.
* Test suite: unit + golden + basic integration as outlined above.

This ADR defines a lean core and a clear UI contract that we can hand to any frontend (Dioxus now, Ratatui later). It’s simple enough to implement quickly, but it keeps all the doors open for undo trees, IME, and Markdown+ extensions without rewrites.

[1]: https://docs.rs/xi-rope?utm_source=chatgpt.com "xi_rope - Rust"
[2]: https://developer.mozilla.org/en-US/docs/Web/API/Element/beforeinput_event?utm_source=chatgpt.com "Element: beforeinput event - MDN - Mozilla"
[3]: https://tree-sitter.github.io/?utm_source=chatgpt.com "Tree-sitter: Introduction"
[4]: https://github.com/tree-sitter/tree-sitter?utm_source=chatgpt.com "tree-sitter/tree-sitter: An incremental parsing system for ..."
[5]: https://github.com/tree-sitter-grammars/tree-sitter-markdown?utm_source=chatgpt.com "Markdown grammar for tree-sitter"
[6]: https://docs.rs/tree-sitter-md?utm_source=chatgpt.com "tree_sitter_md - Rust"
[7]: https://docs.rs/xi-rope/latest/xi_rope/delta/index.html?utm_source=chatgpt.com "xi_rope::delta - Rust"
[8]: https://dioxuslabs.com/learn/0.6/reference/user_input/?utm_source=chatgpt.com "User Input"
[9]: https://developer.mozilla.org/en-US/docs/Web/API/Selection_API?utm_source=chatgpt.com "Selection API - MDN"
[10]: https://dioxuslabs.com/learn/0.6/essentials/state/?utm_source=chatgpt.com "Managing State"
[11]: https://docs.rs/crate/dioxus-signals/latest?utm_source=chatgpt.com "dioxus-signals 0.6.3 - Docs.rs"
[12]: https://docs.rs/ratatui/latest/ratatui/?utm_source=chatgpt.com "ratatui - Rust"
[13]: https://ratatui.rs/?utm_source=chatgpt.com "Ratatui | Ratatui"
[14]: https://tree-sitter.github.io/tree-sitter/using-parsers/?utm_source=chatgpt.com "Using Parsers - Tree-sitter"
[15]: https://prosemirror.net/docs/guide/?utm_source=chatgpt.com "ProseMirror Guide"
[16]: https://codemirror.net/docs/ref/?utm_source=chatgpt.com "Reference Manual"
[17]: https://dioxuslabs.com/learn/0.6/migration/?utm_source=chatgpt.com "How to Upgrade to Dioxus 0.6"
[18]: https://developer.mozilla.org/en-US/docs/Web/API/CompositionEvent?utm_source=chatgpt.com "CompositionEvent - MDN - Mozilla"
[19]: https://github.com/ikatyang/tree-sitter-markdown?utm_source=chatgpt.com "ikatyang/tree-sitter-markdown"
