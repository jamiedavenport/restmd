//! `restmd-tui` — a terminal UI for restmd.
//!
//! Discovers `.md` request files in a directory, lets you navigate files and
//! requests, run them (and the earlier requests they depend on), and inspect
//! the response. Watches the directory and refreshes on change. Read-only,
//! except `o` opens the current file in `$EDITOR`.
//!
//! [`run`] is the entry point used by both the `restmd-tui` binary and the
//! `restmd` CLI.

pub mod app;
pub mod devserver;
pub mod discover;
pub mod event;
pub mod run;
pub mod ui;
pub mod watch;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event as term_event;
use ratatui::crossterm::event::{Event as CtEvent, KeyEventKind};

use crate::app::App;
use crate::event::{Event, Events};

/// How long the loop blocks waiting for input before draining background events.
const TICK: Duration = Duration::from_millis(100);

/// Launch the TUI against `dir`, taking over the terminal until the user quits.
pub fn run(dir: PathBuf) -> Result<()> {
    let events = Events::new(&dir);
    let mut app = App::new(dir, events.tx.clone());

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, &events);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, events: &Events) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Input is polled here (not on a thread) so an external editor can take
        // over stdin cleanly when we suspend.
        if term_event::poll(TICK)? {
            match term_event::read()? {
                CtEvent::Key(key) if key.kind != KeyEventKind::Release => {
                    app.update(Event::Input(key))
                }
                CtEvent::Resize(_, _) => app.update(Event::Resize),
                _ => {}
            }
        }

        // Background events: file changes and run completions.
        while let Ok(event) = events.rx.try_recv() {
            app.update(event);
        }

        if let Some(path) = app.take_open_request()
            && let Err(err) = open_in_editor(terminal, &path)
        {
            app.status_line = format!("Could not open editor: {err}");
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Suspend the TUI, open `path` in `$VISUAL`/`$EDITOR` (default `vi`), then
/// restore. Edits are picked up automatically by the file watcher.
fn open_in_editor(terminal: &mut DefaultTerminal, path: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let mut parts = editor.split_whitespace();
    let Some(program) = parts.next() else {
        anyhow::bail!("no editor set ($VISUAL/$EDITOR)");
    };
    let args: Vec<&str> = parts.collect();

    ratatui::restore();
    let status = Command::new(program).args(&args).arg(path).status();
    *terminal = ratatui::init();
    terminal.clear()?;

    status.map_err(|e| anyhow::anyhow!("failed to launch `{program}`: {e}"))?;
    Ok(())
}
