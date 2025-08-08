# Methodology — markdown-neuraxis

This document captures the evolving methodology behind `markdown-neuraxis`, combining the most effective practices from GTD, PARA, Sunsama, Logseq, and Kanban into a cohesive, markdown-first life and work management system.

## 📥 Task States & Flow

Based on GTD principles, all tasks flow through explicit states to ensure nothing falls through cracks:

### State Values
- **INBOX** - Unprocessed capture; requires triage to determine next action
- **ACTION** - Actionable, bite-sized work that has been fully considered (replaces meaningless "TODO")
- **DOING** - Currently active work (limit WIP for focus)
- **WAITING** - Blocked on external dependency; tracked with context
- **SOMEDAY** - Deferred but potentially valuable; reviewed periodically
- **DONE** - Completed work; archived for reference
- **ABANDONED** - Explicitly discarded; useful for learning patterns

### Flow Rules
- **TODO is banned** - it's a synonym for unprocessed INBOX and has become meaningless
- **ACTION** signifies this is explicitly actionable, not a project with subtasks
- Items must be fully considered before becoming ACTION (what exactly will I do?)
- WAITING items require context (waiting for what/who?)
- Regular review moves SOMEDAY back to INBOX for reconsideration

## 🗓 Daily Journaling

- Inspired by *The Unicorn Project*, Logseq, and engineering team habits.
- Each day opens a fresh `journal/YYYY_MM_DD.md` file.
- Items may be immediately marked with `INBOX`, `ACTION`, `WAITING`, etc.
- Structure is flat, quick to capture — encourages flow.
- Plugins can automatically collect and index inbox-type entries from journals.

## 🎯 Goal Tracking

- Rare but critical.
- Goals are written as outlines and/or files, and can be uniquely ID’d for reference from other files/bullets.
- Tasks and entries anywhere in the system can link directly to goals via `((uuid))`, forming an unbroken chain from life vision → actions.
- Fractal outlines allow arbitrarily deep nesting and linking.

e.g.

```md
# pages/goals.md

- finance
  - work
    - get a promotion
      id:: 68951faf-4df2-4851-9c38-12474ce9806a <-- hidden id to allow cross-linking logseq-style
    - invest in skills
  - investing
  - assets
  - cost control
- happyness
  - be excellent
    - improve skills
      - see also ((68951faf-4df2-4851-9c38-12474ce9806a)) <-- auto-linked to "get a promotion"
```

```md
# journal/yyyy-mm-dd.md

- meditated (not really, kids up first)
- made coffee
- ACTION check email
- INBOX bob called, call him back
- DOING watch ai coding course
  goal::((68951faf-4df2-4851-9c38-12474ce9806a)) <-- magic cross-link to above goal bullet in different file
- ACTION reflect & journal
```


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
  - Statuses (INBOX/ACTION/DOING/WAITING/DONE)
  - Stages (e.g. GTD stages, marketing pipelines, delivery flows)
- Inspired by *The Toyota Way*, *The Goal*, and Trello’s visual simplicity.
- The visual kanban allows you to easily see when you've planned too much, or where your bottleneck is.

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
- Queries power:
  - Kanban views
  - Focus views
  - Daily/weekly planning

## 📝 Notes as Markdown Files

- All content stored in `.md` files.
- Outlines via bullets are optional — works with flat markdown too.
- Ideal for documenting complex software processes, analysis, etc.
- Supports backlinks, headings, and metadata (`property:: value`).
- Supports normal filesystem folders to give "namespacing" as needed. No odd separator characters in filenames, and better interoperability with other systems.

## 📥 Universal Inbox Folder

Pulling from the GTD concept that you *must* capture everything that needs (or demands) your attention into a universal "inbox" for processing - something that has had a huge positive impact on my calmness of mind in a busy life - the plan is a strong support for a simple filesystem based inbox, using filesystem dates to track when they were added.

- `/0_Inbox/` folder accepts anything:
  - Markdown notes
  - Emails
  - Screenshots
  - PDFs
  - Scans
  - Shared links
  - Anything else the OS can hold
- Journal notes and markdown pages are scanned for the `INBOX` prefix on bullets and those are merged in to the virtual universal inbox on the fly ready for triage.
- Future plugins & services may populate from:
  - Browser extension
  - Android share intents
  - Email forwarding
  - Screenshot services
  - Drag & drop from file manager

The system will provide low-friction ways to process INBOX entries into where they should live - whether that's PARA folders for future reference, an addition to the hierarchy of goals, a specific project, an action for the action list, etc etc. or just to have their status flipped to `SOMEDAY`, `WAITING`, or `ABANDONED`.

## 🧠 Philosophy

- **Low friction** capture & organization
- **Markdown first**, text as truth
- **Flow over control** — organize when needed, not pre-emptively
- **Fractal structure** — from daily notes to multi-year goals
- **Opinionated plugin-first UX**, keeping the core tool minimal
- **Extensible but sane** — start simple, build naturally
