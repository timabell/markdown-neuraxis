# ADR-14: External File Change Detection (Syncthing-driven Reload)

Status: Proposed
Date: 2026-05-06

## Context

Markdown notes are kept in sync between the desktop (Dioxus) app and the
Android app via [Syncthing]. Today both apps:

- Read a file from disk once when the user selects it.
- Hold its content in memory (a `Document` on desktop, parsed `Block`s on
  Android) for the entire time it is open.
- Write back to disk on every keystroke (desktop autosave) or on commit
  (Android `saveEdit`).

When Syncthing replaces the file with a copy from another device, neither app
notices. The user keeps reading stale content; if they edit and save, their
write silently overwrites the freshly-synced version. Syncthing's own
conflict-detection only fires on simultaneous *device-level* writes — it does
not protect against an editor that holds an open buffer across a sync.

Concrete races we need to handle:

1. **Pure external update**: file is open, user is not editing, Syncthing
   writes a new version. The view must reflect the new content.
2. **Mid-edit external update**: file is open, user is in the middle of
   editing a block, Syncthing writes a new version. Their pending edit must
   not be silently lost, and they must not silently lose the incoming
   change either.
3. **Save-race**: an external write lands between the user pressing a key
   and our autosave hitting the disk. The autosave would clobber the
   external change without ever knowing it happened.

We also have to suppress *self-echo*: every save we make produces a
filesystem event, which must not be misread as an external change.

## Decision

### 1. Watch the currently open file (only)

When a file is selected, start a watcher scoped to that file. When the file
is closed or replaced, drop the watcher. Tree-wide watching for sidebar
refresh is a separate concern, not in this ADR.

### 2. Hybrid detection: OS watcher + slow poll, both feeding one channel

Every platform runs **two** detection sources concurrently, merged into the
same `ExternalChange` channel:

- **Primary: OS-level watcher.** `notify` (inotify/FSEvents/ReadDirectoryChangesW)
  on desktop; `ContentResolver.registerContentObserver` on Android.
- **Fallback: ~5s mtime/size poll.** A simple periodic stat.

Both sources call the same internal `check_and_emit()` which compares the
current `(mtime, size)` against the last emitted meta and either skips or
re-reads and emits.

**Why both?** OS watchers miss events in real-world configurations:

- On Linux, Syncthing writes a temporary file then renames it over the
  target. An inotify watch on the target's inode is orphaned by that
  rename. We mitigate by watching the **parent directory** non-recursively
  and filtering on filename, but parent-dir watches still miss events on
  some filesystems (Network File System (NFS), Filesystem in Userspace
  (FUSE) bridges, encrypted overlays).
- On macOS, FSEvents coalesces events with a configurable latency; under
  load, events can be reported seconds late or merged with later events.
- On Android, `ContentObserver` only fires when the writer goes through the
  same `DocumentsContract` provider that issued the URI. Third-party
  Syncthing clients write directly to the underlying filesystem, so their
  writes often produce no observer event at all. The Storage Access
  Framework (SAF) makes `FileObserver` (inotify) impractical because we
  hold a `content://` URI, not a real path.

The poll catches what these miss. At 5s it is cheap enough to run
permanently and still feels live. Pure poll alone would be too laggy for
the common case.

### 3. Watch the parent directory, not the file

On desktop, register the `notify` watcher against the file's parent
directory in non-recursive mode and filter events to the target filename.
This survives Syncthing's atomic rename pattern, which would otherwise kill
an inode-level watch on the first sync.

### 4. `(mtime, size)` is the canonical change signal; content hash as tiebreaker

We attach an `OpenFileMeta { mtime, size, content_hash }` to every open
document. It is the single source of truth for "has the file changed?" and
collapses three concerns onto one mechanism:

| Concern              | Mechanism                                          |
|----------------------|----------------------------------------------------|
| External change      | Watcher/poll detects `(mtime, size)` mismatch      |
| Self-echo            | Watcher drops events whose meta matches stored    |
| Save-race            | Pre-write re-stat, abort if meta no longer matches |

Content hash is a tiebreaker only: some Android Filesystem in Userspace
(FUSE) bridges and File Allocation Table (FAT)-backed SD cards expose
second-resolution mtimes, so `(mtime, size)` can falsely match across two
writes within the same second. When meta differs, we read and hash; when
meta matches but we want to be sure (e.g. after our own write), we
recompute the hash.

### 5. Guarded writes for the open document

`io::write_file_guarded(rel, root, content, expected_meta)` re-stats the
file before writing. If `(mtime, size)` no longer matches `expected_meta`,
the write is aborted and the function returns
`GuardedWriteError::ExternalChange { bytes, mtime, size }` carrying the
disk version. The Dioxus app's autosave path uses this and routes the error
into the same banner state as the watcher. Android's `saveEdit` does the
same.

