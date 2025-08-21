# Editor Design: Block-Based Interactive Editing

We need some kind of visual model for moving between editing and viewing of markdown files.

Possible options:

- Full WYSIWIG 🤮
- Flip to full raw text of document for editing
    - kinda like how you'd edit in vscode or vim
    - this fine for normal markdown docs, and people can still use other editors for this, but it's not really a great fit for a slick UX for an outliner / note-taker like this as the raw is likely much more verbose so it'll be jarring
- raw for bullet being edited
    - this is how logseq does it, flipping to show the raw for whatever you are editing, and I rather like this visual model, it feels more like a rich tool, but still lets you write and interact in raw markdown
    - unlike logseq this tool supports items outside of bullet lists, so it remains to be seen how well this model translates beyond bullet lists, my instinct is that it should still work, as headings, paragraphs, code blocks and quotes can all be treated as blocks that are always rendered unless that block is in edit mode

I'm going to attempt the model of having only the thing being edited show as raw markdown, leaving everything else rendered.

## Core Concept: Edit-in-Place with Block Granularity

The key insight from Logseq's approach is that **only one block is ever in edit mode** while everything else remains rendered. This gives us:
- Fast performance (only re-parse one block on keystroke)
- Visual stability (document doesn't jump around)
- Clear mental model (click to edit, elsewhere to save)

## Proposed Architecture

### 1. Block Identity System

```rust
pub struct BlockId(Uuid); // UUID-based for stability across insertions

pub struct DocumentState {
    blocks: Vec<(BlockId, ContentBlock)>,
    editing_block: Option<(BlockId, String)>, // block_id and raw markdown
}
```

**Design Evolution**: Initially considered index-based BlockIds, but UUIDs provide stability when blocks are inserted/removed/split.

### 2. Bidirectional Block Conversion

We need to convert individual ContentBlocks back to markdown:

```rust
impl ContentBlock {
    fn to_markdown(&self) -> String {
        match self {
            ContentBlock::Paragraph { segments } => {
                segments_to_markdown(segments)
            },
            ContentBlock::Heading { level, text } => {
                format!("{} {}", "#".repeat(*level as usize), text)
            },
            ContentBlock::BulletList { items } => {
                items_to_markdown(items, 0)
            },
            // etc...
        }
    }
}
```

### 3. Block-Level Parser

Block parsing needs to handle both single blocks and block splitting:

```rust
impl ContentBlock {
    // Parse a single block from raw markdown
    fn from_markdown(raw: &str) -> Result<ContentBlock, String> {
        // Determine block type and parse accordingly
    }
    
    // Parse multiple blocks separated by double newlines  
    fn parse_multiple_blocks(markdown: &str) -> Vec<ContentBlock> {
        // Split on \n\n and parse each chunk as a separate block
        // Handles the case where users add newlines to split blocks
    }
}
```

### 4. UI Component Architecture

```rust
#[component]
fn EditableBlock(
    block: ContentBlock,
    block_id: BlockId,
    editing_raw: Option<String>, // Some(raw) if this block is being edited
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
) -> Element {
    if let Some(raw) = editing_raw {
        rsx! {
            textarea {
                value: raw,
                autofocus: true,
                onblur: move |evt| {
                    on_save.call((block_id, evt.value));
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape {
                        on_save.call((block_id, evt.value));
                    }
                }
            }
        }
    } else {
        rsx! {
            div {
                onclick: move |_| on_edit.call(block_id),
                // Render the block normally
                ContentBlockComponent { block }
            }
        }
    }
}
```

## Key Design Decisions

### Block Boundaries

- **Paragraph**: Everything until double newline
- **Heading**: Single line starting with #
- **List**: Consecutive lines with same/nested indentation
- **Code block**: Triple backticks to triple backticks

### State Management

Keep editing state separate from content:
- Document remains immutable during render
- Edit state tracks temporary changes
- On save, parse block and update document

### Persistence Strategy

```rust
fn save_document(doc: &DocumentState, path: &Path) {
    let markdown = doc.blocks
        .iter()
        .map(|(_, block)| block.to_markdown())
        .collect::<Vec<_>>()
        .join("\n\n");
    fs::write(path, markdown).unwrap();
}
```

## Challenges to Solve

### List Item Editing

Lists are tricky because:
- Need to preserve indentation
- Enter should create new list item at same level
- Tab/Shift+Tab should indent/outdent
- Need to handle nested items

Solution: Track list context when editing:

```rust
enum BlockContext {
    Standalone,
    ListItem { level: usize, ordered: bool },
}
```

### Cross-Block Operations

- **Selection across blocks**: Might need a "multi-select" mode
- **Copy/paste**: Need to handle partial block content
- **Drag and drop**: Reorder blocks

### Performance with Large Documents (YAGNI for now)

- Virtual scrolling (only render visible blocks)
- Lazy parsing (parse blocks on demand)
- Debounced saves

## Implementation Status

✅ **Phase 1**: Read-only document viewing (completed)
✅ **Phase 2**: Single-block editing with click-to-edit (completed)
✅ **Phase 3**: Full block types support (completed)
✅ **Phase 4**: Block splitting and insertion (completed)

### Current Implementation

The block-based editing system is now fully functional with:

- **UUID-based BlockIds**: Stable identifiers that don't break on insertion
- **Multi-block parsing**: Users can add `\n\n` to split blocks during editing
- **1-to-N block replacement**: `finish_editing()` handles splitting one block into many
- **Block insertion methods**: Add blocks at start/end/before/after positions
- **Automatic file persistence**: Changes saved to disk when editing completes

### Block Operations

```rust
impl DocumentState {
    // Replace one block with multiple blocks (for splitting)
    pub fn finish_editing(&mut self, block_id: BlockId, new_content: String) -> Vec<BlockId>
    
    // Insert blocks at document boundaries
    pub fn insert_block_at_start(&mut self, new_block: ContentBlock) -> BlockId
    pub fn insert_block_at_end(&mut self, new_block: ContentBlock) -> BlockId
    
    // Insert blocks relative to existing blocks
    pub fn insert_block_after(&mut self, after_id: BlockId, new_block: ContentBlock) -> Option<BlockId>
    pub fn insert_block_before(&mut self, before_id: BlockId, new_block: ContentBlock) -> Option<BlockId>
}
```

### UX Design for Block Addition

**Minimalist Approach**: 
- **Block splitting**: Users add `\n\n` during editing to create new blocks
- **Add at start**: Edit first block and add newlines at the beginning  
- **Add at end**: Simple "+" button after the last block
- **Insert between**: Use block splitting capability

This approach leverages the natural markdown behavior where double newlines create block boundaries, making it intuitive for users familiar with markdown while keeping the UI clean.

### Edge Cases Handled

**Block Splitting**:
- User adds newlines at start/end of block content
- Multiple consecutive blank lines are filtered out
- Mixing different block types (paragraph → heading) works correctly
- Code blocks with newlines are preserved as single blocks

**Block Operations**:
- Empty document handling (first block creation)
- Operations between different block types
- UUID-based BlockIds prevent ID conflicts during rapid operations
- Memory efficiency with stable identifiers

**Performance Considerations**:
- Only the edited block is re-parsed during typing
- File I/O happens only on save (blur/escape)
- UUID generation is fast for typical document sizes
- Block insertion/removal is O(n) but documents are typically small

## Why This Architecture Will Last

1. **Clean separation**: Edit logic separate from render logic
2. **Incremental complexity**: Can add features without rewriting
3. **File-first**: Always serializable to plain markdown
4. **Performance scales**: Only parse/render what's needed
5. **Plugin-friendly**: Each block type can be extended independently

The key insight is that **blocks are the natural unit of editing in markdown** - they map to how people think about documents (paragraphs, lists, headings) and how markdown is structured. By making blocks the atomic unit, we get a clean, extensible architecture.
