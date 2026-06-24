//! TUI tests: pure `App` state transitions, a `TestBackend` render check, the
//! file-watch rescan path, and a real end-to-end run against the dev server.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use restmd_tui::app::{App, Pane, RunState};
use restmd_tui::event::Event;

fn make_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}

fn app_for(dir: &Path) -> (App, Receiver<Event>) {
    let (tx, rx) = mpsc::channel();
    (App::new(dir.to_path_buf(), tx), rx)
}

fn key(c: char) -> Event {
    Event::Input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn code(code: KeyCode) -> Event {
    Event::Input(KeyEvent::new(code, KeyModifiers::empty()))
}

// ---------------------------------------------------------------------------
// Discovery & navigation
// ---------------------------------------------------------------------------

#[test]
fn discovers_and_navigates_files_and_requests() {
    let dir = make_dir(&[
        ("a.md", "## GET /one\n## POST /two\n"),
        ("b.md", "## DELETE /three\n"),
    ]);
    let (mut app, _rx) = app_for(dir.path());

    assert_eq!(app.files.len(), 2);
    assert_eq!(app.focus, Pane::Files);

    // Move down the file list -> select b.md, request selection resets.
    app.update(key('j'));
    assert_eq!(app.selected_file, 1);
    assert_eq!(app.selected_request, 0);

    // Focus the requests pane; b.md has one request, so j is a no-op.
    app.update(code(KeyCode::Tab));
    assert_eq!(app.focus, Pane::Requests);
    app.update(key('j'));
    assert_eq!(app.selected_request, 0);

    // Back to files, select a.md (2 requests), move within requests.
    app.update(key('h'));
    assert_eq!(app.focus, Pane::Files);
    app.update(key('k'));
    assert_eq!(app.selected_file, 0);
    app.update(code(KeyCode::Tab));
    app.update(key('j'));
    assert_eq!(app.selected_request, 1);
}

#[test]
fn q_quits() {
    let dir = make_dir(&[("a.md", "## GET /x\n")]);
    let (mut app, _rx) = app_for(dir.path());
    app.update(key('q'));
    assert!(app.should_quit);
}

#[test]
fn o_requests_opening_the_current_file() {
    let dir = make_dir(&[("a.md", "## GET /x\n")]);
    let (mut app, _rx) = app_for(dir.path());
    let path = app.current_file().unwrap().path.clone();

    app.update(key('o'));
    assert_eq!(app.take_open_request(), Some(path));
    assert_eq!(app.take_open_request(), None); // consumed
}

#[test]
fn empty_directory_is_handled() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _rx) = app_for(dir.path());
    assert!(app.files.is_empty());
    assert_eq!(app.request_count(), 0);
}

// ---------------------------------------------------------------------------
// File watching (rescan path)
// ---------------------------------------------------------------------------

#[test]
fn files_changed_event_rescans_directory() {
    let dir = make_dir(&[("a.md", "## GET /one\n")]);
    let (mut app, _rx) = app_for(dir.path());
    assert_eq!(app.files.len(), 1);

    std::fs::write(dir.path().join("b.md"), "## GET /two\n").unwrap();
    app.update(Event::FilesChanged);
    assert_eq!(app.files.len(), 2);
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

#[test]
fn renders_files_and_requests() {
    let dir = make_dir(&[("api.md", "## GET /hello\n")]);
    let (app, _rx) = app_for(dir.path());

    let mut terminal = Terminal::new(TestBackend::new(120, 24)).unwrap();
    terminal.draw(|f| restmd_tui::ui::draw(f, &app)).unwrap();

    let buf = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                text.push_str(cell.symbol());
            }
        }
    }
    assert!(text.contains("api.md"), "files pane should list the file");
    assert!(
        text.contains("GET /hello"),
        "requests pane should list the request"
    );
    assert!(
        text.contains("Not run"),
        "response pane shows the idle hint"
    );
}

// ---------------------------------------------------------------------------
// Running (delivery of results)
// ---------------------------------------------------------------------------

#[test]
fn run_finished_event_stores_the_report() {
    let dir = make_dir(&[("a.md", "## GET /x\n")]);
    let (mut app, _rx) = app_for(dir.path());
    let path = app.current_file().unwrap().path.clone();

    app.update(Event::RunFinished {
        path,
        report: restmd_core::RunReport {
            outcomes: Vec::new(),
            error: None,
        },
    });
    assert!(matches!(app.current_run_state(), Some(RunState::Done(_))));
}

// ---------------------------------------------------------------------------
// End-to-end against the real dev server
// ---------------------------------------------------------------------------

#[test]
fn runs_a_request_against_the_dev_server() {
    let server = restmd_tui::devserver::spawn().unwrap();
    let source = format!(
        "---\nbase: {}\n---\n\n## GET /users/u-7\n\n> assert status == 200\n> assert $.score >= 3\n",
        server.base
    );
    let dir = make_dir(&[("api.md", &source)]);
    let (mut app, rx) = app_for(dir.path());

    // Enter runs the selected request; the result arrives over the channel.
    app.update(code(KeyCode::Enter));
    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("run should finish");
    app.update(event);

    match app.current_run_state() {
        Some(RunState::Done(report)) => {
            let outcome = &report.outcomes[0];
            assert_eq!(outcome.status, Some(200));
            let response = outcome.response.as_ref().expect("response captured");
            assert!(response.body_text().contains("u-7"));
            assert!(outcome.assertions.iter().all(|a| a.passed));
        }
        _ => panic!("expected a completed run"),
    }
}
