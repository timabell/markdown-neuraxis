# Methodology — markdown-neuraxis

This document captures the evolving methodology behind `markdown-neuraxis`, combining the most effective practices from GTD, PARA, Sunsama, Logseq, and Kanban into a cohesive, markdown-first life and work management system.

## 🗓 Daily Journaling

- Inspired by *The Unicorn Project*, Logseq, and engineering team habits.
- Each day opens a fresh `journal/YYYY_MM_DD.md` file.
- Bullets written here (in meetings, thoughts, etc.) are considered your **primary inbox**.
- Items may be immediately marked with `INBOX`, `TODO`, `WAITING`, etc.
- Structure is flat, quick to capture — encourages flow.
- Plugins can automatically collect and index inbox-type entries from journals.

## 🎯 Goal Tracking

- Rare but critical.
- Goals are written as outlines and/or files, and uniquely ID’d.
- Tasks and entries anywhere in the system can link directly to goals via `((uuid))`, forming an unbroken chain from life vision → actions.
- Fractal outlines allow arbitrarily deep nesting and linking.

## 🧬 Fractal Notes, Not Projects

- Unlike GTD, we don’t enforce a separate “project” level.
- Notes and tasks exist as **nested outlines** and/or markdown files.
- Every item can act as a project or task depending on its children — hierarchy is emergent, not imposed.
- Projects are a perspective, not a type.

## 📂 PARA Folder Default

- Folder layout uses a modified PARA structure:
  ```
  0_Inbox/
  1_Projects/
  2_Areas/
  3_Resources/
  4_Archive/
  ```
- Notes and folders naturally evolve toward PARA.
- Resources may be linked into projects/goals without duplication.

## 🏷 Tagging for Focus

- Inline `#tags` represent areas of focus or context.
  - E.g. `#client-abc`, `#personal`, `#chore`, `#@laptop`
- Tags are used for filtering dashboards and queries.
- Tags can be freeform but work best when somewhat standardized.

## 🧱 Kanban & WIP Visualisation

- Core plugin or external service can render Kanban boards:
  - Statuses (TODO/DOING/DONE/WIP)
  - Stages (e.g. GTD stages, marketing pipelines, delivery flows)
- Inspired by *The Toyota Way*, *The Goal*, and Trello’s visual simplicity.
- WIP limits optional but encouraged.

## ☀️ Daily Planning

- At start of day, pick a reasonable set of priorities using a "daily triage" UI.
- Pull from inboxes, goals, and queries.
- Intent is to:
  - Act from clarity
  - Align actions with goals
  - Limit scope (Sunsama-style)
- May be supported by offline daily journaling (e.g. Supernote tablet).

## 🔍 Query-Driven Views

- Simple syntax like `query:: status:: DOING` to generate dashboards.
- Plugin engine allows more complex queries (e.g. `goal:: XYZ` or `tag:: #client-abc`).
- Queries power:
  - Kanban views
  - Focus views
  - Daily/weekly planning

## 📝 Notes as Markdown Files

- All content stored in `.md` files.
- Outlines via bullets are optional — works with flat markdown too.
- Ideal for documenting complex software processes, analysis, etc.
- Supports backlinks, headings, and metadata (`property:: value`).

## 📥 Universal Inbox Folder

- `/0_Inbox/` folder accepts anything:
  - Markdown notes
  - Emails
  - Screenshots
  - PDFs
  - Scans
  - Shared links
  - Anything else the OS can hold
- Future plugins may populate from:
  - Browser extension
  - Android share intents
  - Email forwarding
  - Screenshot services
  - Drag & drop from file manager

## 🧠 Philosophy

- **Low friction** capture & organization
- **Markdown first**, text as truth
- **Flow over control** — organize when needed, not pre-emptively
- **Fractal structure** — from daily notes to multi-year goals
- **Opinionated plugin-first UX**, keeping the core tool minimal
- **Extensible but sane** — start simple, build naturally

---

This is the system we've always wanted. Now we're building it.
