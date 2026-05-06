//! Hybrid file watcher (ADR-0014).
//!
//! Combines an operating-system watcher (via the `notify` crate, scoped
//! to the file's parent directory) with a periodic stat-based poll.
//! Both sources funnel into one `Receiver<ExternalChange>` and dedupe
//! via the file's `(mtime, size)` so callers see at most one event per
//! real change. The OS watcher gives sub-second reaction in the happy
//! case; the poll catches changes that are invisible to the OS source
//! (Syncthing atomic rename invalidating an inode-level watch on
//! Linux, FSEvents coalescing under load on macOS, third-party writers
//! bypassing a SAF-derived ContentObserver on Android — though the
//! Android side has its own Kotlin watcher rather than using this
//! module).

use crate::io::OpenFileMeta;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// One detected external change: the bytes currently on disk plus the
/// `(mtime, size)` we read them with.
#[derive(Debug, Clone)]
pub struct ExternalChange {
    pub bytes: Vec<u8>,
    pub meta: OpenFileMeta,
}

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("File has no parent directory: {0}")]
    NoParent(PathBuf),
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Watch handle. Drop it to stop watching.
pub struct FileWatcher {
    _notify_watcher: Option<RecommendedWatcher>,
    _poll_thread: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl FileWatcher {
    /// Start watching `file` for external mutation.
    ///
    /// `initial_meta` is the `(mtime, size)` baseline captured at load
    /// time (typically from `io::read_file_with_meta`); events whose
    /// meta matches the last-emitted value are dropped, so passing the
    /// load-time meta means "tell me when this file diverges from what
    /// I just read." `poll_interval` is the slow-poll fallback period
    /// — 5 seconds is a reasonable default per ADR-0014; values
    /// smaller than 50ms are clamped to 50ms to avoid a hot loop if a
    /// caller passes `Duration::ZERO` by mistake.
    ///
    /// Deletion of the watched file is *not* surfaced as an event. If
    /// the file disappears, `check_and_emit` silently returns an IO
    /// error; the caller will discover the deletion next time it tries
    /// to write. Re-creation is detected normally because the parent
    /// directory is still being watched.
    pub fn watch(
        file: PathBuf,
        initial_meta: OpenFileMeta,
        poll_interval: Duration,
    ) -> Result<(Self, Receiver<ExternalChange>), WatcherError> {
        Self::start(file, initial_meta, poll_interval, true)
    }

    /// Same as `watch` but only runs the polling source. Used by tests
    /// that want deterministic behaviour without depending on
    /// platform-specific `notify` semantics or timing.
    #[cfg(test)]
    pub(crate) fn watch_poll_only(
        file: PathBuf,
        initial_meta: OpenFileMeta,
        poll_interval: Duration,
    ) -> Result<(Self, Receiver<ExternalChange>), WatcherError> {
        Self::start(file, initial_meta, poll_interval, false)
    }

    fn start(
        file: PathBuf,
        initial_meta: OpenFileMeta,
        poll_interval: Duration,
        os_watcher: bool,
    ) -> Result<(Self, Receiver<ExternalChange>), WatcherError> {
        // A zero or near-zero interval would turn the poll thread into
        // a stat hot-loop. 50ms is fast enough for any realistic
        // recovery scenario and slow enough to not waste cycles.
        let poll_interval = poll_interval.max(Duration::from_millis(50));
        let parent = file
            .parent()
            .ok_or_else(|| WatcherError::NoParent(file.clone()))?
            .to_path_buf();
        let target_name = file
            .file_name()
            .ok_or_else(|| WatcherError::NoParent(file.clone()))?
            .to_os_string();

        let (tx, rx) = channel::<ExternalChange>();
        let last_emitted = Arc::new(Mutex::new(initial_meta));
        let stop = Arc::new(AtomicBool::new(false));

        let notify_watcher = if os_watcher {
            let file_clone = file.clone();
            let last_clone = Arc::clone(&last_emitted);
            let tx_clone = tx.clone();
            let target_name_clone = target_name.clone();

            let mut watcher = RecommendedWatcher::new(
                move |res: notify::Result<notify::Event>| {
                    let Ok(event) = res else {
                        return;
                    };
                    let touches_target = event
                        .paths
                        .iter()
                        .any(|p| p.file_name() == Some(target_name_clone.as_os_str()));
                    if !touches_target {
                        return;
                    }
                    if !is_meaningful_event(&event.kind) {
                        return;
                    }
                    let _ = check_and_emit(&file_clone, &last_clone, &tx_clone);
                },
                Config::default(),
            )?;
            // Watch the parent directory non-recursively so we still
            // see events when Syncthing replaces the target via
            // temp-file-then-rename, which would orphan an inode-level
            // watch on the file itself.
            watcher.watch(&parent, RecursiveMode::NonRecursive)?;
            Some(watcher)
        } else {
            None
        };

        let poll_thread = {
            let file_clone = file;
            let last_clone = Arc::clone(&last_emitted);
            let tx_clone = tx;
            let stop_clone = Arc::clone(&stop);
            Some(thread::spawn(move || {
                while !stop_clone.load(Ordering::Relaxed) {
                    thread::sleep(poll_interval);
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = check_and_emit(&file_clone, &last_clone, &tx_clone);
                }
            }))
        };

        Ok((
            Self {
                _notify_watcher: notify_watcher,
                _poll_thread: poll_thread,
                stop,
            },
            rx,
        ))
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // The poll thread wakes on its next tick and exits. The notify
        // watcher's own thread is shut down by RecommendedWatcher's
        // Drop. We deliberately do not join the poll thread here:
        // joining could block whatever thread holds the FileWatcher
        // (a UI thread, in practice) for up to one poll interval.
    }
}

fn is_meaningful_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn check_and_emit(
    file: &Path,
    last_emitted: &Mutex<OpenFileMeta>,
    tx: &Sender<ExternalChange>,
) -> std::io::Result<()> {
    let metadata = std::fs::metadata(file)?;
    let current = OpenFileMeta::from_metadata(&metadata)?;

    let mut guard = last_emitted.lock().expect("watcher mutex poisoned");
    if *guard == current {
        return Ok(());
    }
    let bytes = std::fs::read(file)?;
    *guard = current;
    drop(guard);

    let _ = tx.send(ExternalChange {
        bytes,
        meta: current,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::tempdir;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn read_meta(p: &Path) -> OpenFileMeta {
        let m = std::fs::metadata(p).unwrap();
        OpenFileMeta::from_metadata(&m).unwrap()
    }

    #[test]
    fn watcher_emits_on_external_write() {
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "test.md", "first");
        let initial = read_meta(&path);

        let (_w, rx) =
            FileWatcher::watch(path.clone(), initial, Duration::from_millis(100)).unwrap();

        // Sleep across a second boundary so mtime is guaranteed to
        // differ on filesystems with second-resolution timestamps.
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&path, "second").unwrap();

        let event = rx.recv_timeout(Duration::from_secs(3)).expect("event");
        assert_eq!(event.bytes, b"second");
        assert_eq!(event.meta.size, 6);
    }

    #[test]
    fn watcher_handles_atomic_rename() {
        // Syncthing's write pattern: write to a sibling temp file then
        // rename it over the target. An inode-level watch on the
        // target would be orphaned; the parent-dir watch survives.
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "test.md", "first");
        let initial = read_meta(&path);

        let (_w, rx) =
            FileWatcher::watch(path.clone(), initial, Duration::from_millis(100)).unwrap();

        std::thread::sleep(Duration::from_millis(1100));
        let tmp = dir.path().join("test.md.tmp");
        std::fs::write(&tmp, "renamed in").unwrap();
        std::fs::rename(&tmp, &path).unwrap();

        let event = rx.recv_timeout(Duration::from_secs(3)).expect("event");
        assert_eq!(event.bytes, b"renamed in");
    }

