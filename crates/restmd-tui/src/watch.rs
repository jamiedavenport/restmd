//! Watching the `.restmd` directory for `.md` changes via `notify`.

use std::path::Path;
use std::sync::mpsc::Sender;

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::event::Event;

/// Keeps the underlying watcher alive; dropping it stops watching.
pub struct Watcher {
    _inner: RecommendedWatcher,
}

/// Watch `dir` (non-recursive) and send [`Event::FilesChanged`] whenever a
/// `.md` file changes. Multiple rapid events simply trigger multiple refreshes,
/// which is harmless (a rescan is cheap and idempotent).
pub fn watch(dir: &Path, tx: Sender<Event>) -> notify::Result<Watcher> {
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res
            && ev
                .paths
                .iter()
                .any(|p| p.extension().is_some_and(|ext| ext == "md"))
        {
            let _ = tx.send(Event::FilesChanged);
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(Watcher { _inner: watcher })
}
