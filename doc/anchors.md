An explanation of "Anchors" courtesy of ChatGPT:

---

Got it — anchors are subtle but *they’re the glue* that keeps everything stable when the buffer is mutating all the time. Let me go slow, no shorthand.

---

# What is an Anchor?

Think of an **Anchor** as a *bookmark* inside the rope.

* A normal bookmark in a book is “page 57, line 3”. If you rip out pages or insert new ones, that bookmark is suddenly pointing to the wrong thing.
* An **Anchor** in our editor is a *smart bookmark*: it moves automatically when you insert or delete text, so it always points to the same logical part of the document.

---

# Why do we need Anchors?

1. **Stable IDs for UI**

   * The UI needs to say: “I’m rendering *this* bullet item, don’t replace it every time text changes.”
   * If you keyed components by line numbers, everything would break when lines are inserted or deleted. Anchors fix that.

2. **Selection & cursor**

   * When you type, the caret (cursor) is at a byte offset. If you insert text before it, the caret should slide forward.
   * Anchors are the way to represent “this caret is between characters X and Y” in a way that survives edits.

3. **Structural references**

   * Features like “keep track of this task even if it’s moved/indented” need an ID that doesn’t change just because bytes shifted around.

---

# Anchor structure (simple version)

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct AnchorId(pub u128); // stable unique id

pub struct Anchor {
    pub id: AnchorId,
    pub range: std::ops::Range<usize>, // absolute byte positions in the rope
}
```

* **`id`**: A stable identifier you hand to the UI (e.g. React/Dioxus keys).
* **`range`**: The slice of the rope this anchor covers (e.g. a whole list item line, or just the editable content after `- `).

---

# How do Anchors “move”?

* Every edit is a **Delta** (from xi-rope). Deltas know how text shifts.
* You feed the anchor’s `range` through the delta.
* Example:

```
Original rope: "Hello world"
Anchor range: [6..11] (the word "world")

Edit: Insert "big " at byte 6
Delta says: everything after 6 is shifted by +4
Anchor range after transform: [10..15]
Now anchor still points to "world"
```

So the anchor *slides forward* automatically.

---

# Anchors vs. Tree-sitter

* **Rope**: stores bytes, editable with deltas.
* **Tree-sitter**: parses bytes into a syntax tree. Nodes are *ephemeral* — they change identity on edits.
* **Anchors**: persistent IDs we control, layered on top. They are how we *stabilize* the connection between “this syntax node” and “the block I’m rendering.”

After an edit:

1. Rope changes.
2. Anchor ranges are transformed by the delta.
3. Tree-sitter reparses only the changed region.
4. For anchors overlapping changed regions, we re-check: does the range still align with a `list_item` node? If yes, keep; if no, update its metadata (e.g. kind, depth).

---

# Example in practice

Say we have:

```
- Buy milk
- Buy bread
```

* Two anchors:

  * `AnchorId(1) → range [0..10]` ("- Buy milk")
  * `AnchorId(2) → range [11..21]` ("- Buy bread")

UI renders:

```
<BlockView key=1 text="- Buy milk" />
<BlockView key=2 text="- Buy bread" />
```

Now you type “eggs and ” before “bread”:

```
Edit: Insert "eggs and " at byte 11
```

* Rope updates.
* Delta transforms Anchor 2’s range to \[19..29].
* Tree-sitter re-parses that line, confirms it’s still a `list_item`.
* UI re-renders Block 2, keyed by the same `AnchorId(2)`.
* Result: only Block 2 updates; focus/caret don’t jump.

Without anchors, the framework would think “oh, new node at position 2” and throw away the old DOM, causing flicker and lost cursor.

---

# Anchor lifecycle rules (v1)

* When creating blocks from parse results, generate a new `AnchorId` for each block if it doesn’t already have one.
* Store `id` + `range` in the Doc’s anchor map.
* On every edit:

  * Transform ranges by delta.
  * Rebind anchors in changed regions to the new syntax nodes.
  * Keep `id` the same unless the block was completely deleted (then drop it).

---

# In short

* **Anchor = (stable id, byte range)**.
* **Delta moves ranges** so anchors “stick” to their text.
* **Tree-sitter labels them** with block kind (heading, list item, paragraph).
* **UI keys by id** so blocks don’t get recreated on every keystroke.

This way the model (rope) stays truth, the parse tree gives structure, and the UI gets stable IDs.

---

👉 Do you want me to sketch a small code example showing: rope → anchors → edit → delta → updated anchors → snapshot → Dioxus render? That might cement it more.
