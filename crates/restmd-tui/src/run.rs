//! Running requests off the UI thread.
//!
//! Execution is blocking (`reqwest::blocking`), so it runs on a spawned thread
//! and reports back via the event channel, keeping input and rendering
//! responsive.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use restmd_core::{Document, ReqwestTransport, RunOptions, Runner};

use crate::event::Event;

/// Run requests `0..=end_index` of `doc` on a background thread, delivering an
/// [`Event::RunFinished`] when done. The prefix is run (not just the one
/// request) so captures from earlier requests are satisfied.
pub fn spawn(tx: Sender<Event>, path: PathBuf, doc: Document, end_index: usize) {
    std::thread::spawn(move || {
        let report = Runner::new(ReqwestTransport::new()).run_through(
            &doc,
            end_index,
            &RunOptions::default(),
        );
        let _ = tx.send(Event::RunFinished { path, report });
    });
}
