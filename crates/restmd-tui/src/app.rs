//! Application state and the `update(event)` transition function.
//!
//! This is the testable heart of the TUI: it owns no terminal and performs no
//! IO beyond spawning background runs and re-scanning the directory. All
//! rendering lives in [`crate::ui`]; all event plumbing in [`crate::event`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use restmd_core::RunReport;

use crate::discover::{LoadedFile, discover};
use crate::event::Event;
use crate::run;

/// Which pane currently has focus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pane {
    Files,
    Requests,
    Response,
}

impl Pane {
    fn next(self) -> Self {
        match self {
            Pane::Files => Pane::Requests,
            Pane::Requests => Pane::Response,
            Pane::Response => Pane::Files,
        }
    }

    fn prev(self) -> Self {
        match self {
            Pane::Files => Pane::Response,
            Pane::Requests => Pane::Files,
            Pane::Response => Pane::Requests,
        }
    }
}

/// The run state of a file's requests.
pub enum RunState {
    Running,
    Done(RunReport),
}

pub struct App {
    pub dir: PathBuf,
    pub files: Vec<LoadedFile>,
    pub selected_file: usize,
    pub selected_request: usize,
    pub focus: Pane,
    pub results: HashMap<PathBuf, RunState>,
    pub response_scroll: u16,
    pub status_line: String,
    pub should_quit: bool,
    /// Set when the user asks to open the current file in `$EDITOR`; the run
    /// loop consumes it (it owns the terminal needed to suspend/restore).
    open_request: Option<PathBuf>,
    tx: Sender<Event>,
}

impl App {
    pub fn new(dir: PathBuf, tx: Sender<Event>) -> Self {
        let files = discover(&dir).unwrap_or_default();
        let status_line = if files.is_empty() {
            format!("No .md files in {}", dir.display())
        } else {
            format!("{} file(s) — press Enter to run, q to quit", files.len())
        };
        Self {
            dir,
            files,
            selected_file: 0,
            selected_request: 0,
            focus: Pane::Files,
            results: HashMap::new(),
            response_scroll: 0,
            status_line,
            should_quit: false,
            open_request: None,
            tx,
        }
    }

    /// Take a pending "open in editor" request, if any. Called by the run loop.
    pub fn take_open_request(&mut self) -> Option<PathBuf> {
        self.open_request.take()
    }

    // --- accessors -------------------------------------------------------

    pub fn current_file(&self) -> Option<&LoadedFile> {
        self.files.get(self.selected_file)
    }

    pub fn request_count(&self) -> usize {
        self.current_file()
            .map(|f| f.document.requests.len())
            .unwrap_or(0)
    }

    /// The run state for the selected file, if any.
    pub fn current_run_state(&self) -> Option<&RunState> {
        self.current_file().and_then(|f| self.results.get(&f.path))
    }

    // --- update ----------------------------------------------------------

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Input(key) => self.on_key(key),
            Event::FilesChanged => self.rescan(),
            Event::RunFinished { path, report } => {
                if self.current_file().map(|f| &f.path) == Some(&path) {
                    self.status_line = format!("Done (exit {})", report.exit_code());
                }
                self.results.insert(path, RunState::Done(report));
            }
            Event::Resize => {}
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab => self.focus = self.focus.next(),
            KeyCode::BackTab => self.focus = self.focus.prev(),
            KeyCode::Char('l') | KeyCode::Right => self.focus = self.focus.next(),
            KeyCode::Char('h') | KeyCode::Left => self.focus = self.focus.prev(),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.rescan(),
            KeyCode::Char('o') => self.request_open(),
            KeyCode::Enter => self.trigger_run(),
            _ => {}
        }
    }

    /// Flag the current file to be opened in the editor.
    fn request_open(&mut self) {
        if let Some(file) = self.current_file() {
            self.open_request = Some(file.path.clone());
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            Pane::Files => {
                if self.selected_file + 1 < self.files.len() {
                    self.selected_file += 1;
                    self.selected_request = 0;
                    self.response_scroll = 0;
                }
            }
            Pane::Requests => {
                if self.selected_request + 1 < self.request_count() {
                    self.selected_request += 1;
                    self.response_scroll = 0;
                }
            }
            Pane::Response => self.response_scroll = self.response_scroll.saturating_add(1),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Pane::Files => {
                if self.selected_file > 0 {
                    self.selected_file -= 1;
                    self.selected_request = 0;
                    self.response_scroll = 0;
                }
            }
            Pane::Requests => {
                if self.selected_request > 0 {
                    self.selected_request -= 1;
                    self.response_scroll = 0;
                }
            }
            Pane::Response => self.response_scroll = self.response_scroll.saturating_sub(1),
        }
    }

    fn trigger_run(&mut self) {
        if self.request_count() == 0 {
            return;
        }
        let Some(file) = self.current_file() else {
            return;
        };
        // Clone everything needed before mutating self (ends the borrow of `file`).
        let path = file.path.clone();
        let doc = file.document.clone();
        let name = file.name.clone();
        let end_index = self.selected_request;

        self.results.insert(path.clone(), RunState::Running);
        self.response_scroll = 0;
        self.status_line = format!("Running {name}…");
        run::spawn(self.tx.clone(), path, doc, end_index);
    }

    /// Re-read the directory, preserving the selected file by path where
    /// possible, and keeping prior run results.
    fn rescan(&mut self) {
        let current_path = self.current_file().map(|f| f.path.clone());
        self.files = discover(&self.dir).unwrap_or_default();

        if let Some(path) = current_path
            && let Some(index) = self.files.iter().position(|f| f.path == path)
        {
            self.selected_file = index;
        }
        self.clamp();
        self.status_line = format!("Refreshed — {} file(s)", self.files.len());
    }

    fn clamp(&mut self) {
        if self.selected_file >= self.files.len() {
            self.selected_file = self.files.len().saturating_sub(1);
        }
        let count = self.request_count();
        if self.selected_request >= count {
            self.selected_request = count.saturating_sub(1);
        }
    }
}
