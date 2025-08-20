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
pub struct BlockId(usize); // Simple index-based for now

pub enum EditState {
    Rendered,
    Editing {
        raw_markdown: String,
        cursor_position: usize,
    }
}

pub struct DocumentState {
    blocks: Vec<(BlockId, ContentBlock)>,
    edit_states: HashMap<BlockId, EditState>,
    focused_block: Option<BlockId>,
}
```

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

Instead of parsing the entire document, we need a parser that can handle single blocks:

```rust
fn parse_single_block(raw: &str) -> ContentBlock {
    // Parse just this block's markdown
    // This is tricky - need to determine block type from raw text
}
```

### 4. UI Component Architecture

```rust
#[component]
fn EditableBlock(
    block: ContentBlock,
    block_id: BlockId,
    is_editing: bool,
    on_edit: Callback<BlockId>,
    on_save: Callback<(BlockId, String)>,
) -> Element {
    if is_editing {
        rsx! {
            textarea {
                value: block.to_markdown(),
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

## Migration Path

1. **Phase 1**: Read-only with click-to-copy-markdown
2. **Phase 2**: Single-block editing (paragraphs only)
3. **Phase 3**: Full block types support
4. **Phase 4**: Cross-block operations

## Why This Architecture Will Last

1. **Clean separation**: Edit logic separate from render logic
2. **Incremental complexity**: Can add features without rewriting
3. **File-first**: Always serializable to plain markdown
4. **Performance scales**: Only parse/render what's needed
5. **Plugin-friendly**: Each block type can be extended independently

The key insight is that **blocks are the natural unit of editing in markdown** - they map to how people think about documents (paragraphs, lists, headings) and how markdown is structured. By making blocks the atomic unit, we get a clean, extensible architecture.
