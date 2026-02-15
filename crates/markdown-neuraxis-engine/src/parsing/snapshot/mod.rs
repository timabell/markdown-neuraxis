//! # Snapshot Testing Support
//!
//! Utilities for testing the parser via snapshot assertions and invariant checks.
//!
//! ## Modules
//!
//! - **`normalize`**: Converts parsed structures to a stable, serializable `Snap` format
//!   for `insta` snapshot testing
//! - **`invariants`**: Runtime checks for parser correctness (spans in bounds,
//!   child spans contained in parents, raw zones produce no inline nodes)
//!
//! ## Testing Strategy (from ADR-0012)
//!
//! "Tests are the spec" - parsing behavior is defined by snapshot tests rather than
//! a separate formal grammar. Snapshots assert block/inline kinds, spans, and key
//! sub-spans (e.g., wikilink target/alias).

pub mod invariants;
pub mod normalize;

pub use invariants::check as invariants;
pub use normalize::{Snap, normalize};