This is the save-race guard; it is not a substitute for the watcher (the
watcher reaches the user when they are not actively typing) but it covers
the narrow window between blur and the write syscall. A residual
time-of-check-to-time-of-use window remains between the re-stat and the
write syscall; closing it would require `renameat2(RENAME_NOREPLACE)` or
similar atomic primitives and is not pursued here — the watcher will pick
up any external change that lands in that window on the next event.

### 6. UX: silent reload when idle, banner when editing

- **No block focused** → reload `Document::from_bytes(disk_bytes)` silently,
  refresh the snapshot, update meta.
- **A block is focused** → set `external_change` state. The banner appears
  above the editor:
  - **Discard mine** → reload disk version, drop the in-progress edit.
  - **Save copy** → write the in-memory content (with the pending edit
    applied) to a sibling `name.mdnx-conflict-YYYYMMDD-HHMMSS-{device}.md`
    file, then reload the disk version. The `mdnx-conflict-` prefix is
    deliberately distinct from Syncthing's own `.sync-conflict-` pattern
    so a markdown-neuraxis save-race conflict cannot be confused with
    (or cleaned up alongside) a Syncthing device-level conflict.

Device tag: hostname on desktop; `Build.MODEL` (sanitised) on Android.

### 7. Out of scope (future ADRs)

- Suppressing autosave-on-blur while the banner is showing (UI race
  between the banner button and the editor's blur handler).
- Three-way merge / inline diff UI.
- Watching the notes tree for sidebar refresh (file additions / deletions
  via Syncthing).
- Detecting external rename or delete of the open file.
- Cross-app coordination of unsaved buffers between desktop and Android.

## Consequences

### Positive

- Real-time view of files synced from other devices, on both platforms.
- No silent data loss in the common save-race scenario.
- Self-echo is handled at the same layer as external change, with one
  mechanism — easier to reason about and test.
- Parent-dir watching plus poll fallback means the design degrades
  gracefully on quirky filesystems instead of failing closed.

### Negative

- Adds `notify` (and a tiny stat-poll loop) to the engine. New runtime
  threads / coroutines need lifecycle management tied to the open-file
  signal: on desktop the watcher handle is owned by the file-open signal
  and is dropped on file close, app exit, or panic, releasing both the
  operating-system watch and the poll task. Android relies on
  `Lifecycle.repeatOnLifecycle(STARTED)` to pause both sources while
  backgrounded.
- Banner UX puts a decision in front of the user during edits. If we ever
  want to be smarter (auto-merge, three-way diff) it has to layer on top
  of this rather than replace it.
- Five-second poll adds a small steady-state cost (one stat per open file
  per 5s). Negligible on desktop; on Android we pause it via
  `Lifecycle.repeatOnLifecycle(STARTED)` so it doesn't run when
  backgrounded.
- `(mtime, size)` can theoretically collide; the content-hash tiebreaker
  exists for that case but adds a re-read on every change.
- `OpenFileMeta` and the guarded-write error type are engine-internal,
  not exposed across the UniFFI boundary. Android implements the same
  `(mtime, size)` check in Kotlin using `DocumentFile`; only the
  `conflict_path` helper crosses the boundary so both platforms produce
  identical conflict filenames.

### Neutral

- We keep `io::write_file` for non-open-document writes (e.g. creating new
  files from the file tree). Only the autosave path of the currently open
  document needs the guarded variant.

## Alternatives considered

1. **Banner only, no auto-reload.** Simpler, but punishes the common case
   (passive read while syncing) with an extra click for no benefit.
2. **Auto-reload always, no banner.** Discards the user's pending edit
   without warning when they were mid-flow. Unacceptable.
3. **Last-writer-wins.** Don't guard writes; rely on Syncthing's own
   conflict-file mechanism. But Syncthing only sees the device-level race;
   the autosave-after-stale-buffer race is invisible to it because, from
   Syncthing's perspective, our write is just a normal local edit.
4. **Resolve SAF URI → real path → `FileObserver`.** Faster reaction on
   Android, but resolving SAF to a real path is undocumented, breaks for
   non-primary storage, and is fragile across Original Equipment
   Manufacturers (OEMs). Rejected.
5. **Pure poll on Android, no `ContentObserver`.** Simpler, but loses the
   sub-5s reaction time when the provider does notify. Cheap to run both,
   so we run both.
6. **Operating-system watcher only, no poll.** Fails silently on the
   surprisingly large set of configurations where the operating-system
   signal is unreliable (Syncthing rename invalidating an inode watch;
   Android providers that don't notify for direct filesystem writes;
   FSEvents coalescing under load).

## References

- [Syncthing] — file synchronisation tool whose write pattern motivates
  this ADR.
- [`notify`](https://docs.rs/notify) — cross-platform FS event crate.
- ADR-0004 — Editor core architecture; `Document` and the autosave loop
  this ADR modifies.
- ADR-0008 — Android storage architecture; SAF context for the
  `ContentObserver` choice.

[Syncthing]: https://syncthing.net/
