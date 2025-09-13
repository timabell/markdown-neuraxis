// Module exports
pub mod anchors;
pub mod commands;
pub mod document;
pub mod patch;
pub mod snapshot;

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
