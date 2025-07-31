# Design Document — `markdown-neuraxis`

## 🧠 Overview

This document outlines the technical and product design for the MVP of **`markdown-neuraxis`** — a local-first, markdown-based tool for organizing thoughts, tasks, and knowledge using familiar text files, structured outlines, and meaningful links.

The goal is to create a fast, keyboard-first desktop application that unifies:
- **Task management** (GTD/Sunsama style)
- **Note-taking & outlining** (Logseq/Markdown/Wiki hybrid)
- **Local-first file storage**, with folder support and plaintext primacy

This doc is designed to be directly actionable by a capable AI coding assistant (e.g. Claude, GPT-4) or a senior developer.

---

## 📦 MVP Feature Scope

### Core Concepts
- Local-first app, no server or sync in MVP
- Markdown is the **single source of truth**
- Notes are just `.md` files in folders
- Folders represent hierarchy
- Each file can have:
  - Headings (`#`, `##`, `###`, etc.)
  - Bulleted outlines (`-`, `*`, or `+`)
  - Task states (`TODO`, `DOING`, `DONE`, etc.)
  - Tags and page links (`#tag`, `[[wiki-link]]`)
  - Properties (`property:: value` inline metadata)

---

## 🧱 File & Folder Structure

The following structure is inspired by Logseq:

```
notes/
├── journal/
│   ├── 2025_08_01.md
│   ├── 2025_08_02.md
├── pages/
│   ├── project-x.md
│   ├── client-y.md
├── assets/
│   ├── image1.png
```

- `journal/` — one file per day, for daily logs/tasks
- `pages/` — user-created notes, wiki-style
- `assets/` — optional embedded files/images

---

## ⚙️ Application Stack (Proposed)

| Layer         | Stack / Library Suggestion     |
|---------------|----------------------------------|
| Language      | **Rust** (fast, native, no bloat) |
| GUI Framework | **Tauri** + **Svelte**, or pure Rust GUI (e.g. Dioxus, egui) |
| Markdown      | `pulldown-cmark` or `comrak`      |
| File Access   | Direct OS filesystem, cross-platform |
| State Mgmt    | In-memory + indexed local cache for links and metadata |

Optional future layers:
- Plugin system via WASM or Rust trait loading
- Sync layer (Syncthing or custom rsync-like plugin)
- AI plugin via local LLM or API endpoint

---

## 🖥️ UI Layout (Initial Idea)

- **Tabbed Interface** like VSCode or Firefox
- **Left Sidebar**: File tree (folders/files) or backlinks
- **Main View**: Markdown editor (WYSIWYM, not WYSIWYG)
- **Keyboard Shortcuts** for everything:
  - New file
  - Jump to link
  - Outline collapse/expand
  - Search

---

## 🔍 Core Features (MVP Build Targets)

1. **Markdown File Parsing**
   - Load `.md` files from a selected folder root
   - Parse headings, bullet lists, code blocks, metadata

2. **Outliner UI**
   - Collapsible bullets (`-`, `*`, `+`)
   - Show/hide child items

3. **Backlink Index**
   - On opening a file, show list of inbound links
   - Cross-reference by scanning `[[link]]` usage across files

4. **Folder Navigation**
   - True folder support (unlike Logseq namespaces)
   - Folders = first-class UX in sidebar

5. **Link Autocomplete**
   - When typing `[[...`, suggest matching files/pages

6. **Metadata Handling**
   - Bullet properties (e.g. `status:: active`, `due:: 2025-08-03`)
   - Queryable

7. **Simple Query Feature**
   - Allow rendering dynamic lists like:
     ```
     query:: status:: contacted
     ```
   - Render matching bullets/blocks across files

8. **Theme Support**
   - Dark/light mode themes with CSS-like styling

9. **Tabs and Panes**
   - Open multiple notes side-by-side
   - Optional: Drag-to-rearrange

10. **Logseq Namespace Import Tool**
   - Convert `my-file/my-subnote.md` into `my-file/subnote.md`
   - Clean up old `my-file_my-subnote.md` patterns

---

## 💡 Nice-to-Haves (Later Phases)

- Command Palette (like VSCode)
- Timeline View (Chronological journal review)
- Mobile-friendly Markdown reader (read-only)
- Plugin API
- Web publishing from `.md` files
- AI integration (task triage, daily summaries, voice-to-task)
- Git-backed version control (optionally)

---

## 🧪 Dev & Debug Tools

- Logging to console + file
- Live reload of markdown edits
- Keyboard shortcut logger/debugger

---

## 🏁 First User Goal: Tim (you)

You're the first power user. MVP should enable:
- Daily journaling with timestamped bullets
- Organizing project work across `pages/`
- Linking context between client/project/goals
- Copy-paste rich bullet lists to Docs/Writer
- Seeing backlinks and forward context in a glance

---

## 📎 GitHub Repo

https://github.com/timabell/markdown-neuraxis

---

## ✅ Final Note

This project is your digital CNS — your **markdown neuraxis**.
Start small. Move fast. Dogfood early.
And let the system emerge from use.

> Build the brain you want to live in.

