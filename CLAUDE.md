# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `markdown-neuraxis`, an experimental local-first tool for structured thought, life organization, and personal knowledge management built on plain Markdown files. The project is currently in early groundwork stage with no implementation yet - only documentation and design files exist.

## Current Status

The repository contains only documentation at this stage:
- README.md - Project description and goals
- design.md - Technical and product design document
- LICENSE - AGPL v3 license

This is a greenfield project with no code implementation yet. The next phase will involve building the MVP as a desktop application.

## Planned Architecture

Based on the design document, the planned implementation will be:

### Technology Stack
- **Language**: Rust (for fast, native performance)
- **GUI Framework**: Tauri + Svelte, or pure Rust GUI (Dioxus, egui)
- **Markdown Parsing**: `pulldown-cmark` or `comrak`
- **File System**: Direct OS filesystem access, cross-platform
- **State Management**: In-memory + indexed local cache for links and metadata

### Core Concepts
- Local-first app with no server/sync in MVP
- Markdown files as single source of truth
- Folder-based hierarchy (unlike Logseq namespaces)
- Support for headings, bullet outlines, task states, tags, page links
- Properties system (`property:: value` inline metadata)
- Executable takes single argument - path to root note folder

### File Structure Convention
```
notes/
├── journal/          # Daily journal files (YYYY_MM_DD.md)
├── pages/           # User-created notes, wiki-style
│   ├── 0_Inbox/     # PARA method organization
│   ├── 1_Projects/
│   ├── 2_Areas/
│   ├── 3_Resources/
│   ├── 4_Archive/
│   └── assets/      # Images and other media
```

### MVP Features Planned
- Markdown file parsing with headings, bullets, code blocks, metadata
- Collapsible bullet outlines
- Backlink index and cross-referencing
- Folder navigation in sidebar
- Link autocomplete for `[[wiki-links]]`
- Simple query system for metadata
- Split view layout (no tabs - all content visible)
- Keyboard-first UX

## Development Philosophy

### UI Design Principles
- **Split views, not tabs** - Avoid hidden state and cognitive load
- **Keyboard-first** - Fast navigation and editing
- **Local-first** - No cloud dependencies in MVP
- **Plain text primacy** - Markdown files remain readable outside the app

### Testing Strategy
- Outside-in integration tests for all features
- Unit tests for modules/functions as needed
- No feature should be deliverable without passing tests

## Getting Started (Future)

Once implementation begins:
1. Ensure Rust toolchain is installed
2. Clone the repository
3. The executable will take a single argument - path to notes folder
4. No build commands exist yet as no code has been written

## Development Notes

- Project draws inspiration from GTD, Sunsama, Logseq, Trello/Kanban, and PARA method
- Aims to connect goals → tasks → notes into coherent system
- Licensed under AGPL v3
- Repository is clean with no development tooling or code yet