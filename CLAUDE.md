# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `markdown-neuraxis`, an experimental local-first tool for structured thought, life organization, and personal knowledge management built on plain Markdown files. The project combines inspiration from GTD, PARA method, Sunsama, Logseq, and Kanban into a cohesive markdown-first system.

## Core Philosophy & Vision

Tim's personal journey through various tools (GTD+Trello → Sunsama → journalling → Logseq + wiki system) led to this unified approach. Key principles:
- **Local-first, open source** to avoid enshittification and vendor lock-in
- **Markdown as single source of truth** - plain text files that work everywhere
- **Connects goals → tasks → notes** into a coherent, navigable system
- **Low friction capture & organization** with flow over control
- **Keyboard-first UX** for speed and clarity
- **Split views, not tabs** to avoid cognitive load of hidden state

## Current Status

The project has evolved from documentation-only to working implementations on multiple platforms:

### Desktop (Rust/Dioxus)
- **GUI Framework**: Switched from egui to Dioxus (see ADR-0001)
- **Basic functionality**: File browser, markdown parsing, outline display
- **Build system**: Rust with Dioxus desktop framework

### Android (Kotlin/Compose)
- **GUI Framework**: Jetpack Compose with Material 3
- **Functionality**: File browser, markdown rendering, wiki-link navigation
- **FFI Integration**: UniFFI bindings to shared Rust parsing library
- **Status**: Read-only viewer with full wiki-link support

Task list lives in `TASKS.md`

## File Structure & Methodology

Based on `doc/methodology.md`, the system supports:

### Daily Workflow
- **Daily journals**: `journal/YYYY_MM_DD.md` files for flow-based capture
- **INBOX system**: Universal capture with `INBOX` prefixed bullets + `0_Inbox/` folder
- **Status tracking**: `TODO`, `DOING`, `DONE`, `WAITING`, `SOMEDAY` states
- **Goal linking**: UUID-based cross-references between files `((uuid))`
- **Daily planning**: Sunsama-inspired daily triage and priority setting

### File Organization (PARA-based)
```
notes/               # Markdown files can be anywhere in root
├── journal/         # Optional subfolder for journals
├── assets/          # Optional subfolder for assets
├── 0_Inbox/         # Universal capture folder
├── 1_Projects/      # Active projects
├── 2_Areas/         # Ongoing responsibilities
├── 3_Resources/     # Reference materials
├── 4_Archive/       # Completed/inactive items
└── any-folders/     # Complete flexibility
```


### Core Features (Current/Planned)
- **Markdown parsing**: Headings, bullets, code blocks, metadata (`property:: value`)
- **Fractal outlines**: Arbitrarily deep nesting, collapsible bullets
- **Cross-linking**: `[[wiki-links]]` with backlink index and autocomplete
- **Tagging**: `#tags` for context and filtering
- **Query system**: `query:: status:: DOING` for dynamic dashboards
- **Kanban views**: Visual WIP management inspired by Toyota Way

## Architecture

### Technology Stack

**Shared Core (Rust)**:
- **Markdown Parsing**: `tree-sitter-md` for incremental parsing (ADR-0004 editing architecture)
- **FFI**: UniFFI for generating cross-platform bindings (Kotlin, Swift)
- **Testing**: `rstest` for parameterized tests, `insta` for snapshot testing, `pretty_assertions`

**Desktop (Rust/Dioxus)**:
- **GUI Framework**: Dioxus desktop 0.6 (switched from egui, see ADR-0001)
- **File System**: Direct OS filesystem access
- **State Management**: In-memory signals for UI state

**Android (Kotlin/Compose)**:
- **Language**: Kotlin 2.0
- **GUI Framework**: Jetpack Compose with Material 3
- **FFI**: JNA 5.15 for Rust library calls via UniFFI
- **File System**: Storage Access Framework (DocumentFile API)
- **Min SDK**: 29 (Android 10), Target SDK: 35 (Android 15)

### Desktop Code Organization
```
src/
├── main.rs              # Entry point, CLI argument handling, window config
├── lib.rs               # Module exports and core unit tests
├── models/              # Core data structures
│   ├── document.rs      # Document with ContentBlock enum (headings, lists, etc.)
│   └── mod.rs
├── editing/             # Core editing model (ADR-0004)
│   ├── document.rs      # xi-rope buffer + tree-sitter parsing
│   ├── commands.rs      # Edit command compilation
│   ├── anchors.rs       # Stable block identity system
│   └── snapshot.rs      # UI-ready document view
├── io/                  # File system operations
│   └── mod.rs           # File scanning, validation, reading
├── ui/                  # Dioxus components
│   ├── app.rs           # Main App component with sidebar/content layout
│   └── components/      # Reusable UI components
│       ├── file_item.rs # Individual file list items
│       ├── main_panel.rs # Content display panel
│       ├── outline.rs   # Hierarchical outline renderer
│       └── mod.rs
├── assets/              # Static resources
│   └── solarized-light.css # Theme styling
├── tests/               # Integration tests
│   ├── integration.rs
│   └── mod.rs
└── snapshots/           # Insta snapshot test files
```