    #[test]
    fn poll_only_picks_up_change() {
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "test.md", "x");
        let initial = read_meta(&path);

        let (_w, rx) =
            FileWatcher::watch_poll_only(path.clone(), initial, Duration::from_millis(100))
                .unwrap();

        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&path, "yy").unwrap();

        let event = rx.recv_timeout(Duration::from_secs(3)).expect("event");
        assert_eq!(event.bytes, b"yy");
    }

    #[test]
    fn echo_suppressed_when_meta_matches_initial() {
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "test.md", "stable");
        let initial = read_meta(&path);

        let (_w, rx) =
            FileWatcher::watch_poll_only(path.clone(), initial, Duration::from_millis(50))
                .unwrap();

        // Several poll ticks elapse without any external write.
        // Nothing should be emitted because (mtime, size) is unchanged.
        let result = rx.recv_timeout(Duration::from_millis(500));
        assert!(result.is_err(), "expected no event, got {result:?}");
    }

    #[test]
    fn dropping_watcher_stops_emissions() {
        let dir = tempdir().unwrap();
        let path = write_file(dir.path(), "test.md", "first");
        let initial = OpenFileMeta {
            mtime: SystemTime::UNIX_EPOCH,
            size: 0,
        };

        let (watcher, rx) =
            FileWatcher::watch_poll_only(path.clone(), initial, Duration::from_millis(50))
                .unwrap();
        // First poll detects the divergence from the bogus initial meta.
        let _ = rx.recv_timeout(Duration::from_secs(1)).expect("first event");
        drop(watcher);

        // After a generous grace window, no more events arrive
        // because both sources have stopped.
        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(&path, "after-drop").unwrap();
        let result = rx.recv_timeout(Duration::from_millis(300));
        assert!(
            result.is_err(),
            "expected no event after drop, got {result:?}"
        );
    }
}
