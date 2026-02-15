//! # Inline Kinds
//!
//! Inline-specific types that own their syntax delimiters per ADR-0012's
//! "knowledge ownership" principle.
//!
//! ## Types
//!
//! - **`CodeSpan`**: `TICK = b'\`'` - raw zone that suppresses other parsing
//! - **`WikiLink`**: `OPEN = b"[["`, `CLOSE = b"]]"`, `ALIAS = b'|'`
//!
//! ## Design Principle
//!
//! All delimiter constants live here, not scattered in parser code.
//! The parser calls these constants; it never hardcodes `[[` or `` ` ``.

pub mod code_span;
pub mod wikilink;

pub use code_span::CodeSpan;
pub use wikilink::WikiLink;
