//! # Inline Parsing
//!
//! Cursor-based inline parsing with explicit raw zones per ADR-0012.
//!
//! ## Architecture
//!
//! Inline parsing is separate from block parsing and operates over the full
//! content span of inline-eligible blocks (paragraphs, headings, list item text).
//!
//! The parser uses a cursor-based approach with "raw zones":
//! - Code spans suppress all other inline parsing inside them
//! - WikiLinks are parsed only outside raw zones
//!
//! ## Modules
//!
//! - **`types`**: `InlineNode` enum (Text, CodeSpan, WikiLink)
//! - **`kinds`**: Inline-specific types with owned delimiters (CodeSpan, WikiLink)
//! - **`cursor`**: `Cursor` for character-by-character parsing with position tracking
//! - **`parser`**: `parse_inline()` main entry point with `try_parse_*` helpers
//!
//! ## Raw Zone Precedence
//!
//! Code spans take precedence: `` `[[not a link]]` `` parses as a single CodeSpan,
//! not as text containing a WikiLink.

pub mod cursor;
pub mod kinds;
pub mod parser;
pub mod types;

pub use parser::parse_inline;
pub use types::InlineNode;
