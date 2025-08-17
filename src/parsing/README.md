# Parsing Module

## Tree Building Challenge with pulldown-cmark

### The Problem

pulldown-cmark emits a flat stream of events for nested structures. For a nested list like:
```markdown
- Parent
  - Child
```

The event sequence is:
```
Start(List)
  Start(Item)
    Text("Parent")
    Start(List)  ← nested list comes AFTER parent text
      Start(Item)
        Text("Child")
      End(Item)
    End(List)
  End(Item)      ← parent item ends AFTER its nested list
End(List)
```

### The Challenge in Rust

The fundamental issue: when we see `Start(List)` for a nested list, we need to attach it to its parent item, but that parent item hasn't been fully constructed yet (we haven't seen `End(Item)`).

Classic solutions that don't work well:
- **Mutable reference stack**: Rust's borrow checker prevents storing `&mut` references that persist across loop iterations
- **Direct tree building**: Can't modify a parent while building its children

### Our Solution: Deferred Item Addition

Key insight: A nested list always appears between a parent item's text and its `End(Item)` event.

Strategy:
1. When we see `End(Item)`, create the item but DON'T add it yet - store as `pending_item`
2. When we see `Start(List)`:
   - If there's a `pending_item`, this list belongs to it
   - If not, it's a new top-level list
3. When we see `Start(Item)` or `End(List)`, add any `pending_item` to its parent

This works because we defer the parent-child connection until we have all the information.

### Alternative Approaches Considered

1. **Arena Allocator** (e.g., `indextree`): Good for complex trees but overkill for markdown lists
2. **Two-pass**: Parse flat with depths, then build tree - inefficient
3. **Recursive Descent**: Incompatible with pulldown-cmark's iterator API
4. **Interior Mutability** (`Rc<RefCell<>>`): Would work but adds complexity

### Implementation Notes

The current implementation uses:
- `list_stack: Vec<Vec<ListItem>>` - Stack of lists being built at each nesting level
- `pending_item: Option<ListItem>` - Item waiting to be added once we know if it has children
- `list_type_stack: Vec<bool>` - Track whether each list level is numbered or bulleted

This approach is idiomatic Rust: no unsafe code, no complex lifetimes, just careful state management.