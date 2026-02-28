/*!
 * # Editing Core Module (ADR-0004)
 *
 * This module implements the editor core architecture described in ADR-0004:
 * **Editor Core & UI Architecture for the next iteration**.
 *
 * ## Architecture Overview
 *
 * The editing system follows these key architectural principles from ADR-4:
 *
 * ### 1. Single Source of Truth: xi-rope Buffer
 * - The entire document is stored in a single **`xi_rope::Rope`** buffer
 * - Provides efficient insert/delete operations and **Delta** representation of edits
 * - **Lossless round-trip**: saving writes rope bytes verbatim with no formatting drift
 * - Never regenerates Markdown from a model - preserves exact byte representation
 *
 * ### 2. Command-Based Editing
 * - All edits are represented as **Commands** (`Cmd` enum) that compile to **Deltas**
 * - Commands are applied immediately on every input event for authoritative model updates
 * - Supports undo/redo through Delta history (future enhancement)
 *
 * ### 3. Incremental Parsing with Tree-sitter
 * - Uses **Tree-sitter Markdown** for incremental parsing over the rope buffer
 * - Feeds edits via `tree.edit()`, then re-parses to update only changed regions
 * - Provides structured access to document elements while preserving round-trip fidelity
 *
 * ### 4. Stable Block IDs via Anchors
 * - **Anchors** provide stable identifiers for text ranges that survive edits
 * - Anchor ranges are transformed through Deltas using xi-rope's interval transformation
 * - After incremental parse, anchors are re-associated with best-overlapping nodes
 * - Enables stable UI references to document blocks across edits
 *
 * ### 5. Read API: Immutable Snapshots
 * - The core exposes **Snapshots** describing how to render without exposing rope directly
 * - Snapshots contain **RenderBlocks** with stable AnchorIds, content ranges, and metadata
 * - UI renders from snapshots and never directly mutates the rope
 * - Supports both "pretty" rendering and raw Markdown editing of focused blocks
 *
 * ## Module Structure
 *
 * - **`document`**: Core `Document` type with xi-rope buffer and Tree-sitter integration
 * - **`commands`**: `Cmd` enum and delta compilation logic for all edit operations
 * - **`anchors`**: Stable block ID system with range transformation and rebinding
 * - **`snapshot`**: Immutable view generation with `RenderBlock`s for UI consumption
 * - **`patch`**: Edit result metadata including changed ranges and new selection
 *
 * ## Usage Pattern
 *
 * ```rust
 * use markdown_neuraxis_engine::editing::*;
 *
 * // 1. Create document from bytes (lossless)
 * let markdown_bytes = b"# Hello\n\n- Item 1\n- Item 2";
 * let mut doc = Document::from_bytes(markdown_bytes).unwrap();
 *
 * // 2. Initialize anchors for stable block IDs
 * doc.create_anchors_from_tree();
 *
 * // 3. Apply edits via commands
 * let patch = doc.apply(Cmd::InsertText { at: 0, text: "# ".to_string() });
 *
 * // 4. Get immutable snapshot for rendering
 * let snapshot = doc.snapshot();
 *
 * // 5. Round-trip: save exact bytes
 * let saved_bytes = doc.text(); // Get current content
 * ```
 *
 * This architecture enables:
 * - **Exact round-trip preservation** of Markdown files
 * - **Fast edits** that scale to large documents via incremental parsing
 * - **Stable block references** for UI consistency across edits
 * - **Clean separation** between model (rope) and view (snapshots)
 * - **Multiple frontend support** (Dioxus desktop, future TUI)
 */

// Module exports
pub mod anchors;
pub mod commands;
pub mod document;
pub mod patch;
pub mod snapshot;
pub mod snapshot_v2;

// Public API re-exports
pub use anchors::{Anchor, AnchorId};
pub use commands::Cmd;
pub use document::{Document, Marker};
pub use patch::Patch;
pub use snapshot::{BlockKind, ContentGroup, ListItem, RenderBlock, Snapshot};

/// Point description for ADR-0004 selection/caret transformation
/// Maps global document positions to local textarea coordinates
#[derive(Debug, Clone, PartialEq)]
pub struct PointDescription {
    /// The block containing this point
    pub block_id: AnchorId,
    /// Byte offset within the block's content (not including prefix)
    pub local_byte_offset: usize,
    /// Line number within the block's content (0-based)
    pub local_line: usize,
    /// Column within the line (0-based)
    pub local_col: usize,
    /// Cursor position for textarea selectionStart/selectionEnd
    pub textarea_cursor_pos: usize,
}
