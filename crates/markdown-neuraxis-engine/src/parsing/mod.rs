/*!
 * # MDNX Parsing Module
 *
 * This module implements a pure-Rust markdown parser as described in
 * [ADR-0012](../../../../doc/adr/0012-replace-tree-sitter-with-rust-parser.md):
 * **Replace Tree-sitter Markdown overlay with a custom pure-Rust "MDNX Markdown+" parser**.
 *
 * ## Architecture Overview
 *
 * The parser follows these key architectural principles from ADR-0012:
 *
 * ### 1. Lossless, Span-Based Parsing
 * - All parsed structures store **byte spans** via [`rope::Span`] into the `xi_rope::Rope`
 * - The rope remains the single source of truth
 * - Slicing any node's span reproduces the exact source text
 * - No markdown is regenerated during rendering
 *
 * ### 2. Two-Phase Block Parsing
 * - **Line classification**: [`blocks::MarkdownLineClassifier`] classifies each line
 *   independently (indentation, blockquote prefixes, fence detection, blank/non-blank)
 * - **Block construction**: [`blocks::BlockBuilder`] maintains a container stack
 *   and emits [`blocks::BlockNode`]s as blocks open and close
 *
 * ### 3. Cursor-Based Inline Parsing
 * - [`inline::parse_inline`] operates over the full block content span as a character stream
 * - Uses [`inline::cursor::Cursor`] for position tracking
 * - Uses explicit "raw zones" where code spans suppress all other inline parsing
 * - Supports MDNX-specific constructs: `[[wikilinks]]`, `[[target|alias]]`
 *
 * ### 4. Knowledge Ownership
 * - All syntax delimiters live with the types that represent them
 * - [`blocks::kinds::BlockQuote::PREFIX`], [`blocks::kinds::CodeFence::BACKTICKS`],
 *   [`inline::kinds::WikiLink::OPEN`], etc.
 * - Classifier/builder/parser code calls these helpers, never hardcodes delimiters
 *
 * ### 5. Block-Level Incrementality
 * - On edit, only impacted blocks are reparsed (not fine-grained tree mutation)
 * - Aligns with [ADR-0004](../../../../doc/adr/0004-editing-code-architecture.md)'s
 *   `xi_rope::Delta` for selection/anchor transforms
 *
 * ## Module Structure
 *
 * - **[`rope`]**: Primitives for span handling ([`rope::Span`]), slicing, line iteration
 *   - [`rope::span`](rope/span.rs) - Byte range type
 *   - [`rope::slice`](rope/slice.rs) - Rope slicing utilities
 *   - [`rope::lines`](rope/lines.rs) - Line iteration with spans
 * - **[`blocks`]**: Block parsing with line classification and container stack
 *   - [`blocks::classify`](blocks/classify.rs) - Line classification
 *   - [`blocks::builder`](blocks/builder.rs) - Block construction state machine
 *   - **[`blocks::kinds`]**: Block types with owned delimiters
 *     - [`blocks::kinds::block_quote`](blocks/kinds/block_quote.rs) - `>` prefix handling
 *     - [`blocks::kinds::code_fence`](blocks/kinds/code_fence.rs) - ``` and ~~~ fences
 *     - [`blocks::kinds::paragraph`](blocks/kinds/paragraph.rs) - Default leaf block
 * - **[`inline`]**: Character-level inline parsing with cursor abstraction
 *   - [`inline::cursor`](inline/cursor.rs) - Position-tracking cursor
 *   - [`inline::parser`](inline/parser.rs) - Main parsing logic
 *   - **[`inline::kinds`]**: Inline types with owned delimiters
 *     - [`inline::kinds::code_span`](inline/kinds/code_span.rs) - Backtick code spans
 *     - [`inline::kinds::wikilink`](inline/kinds/wikilink.rs) - `[[target|alias]]` links
 * - **`tests/`**: Snapshot tests with fixtures (internal, not exported)
 *
 * ## Usage Pattern
 *
 * ```rust
 * use xi_rope::Rope;
 * use markdown_neuraxis_engine::parsing::{parse_document, parse_inline_for_block};
 *
 * // 1. Create rope from markdown text
 * let rope = Rope::from("Hello [[world]]!\n\n> Quote with `code`");
 *
 * // 2. Parse document into blocks
 * let doc = parse_document(&rope);
 *
 * // 3. For each paragraph block, parse inline nodes
 * for block in &doc.blocks {
 *     let inlines = parse_inline_for_block(&rope, block);
 *     // inlines contains Text, WikiLink, CodeSpan nodes with byte spans
 * }
 *
 * // 4. Slice rope with any span to get exact source text
 * // rope.slice_to_cow(span.start..span.end)
 * ```
 *
 * ## Supported Constructs (v1)
 *
 * **Blocks:**
 * - Paragraphs (default leaf block)
 * - Fenced code blocks (``` and ~~~)
 * - Blockquotes (nested, with proper prefix stripping)
 *
 * **Inline:**
 * - Plain text
 * - Code spans (raw zone - suppresses other parsing)
 * - WikiLinks: `[[target]]` and `[[target|alias]]`
 *
 * ## Future Additions
 *
 * - Lists and list items (container blocks)
 * - Headings (ATX style)
 * - Emphasis, links, images
 * - Integration with [ADR-0004](../../../../doc/adr/0004-editing-code-architecture.md)
 *   Snapshot/RenderBlock system
 */

pub mod blocks;
pub mod inline;
pub mod rope;

#[cfg(test)]
mod tests;

use xi_rope::Rope;

use blocks::{BlockBuilder, BlockKind, BlockNode, MarkdownLineClassifier};
use rope::{lines_with_spans, slice::slice_to_string};

/// A parsed markdown document containing all blocks.
#[derive(Debug)]
pub struct ParsedDoc {
    /// All blocks in document order.
    pub blocks: Vec<BlockNode>,
}

/// Parses a rope into a document structure.
///
/// Performs block parsing only (two-phase: line classification + block construction).
/// Inline parsing should be done separately via [`parse_inline_for_block`].
pub fn parse_document(rope: &Rope) -> ParsedDoc {
    let classifier = MarkdownLineClassifier;
    let mut builder = BlockBuilder::new();

    for lr in lines_with_spans(rope) {
        let lc = classifier.classify(&lr);
        builder.push(&lc);
    }

    ParsedDoc {
        blocks: builder.finish(),
    }
}

/// Parses inline content for a block node.
///
/// Currently only parses paragraphs; fenced code blocks return empty
/// (they are raw zones with no inline parsing).
///
/// # Arguments
/// - `rope`: The source rope for slicing content
/// - `b`: The block node to parse inlines for
///
/// # Returns
/// A vector of [`inline::InlineNode`]s covering the block's content span.
pub fn parse_inline_for_block(rope: &Rope, b: &BlockNode) -> Vec<inline::InlineNode> {
    if !matches!(b.kind, BlockKind::Paragraph) {
        return vec![];
    }
    let s = slice_to_string(rope, b.content_span);
    inline::parse_inline(b.content_span.start, &s)
}
