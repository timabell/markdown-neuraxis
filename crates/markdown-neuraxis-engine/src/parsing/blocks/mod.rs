//! # Block Parsing
//!
//! Two-phase block parsing following ADR-0012's container stack model.
//!
//! ## Parsing Phases
//!
//! 1. **Line Classification** (`classify`): Each line is classified into a `LineClass`
//!    containing local facts (indentation, blockquote depth, fence detection, blank status)
//!
//! 2. **Block Construction** (`builder`): A `BlockBuilder` maintains a container stack
//!    and emits `BlockNode`s as blocks open and close
//!
//! ## Modules
//!
//! - **`types`**: Core types (`BlockNode`, `BlockKind`, `ContainerFrame`)
//! - **`kinds`**: Block-specific types with owned delimiters (BlockQuote, CodeFence, Paragraph)
//! - **`classify`**: `MarkdownLineClassifier` produces `LineClass` for each line
//! - **`containers`**: `ContainerPath` for managing nested container state
//! - **`open`**: `try_open_leaf` dispatch for detecting block openers
//! - **`builder`**: `BlockBuilder` state machine for block construction
//!
//! ## Key Invariants
//!
//! - Nesting depth is unbounded (lists in blockquotes in lists, etc.)
//! - Fenced code blocks are raw zones: no block/inline parsing inside
//! - All block nodes store byte spans into the rope

pub mod builder;
pub mod classify;
pub mod containers;
pub mod content;
pub mod kinds;
pub mod open;
pub mod types;

pub use builder::BlockBuilder;
pub use classify::{LineClass, MarkdownLineClassifier};
pub use content::{ContentLine, ContentView};
pub use types::{BlockKind, BlockNode, ContainerFrame};
