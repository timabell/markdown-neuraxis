//! # Rope Primitives
//!
//! Low-level utilities for working with `xi_rope::Rope` in a span-based manner.
//!
//! ## Modules
//!
//! - **`span`**: `Span` type representing byte ranges `[start, end)` into the rope
//! - **`slice`**: Helpers for extracting text from rope spans (`slice_to_string`, `preview`)
//! - **`lines`**: Line-by-line iteration with span tracking (`LineRef`, `lines_with_spans`)
//!
//! ## Design Notes
//!
//! All parsing operates over spans rather than copied strings where possible.
//! The rope is never modified by parsing - it remains the single source of truth.

pub mod lines;
pub mod slice;
pub mod span;

pub use lines::{LineRef, lines_with_spans};
pub use slice::{preview, slice_to_string};
pub use span::Span;
