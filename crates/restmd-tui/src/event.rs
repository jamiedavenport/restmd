//! The event channel for *background* sources.
//!
//! Terminal input is polled directly in the run loop (see [`crate::run`]'s
//! caller) so that suspending the TUI to launch an external editor hands stdin
//! cleanly to that editor. Filesystem changes and run completions arrive here,
//! on a channel the loop drains each tick.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use ratatui::crossterm::event::KeyEvent;
use restmd_core::RunReport;

use crate::watch;

/// Something the app reacts to.
pub enum Event {
    Input(KeyEvent),
    Resize,
    /// A `.md` file in the watched directory changed.
    FilesChanged,
    /// A background run finished for the file at `path`.
    RunFinished {
        path: PathBuf,
        report: RunReport,
    },
}

/// Owns the channel and keeps the watcher alive for the app's lifetime.
pub struct Events {
    pub tx: Sender<Event>,
    pub rx: Receiver<Event>,
    _watcher: Option<watch::Watcher>,
}

impl Events {
    /// Create the channel and start watching `dir`.
    pub fn new(dir: &Path) -> Self {
        let (tx, rx) = mpsc::channel();
        let watcher = watch::watch(dir, tx.clone()).ok();
        Self {
            tx,
            rx,
            _watcher: watcher,
        }
    }
}
