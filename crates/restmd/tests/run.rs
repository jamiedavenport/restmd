//! End-to-end tests for `restmd run`, spawning the real binary against a real
//! local `tiny_http` server. We assert the documented exit codes, each output
//! format, request selection, and that `--var` reaches the wire.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::server::{TestServer, closed_port};
use tempfile::TempDir;

/// Write `body` to `<dir>/<name>` and return its path.
fn write_file(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

/// Run `restmd run <args...>` and capture the result.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_restmd"))
        .arg("run")
        .args(args)
        .output()
        .expect("spawn restmd")
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exit code")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---------------------------------------------------------------------------
// Exit codes
// ---------------------------------------------------------------------------

#[test]
fn passing_assertion_exits_zero() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "ok.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /status/200\n\n> assert status == 200\n",
            s.base
        ),
    );
    let out = run(&[file.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).contains('✓'));
}

#[test]
fn failing_assertion_exits_one() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "bad.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /status/200\n\n> assert status == 201\n",
            s.base
        ),
    );
    let out = run(&[file.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
    assert!(stdout(&out).contains('✗'));
}

#[test]
fn parse_error_exits_two() {
    let dir = TempDir::new().unwrap();
    let file = write_file(dir.path(), "broken.md", "## GET\n");
    let out = run(&[file.to_str().unwrap()]);
    assert_eq!(code(&out), 2, "{}", stdout(&out));
    assert!(stdout(&out).contains("broken.md:"), "{}", stdout(&out));
}

#[test]
fn network_error_exits_three() {
    let dir = TempDir::new().unwrap();
    let base = format!("http://127.0.0.1:{}", closed_port());
    let file = write_file(
        dir.path(),
        "net.md",
        &format!("---\nbase: {base}\n---\n\n## GET /data\n"),
    );
    let out = run(&[file.to_str().unwrap()]);
    assert_eq!(code(&out), 3, "{}", stdout(&out));
}

#[test]
fn unknown_env_exits_four() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "env.md",
        &format!("---\nbase: {}\n---\n\n## GET /data\n", s.base),
    );
    let out = run(&[file.to_str().unwrap(), "--env", "nope"]);
    assert_eq!(code(&out), 4, "{}", stdout(&out));
}

#[test]
fn malformed_var_exits_four() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "v.md",
        &format!("---\nbase: {}\n---\n\n## GET /data\n", s.base),
    );
    let out = run(&[file.to_str().unwrap(), "--var", "missing_eq"]);
    assert_eq!(code(&out), 4);
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid --var"));
}

#[test]
fn bad_selector_exits_four() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "sel.md",
        &format!("---\nbase: {}\n---\n\n## GET /data\n", s.base),
    );
    let out = run(&[file.to_str().unwrap(), "-r", "99"]);
    assert_eq!(code(&out), 4);
}

#[test]
fn request_flag_with_multiple_files_exits_four() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    write_file(
        dir.path(),
        "a.md",
        &format!("---\nbase: {}\n---\n\n## GET /data\n", s.base),
    );
    write_file(
        dir.path(),
        "b.md",
        &format!("---\nbase: {}\n---\n\n## GET /data\n", s.base),
    );
    let out = run(&[dir.path().to_str().unwrap(), "-r", "1"]);
    assert_eq!(code(&out), 4);
}

// ---------------------------------------------------------------------------
// Multi-file aggregation
// ---------------------------------------------------------------------------

#[test]
fn directory_aggregates_worst_exit_code() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    write_file(
        dir.path(),
        "pass.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /status/200\n\n> assert status == 200\n",
            s.base
        ),
    );
    write_file(
        dir.path(),
        "fail.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /status/200\n\n> assert status == 500\n",
            s.base
        ),
    );
    let out = run(&[dir.path().to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stdout(&out));
}

// ---------------------------------------------------------------------------
// Formats
// ---------------------------------------------------------------------------

#[test]
fn json_format_round_trips() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "j.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /status/200\n\n> assert status == 201\n",
            s.base
        ),
    );
    let out = run(&[file.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code(&out), 1);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("valid json");
    assert_eq!(v["exit_code"], 1);
    assert_eq!(v["passed"], false);
    assert_eq!(
        v["files"][0]["requests"][0]["assertions"][0]["passed"],
        false
    );
}

#[test]
fn junit_format_is_well_formed_and_escaped() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "u.md",
        &format!(
            "---\nbase: {}\n---\n\n## GET /data\n\n> assert $.name == \"missing\"\n",
            s.base
        ),
    );
    let out = run(&[file.to_str().unwrap(), "--format", "junit"]);
    assert_eq!(code(&out), 1);
    let xml = stdout(&out);
    assert!(xml.contains("<testsuites"));
    assert!(xml.contains("<testsuite "));
    assert!(xml.contains("failures=\"1\""));
    // The assertion description contains a quote, which must be escaped.
    assert!(xml.contains("&quot;"), "{xml}");
}

// ---------------------------------------------------------------------------
// Request selection (prefix threading) and --var
// ---------------------------------------------------------------------------

#[test]
fn selector_runs_prefix_and_reports_only_selected() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let body = format!(
        "---\nbase: {}\n---\n\n## POST /auth/login\n\n```json\n{{\"e\":\"x\"}}\n```\n\n\
> capture uid = $.user.id\n\n\
## GET /projects/{{{{uid}}}}\n\n> assert status == 200\n",
        s.base
    );
    let file = write_file(dir.path(), "flow.md", &body);
    // Select the second request by heading; the first must still run so {{uid}}
    // resolves.
    let out = run(&[file.to_str().unwrap(), "-r", "projects"]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));

    let reqs = s.requests();
    assert_eq!(
        reqs.len(),
        2,
        "prefix should have run the login request too"
    );
    assert_eq!(reqs[1].path, "/projects/u1");

    // Only the selected request is reported.
    let out_text = stdout(&out);
    assert!(out_text.contains("/projects/u1"));
    assert!(!out_text.contains("/auth/login"));
}

#[test]
fn var_override_reaches_the_wire() {
    let s = TestServer::start();
    let dir = TempDir::new().unwrap();
    let file = write_file(
        dir.path(),
        "var.md",
        &format!("---\nbase: {}\n---\n\n## GET /thing/{{{{id}}}}\n", s.base),
    );
    let out = run(&[file.to_str().unwrap(), "--var", "id=42"]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert_eq!(s.requests()[0].path, "/thing/42");
}
