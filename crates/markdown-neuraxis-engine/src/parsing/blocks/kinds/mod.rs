//! # Block Kinds
//!
//! Block-specific types that own their syntax delimiters per ADR-0012's
//! "knowledge ownership" principle.
//!
//! ## Types
//!
//! - **`BlockQuote`**: `PREFIX = '>'`, with `strip_prefixes()` helper
//! - **`CodeFence`**: `BACKTICKS = "```"`, `TILDES = "~~~"`, with `sig()` and `closes()` helpers
//! - **`Paragraph`**: Default leaf block (no delimiters)
//!
//! ## Design Principle
//!
//! All delimiter constants and recognition logic live here, not in classifier/builder code.
//! This enables future text regeneration and keeps magic strings out of parsing logic.

pub mod block_quote;
pub mod code_fence;
pub mod paragraph;

pub use block_quote::BlockQuote;
pub use code_fence::{CodeFence, FenceKind, FenceSig};
pub use paragraph::Paragraph;
