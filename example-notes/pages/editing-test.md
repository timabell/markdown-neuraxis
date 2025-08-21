# Block Editing & Splitting Test

This is a test file for the new block editing and splitting features.

## How it works

- Click on any block to edit it
- Press Escape or click elsewhere to save
- Changes are automatically saved to disk
- **NEW**: Add double newlines to split blocks!

Try editing this paragraph and add double newlines!

## Block Splitting Demo

Edit this paragraph and try typing something like:

First paragraph

Second paragraph

When you save, it should become two separate blocks!

## Example List

- First item
- Second item with [[wiki-link]]  
- Third item

> This is a quote block

```rust
fn hello() {
    println!("This is a code block");
}
```

## Technical Implementation

- **UUID-based BlockIds**: Stable identifiers that don't break on insertion
- **Multi-block parsing**: `parse_multiple_blocks()` splits on `\n\n`
- **1-to-N replacement**: `finish_editing()` can replace one block with many
- **Block insertion methods**: Add blocks at start/end/before/after positions