### Android Code Organization
```
android/
├── app/src/main/java/co/rustworkshop/markdownneuraxis/
│   ├── MainActivity.kt              # Entry point, navigation, state management
│   ├── ui/
│   │   ├── screens/
│   │   │   ├── SetupScreen.kt       # Initial folder selection
│   │   │   ├── FileListScreen.kt    # File browser with tree view
│   │   │   ├── FileViewScreen.kt    # Markdown content viewer
│   │   │   └── MissingFileScreen.kt # Broken wiki-link placeholder
│   │   ├── components/
│   │   │   ├── AppBottomBar.kt      # Bottom nav with menu/home
│   │   │   ├── AppDrawer.kt         # Navigation drawer
│   │   │   ├── FileTreeNodeItem.kt  # Expandable tree items
│   │   │   └── StatusToast.kt       # Scanning progress overlay
│   │   └── theme/Theme.kt           # Solarized color scheme
│   ├── model/FileTree.kt            # Hierarchical file tree structure
│   ├── io/
│   │   ├── FileScanner.kt           # High-performance directory scanning
│   │   └── Preferences.kt           # URI persistence via SharedPreferences
│   └── uniffi/markdown_neuraxis_ffi/ # Auto-generated Rust FFI bindings
├── app/build.gradle.kts             # App configuration (SDK 29-35)
├── gradle/libs.versions.toml        # Dependency versions (Compose 2024.12)
└── lint.xml                         # Lint rules for UniFFI compatibility
```

### Android Architecture
- **Framework**: Jetpack Compose with Material 3 design system
- **FFI**: UniFFI-generated Kotlin bindings via JNA to shared Rust library
- **State**: Composable state with file stack for navigation history
- **Storage**: Storage Access Framework (SAF) with DocumentFile API
- **Performance**: Cursor-based directory queries, batch processing, file caching

### Android Features
- ✅ Folder selection with persistent URI permissions
- ✅ Progressive file scanning with real-time UI updates
- ✅ Hierarchical file browser with expand/collapse
- ✅ Markdown rendering (headings, lists, code blocks, quotes)
- ✅ Wiki-link resolution and navigation
- ✅ URL click handling (opens external browser)
- ✅ Pull-to-refresh scanning
- ✅ Dark/light theme support (Solarized)
- ❌ File editing (not yet implemented)
- ❌ Search functionality (not yet implemented)

### Data Flow Architecture
1. **Startup**: CLI validates notes directory structure via `io::validate_notes_dir()`
2. **File Discovery**: `io::scan_markdown_files()` recursively finds `.md` files in notes root directory
3. **File Selection**: User clicks file → `io::read_file()` → `parsing::parse_markdown()` 
4. **Rendering**: Parsed `Document` with hierarchical `OutlineItem`s rendered via Dioxus components
5. **State Management**: Dioxus signals track selected file and current document

### Plugin Architecture (ADR-0002)
- **Current**: Static internal plugins via traits (compile-time)
- **Future**: WASM-based dynamic plugins for third-party extensibility
- **Examples**: Inbox aggregation, goal tracing, PARA dashboards, file importers

### UI Design Principles (from doc/design.md)
- **Split views, not tabs** - Sidebar file browser + main content panel
- **Keyboard-first** - Fast navigation and editing  
- **Local-first** - No cloud dependencies in MVP
- **Plain text primacy** - Markdown files remain readable outside the app

### Desktop Implementation Status
- ✅ **CLI Interface**: Single argument for notes folder path
- ✅ **File Browser**: Recursive markdown file discovery and selection
- ✅ **Markdown Parsing**: Hierarchical bullet point outline extraction
- ✅ **UI Layout**: Sidebar + main content with Solarized Light theme
- ✅ **Error Handling**: Graceful validation and error display
- ✅ **Testing**: Snapshot tests for outline parsing, unit tests for core logic

## Development Notes

### Development Process
- **Follow the standardized workflow**: See `doc/claude-workflow.md` for the complete development process
- Uses specialized agents (feature-implementor, code-reviewer) for complex changes
- Simple changes (typos, single-line fixes) can skip the full workflow
- All commits must have passing tests, be formatted, and include prompt history

### Build Requirements

**Desktop (Rust/Dioxus)**:
- System dependencies for Dioxus desktop (WebKit, GTK, etc.)
- See `doc/development.md` for full setup instructions
- Run `./dev-setup.sh` for automated Ubuntu/Debian setup

**Android**:
- Android Studio with SDK 35
- JDK 11+
- UniFFI library built for target architecture (ARM64/x86_64)
- Run `./lint.sh` to check both Rust and Android linting

### Testing Strategy
- Outside-in integration tests for all features (as per design doc)
- Unit tests for modules/functions as needed
- No feature should be deliverable without passing tests

### Project License & Future
- Licensed under AGPL v3
- Core tooling will remain AGPL
- May provide additional paid services (e.g. email-to-inbox bridge)
- Explicitly avoids duplicating well-solved problems (file sync, etc.)

## Getting Started

### Desktop
1. Install system dependencies (see `doc/development.md`)
2. Clone the repository
3. Run: `cargo run <path-to-notes-folder>`
4. The app will open showing markdown files from the notes root directory

The executable takes a single argument - path to the root notes folder. Markdown files can be organized anywhere within this directory using any folder structure you prefer.

### Android
1. Open the `android/` directory in Android Studio
2. Build the UniFFI library: `cargo build --release --features ffi`
3. Copy the native library to `android/app/src/main/jniLibs/`
4. Build and run from Android Studio
5. On first launch, select your notes folder via the document picker
