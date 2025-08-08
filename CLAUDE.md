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

The project has evolved from documentation-only to a basic working implementation:
- **GUI Framework**: Switched from egui to Dioxus (see ADR-0001)
- **Basic functionality**: File browser, markdown parsing, outline display
- **Build system**: Rust with Dioxus desktop framework
- **Current branch**: `main` (Dioxus implementation merged)
- Task list lives in `TASKS.md`

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
notes/
├── journal/          # Daily journal files (YYYY_MM_DD.md)
├── pages/           # User-created notes, wiki-style
│   ├── 0_Inbox/     # Universal capture folder
│   ├── 1_Projects/  # Active projects
│   ├── 2_Areas/     # Ongoing responsibilities
│   ├── 3_Resources/ # Reference materials
│   ├── 4_Archive/   # Completed/inactive items
│   └── assets/      # Images and other media
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
- **Language**: Rust (for fast, native performance)
- **GUI Framework**: Dioxus desktop (switched from egui, see ADR-0001)
- **Markdown Parsing**: `pulldown-cmark` (already implemented)
- **File System**: Direct OS filesystem access, cross-platform
- **State Management**: In-memory + indexed local cache for links and metadata

### Plugin Architecture (ADR-0002)
- **Current**: Static internal plugins via traits (compile-time)
- **Future**: WASM-based dynamic plugins for third-party extensibility
- **Examples**: Inbox aggregation, goal tracing, PARA dashboards, file importers

### UI Design Principles (from doc/design.md)
- **Split views, not tabs** - Avoid hidden state and cognitive load
- **Keyboard-first** - Fast navigation and editing  
- **Local-first** - No cloud dependencies in MVP
- **Plain text primacy** - Markdown files remain readable outside the app

### Current Implementation
- Basic Dioxus desktop app with file browser
- Markdown outline parsing and display
- Solarized light theme
- File selection and content viewing
- Works with folder structure as specified

## Development Notes

### Build Requirements
- System dependencies for Dioxus desktop (WebKit, GTK, etc.)
- See `doc/development.md` for full setup instructions
- Run `./dev-setup.sh` for automated Ubuntu/Debian setup

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

Current usage:
1. Install system dependencies (see `doc/development.md`)
2. Clone the repository 
3. Run: `cargo run <path-to-notes-folder>`
4. The app will open showing markdown files from `<path>/pages/` folder

The executable takes a single argument - path to the root notes folder containing the expected structure (journal/, pages/, etc.).
