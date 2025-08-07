# ADR-0002: Plugin Architecture — Static Internal Plugins (for now)

## Status

Accepted

## Context

As part of the `markdown-neuraxis` project, we plan to support plugin-like extensions to allow features such as:

- Inbox aggregation from journal pages
- Task-to-goal tracing
- PARA/GTD dashboard views
- File importers and converters
- Semantic queries over notes

There are multiple viable architectures for building a plugin system in Rust:

| Option | Summary                                | Notes                                         |
|--------|----------------------------------------|-----------------------------------------------|
| A      | Static plugins via traits              | Compile-time registration; safe and fast      |
| B      | Dynamic plugins via shared libraries   | Loaded via `libloading`; fragile and unsafe   |
| C      | WASM plugin runtime                    | Safe sandboxing; future-facing and flexible   |

This ADR documents the choice for **Option A** in the MVP and establishes guidance for future expansion.

## Decision

We will implement a **static plugin architecture** in Rust using compile-time trait registration.

Each plugin:

- Implements a shared `Plugin` trait
- Registers itself via a central `PluginRegistry`
- Receives events or data from the app (e.g. parsed Markdown)
- Returns output for rendering or indexing

### Example: `Plugin` Trait

```rust
pub trait Plugin {
    fn id(&self) -> &str;
    fn summary(&self) -> &str;

    // Called when a file is parsed and added to the system
    fn on_file_loaded(&mut self, file: &MarkdownFile, index: &mut Index);

    // Called to respond to plugin queries
    fn run_query(&self, query: &str) -> Vec<PluginResult>;
}
```

### Plugin Registration

```rust
let mut registry = PluginRegistry::new();
registry.register(Box::new(InboxFromJournal::new()));
registry.register(Box::new(GoalTracePlugin::new()));
```

### Sample Plugin

```rust
pub struct InboxFromJournal;

impl Plugin for InboxFromJournal {
    fn id(&self) -> &str { "inbox-from-journal" }
    fn summary(&self) -> &str { "Collects INBOX-tagged entries from journal files" }

    fn on_file_loaded(&mut self, file: &MarkdownFile, index: &mut Index) {
        for block in &file.blocks {
            if block.prefix == "INBOX" {
                index.inbox_entries.push(block.clone());
            }
        }
    }

    fn run_query(&self, query: &str) -> Vec<PluginResult> {
        // Not needed for this plugin
        vec![]
```

This lets us rapidly build opinionated workflows (like GTD/Logseq hybrids) as internal plugins without impacting the core.

## Consequences

- ❌ No third-party plugin support in MVP
- ❌ Users cannot load their own code at runtime
- ✅ Extremely safe, cross-platform, and fast to implement
- ✅ Easy to iterate on your own workflow internally

## Future Strategy: WASM Plugin Runtime

We aim to support **WASM-based dynamic plugins** in a future version. This will enable third-party extensibility while maintaining safety and stability.

### Why WASM?

- Portable across platforms
- Sandboxed execution (no direct file/network access)
- Language-flexible: Rust, JavaScript, AssemblyScript, etc.
- Well-supported via [wasmtime](https://github.com/bytecodealliance/wasmtime) or [wasmer](https://wasmer.io)

### WASM Plugin Plan (later phase)

- Define a stable plugin API surface using **WIT (WebAssembly Interface Types)** or **Cap’n Proto**
- Plugins compiled to `.wasm` and placed in a known folder
- Runtime loads `.wasm` modules and passes Markdown data
- Controlled host functions:
  - read/write virtual files
  - query graph/index
  - return plugin output blocks
- Plugins sandboxed with:
  - No filesystem/network access by default
  - Optional permissions declared in `plugin.toml`

Example:

```toml
[plugin]
id = "my-task-analytics"
permissions = ["read-files", "read-properties"]
```

Host enforces access limits per plugin.

## Decision Summary

We choose **static internal plugin architecture** for now to:

- Maximize speed and simplicity
- Keep the core focused
- Dogfood opinionated workflows
- Delay WASM runtime effort until v0.3+
