# Design Document — `markdown-neuraxis`

## 🧠 Overview

This document outlines the technical and product design for the MVP of **`markdown-neuraxis`** — a local-first, markdown-based tool for organizing thoughts, tasks, and knowledge using familiar text files, structured outlines, and meaningful links.

The goal is to create a fast, keyboard-first desktop application that unifies:

- **Task management** (GTD/Sunsama style)
- **Note-taking & outlining** (Logseq/Markdown/Wiki hybrid)
- **Local-first file storage**, with folder support and plaintext primacy

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
- Executable takes single argument - path to root of note folder to open

## 🧱 File & Folder Structure

The following structure is inspired by Logseq & PARA

```
notes/
├── journal/
│   ├── 2025_08_01.md
│   ├── 2025_08_02.md
│   ├── assets/
│   │   ├── image1.png
├── pages/
│   ├── index.md
│   ├── something.md
│   ├── anything-else.md
│   ├── 0_Inbox/
│   │   ├── 19991231-232359-foo.md
│   │   ├── 19991231-232359-bar.png
│   │   ├── 19991231-232359-baz.eml
│   ├── 1_Projects/
│   │   ├── BigProj1/
│   │   │   ├── index.md
│   │   │   ├── something.md
│   │   ├── BigProj2/
│   │   │   ├── something.md
│   ├── 2_Areas/
│   │   ├── Family/
│   ├── 3_Resources/
│   ├── 4_Archive/
│   ├── Companies/
│   │   ├── BigCorpA.md
│   │   ├── BigCorpB.md
│   ├── People/
│   │   ├── Jo Bloggers.md
│   ├── assets/
│   │   ├── image1.png
```

- `journal/` — one file per day, for daily logs/tasks, engineering notebook
- `pages/` — user-created notes, wiki-style
- `assets/` — optional embedded files/images for md files in root 

## ⚙️ Application Stack (Proposed)

| Layer         | Stack / Library Suggestion     |
|---------------|----------------------------------|
| Language      | **Rust** (fast, native, no bloat) |
| GUI Framework | **Tauri** + **Svelte**, or pure Rust GUI (e.g. Dioxus, egui) |
| Markdown      | `pulldown-cmark` or `comrak`      |
| File Access   | Direct OS filesystem, cross-platform |
| State Mgmt    | In-memory + indexed local cache for links and metadata |

## Tests

Outside in integration tests for all features. Unit tests as needed to fill in details, variations and thrash out modules/functions. It must not be possible to break a delivered feature of the app without a test failing.

<https://0x5.uk/2024/03/27/why-do-automated-tests-matter/>

## 🖥️ UI Layout (Initial Idea)

- **Left Sidebar**: File tree (folders/files) or backlinks
- **Main View**: Markdown editor (WYSIWYM, not WYSIWYG)
- **Keyboard Shortcuts** for everything:
  - New file
  - Jump to link
  - Outline collapse/expand
  - Search

### 🪟 Layout Philosophy: Split, Not Tabbed

`markdown-neuraxis` intentionally avoids browser-style tabs.

Tabs create hidden state — which means your brain has to **remember what’s open but not visible**. This adds unnecessary **cognitive load**. Our goal is the opposite: offloading your mental stack into plain sight.

Instead, `markdown-neuraxis` supports **arbitrary split views**, inspired by `tmux`, `i3`, and terminals — where every open note is visible, side-by-side, at once.

This means:
- **No hidden context** — everything is on screen, nothing is tucked away
- **No tab juggling** or mental tax for “what’s open where”
- Vertical and horizontal splits, keyboard-driven
- Perfect for systems thinkers who want multiple perspectives visible (e.g. journal + task list + project file)

The result is a calm, intentional, distraction-free workspace — where you don’t have to remember anything the tool isn’t showing you.

If you want tabs for more complex tasks, you can still use VSCode with your notes as there is no proprietary data store, it's all just folders and markdown files.

## 🔍 Core Features (MVP Build Targets)

### Markdown File Parsing

- Load `.md` files from a selected folder root
- Parse headings, bullet lists, code blocks, metadata

### Outliner UI

- Collapsible bullets (Multiple styles supported: `-`, `*`, `+`, mvp will read all, but only write `-` bullets)
- Show/hide child items

### Backlink Index

- On opening a file, show list of inbound links
- Cross-reference by scanning `[[link]]` usage across files

### Folder Navigation

- True folder support (unlike Logseq namespaces)
- Folders = first-class UX in sidebar

### Link Autocomplete

- When typing `[[...`, suggest matching files/pages

### Metadata Handling

- Bullet properties (e.g. `status:: active`, `due:: 2025-08-03`)
- Queryable

### Simple Query Feature

- Allow rendering dynamic lists like:
   ```
   query:: status:: contacted
   ```
- Render matching bullets/blocks across files

### Tabs and Panes

- Open multiple notes side-by-side
- Open note(s) in new window

### Theme Support

   - Solarized Dark/light mode themes with CSS-like styling

### More MVP features

- Instant write - no save button
- Filesystem watch - instant reload of on-disk changes from other applications
- Timeline View (Chronological journal review)
- Journal calendar (plugin?)

## 💡 Nice-to-Haves (Later Phases)

- Command Palette (like VSCode)
- graph view of nearby / all pages and how they link
- automatic dark/light switching based on system theme changes
- slash-commands
- plugin support
- cloud things
- Android/iOS
- much much more

## 🏁 Initial goals

- Daily journaling with timestamped bullets
- Organizing project work across `pages/` with para method folders
- Linking context between client/project/goals
- Copy-paste rich bullet lists to Docs/Writer
- Seeing backlinks and forward context in a glance
