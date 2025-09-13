# ADR-0003: Multi-Crate Workspace Structure

## Status
Accepted

## Context
The markdown-neuraxis codebase originally existed as a single crate mixing UI concerns with core processing logic. This monolithic structure presented several challenges:

1. **Testing boundaries**: UI and engine logic were intertwined, making it difficult to test core functionality in isolation
2. **Frontend flexibility**: Supporting multiple frontends (desktop GUI, terminal UI, future web UI) required clear separation
3. **Compilation times**: Changes to UI code required recompiling engine code and vice versa
4. **Dependency management**: UI-specific dependencies (Dioxus, ratatui) were mixed with core dependencies

## Decision
Restructure the codebase into a Rust workspace with three separate crates:

1. **markdown-neuraxis-engine**: Core processing logic
   - Document parsing and manipulation
   - File I/O operations
   - Editing commands and anchoring system
   - Zero UI dependencies
   - Comprehensive unit tests at module boundaries

2. **markdown-neuraxis-dioxus**: Desktop GUI implementation
   - Dioxus-based desktop application
   - Depends on engine crate
   - UI components and state management
   - Integration tests for UI behavior

3. **markdown-neuraxis-cli**: Terminal UI implementation
   - Ratatui-based TUI
   - Depends on engine crate
   - Proof of concept for multi-frontend support
   - Basic file browsing and document viewing

## Consequences

### Positive
- **Clear separation of concerns**: Engine logic is completely isolated from UI concerns
- **Improved testability**: Engine can be tested without UI dependencies
- **Multiple frontend support**: New UIs can be added by creating new crates that depend on the engine
- **Faster iteration**: Changes to UI don't require recompiling the engine
- **Better dependency management**: Each crate only includes dependencies it needs
- **Cleaner API boundaries**: Forces explicit public API design for the engine

### Negative
- **Initial complexity**: More complex project structure with multiple crates
- **Cross-crate coordination**: Changes affecting both engine and UI require updates to multiple crates
- **Build configuration**: Workspace configuration adds complexity to build process

### Neutral
- **Binary size**: Separate binaries for each frontend (can be positive or negative depending on use case)
- **Development workflow**: Developers need to understand workspace structure

## Implementation Notes
- All existing functionality has been preserved
- Tests have been distributed appropriately (engine unit tests in engine crate, UI integration tests in UI crates)
- The workspace uses shared dependencies to avoid version conflicts
- Both frontends demonstrate the same engine can power different UI paradigms