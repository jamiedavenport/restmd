//! Rendering for `restmd run`: the `pretty`, `json`, and `junit` formats.
//!
//! The core report types ([`RunReport`](restmd_core::RunReport) etc.) carry no
//! serde derives — the machine-readable shapes are owned here as view structs
//! so the wire format stays stable and independent of the core internals.

use std::io::IsTerminal;

use restmd_core::{RequestOutcome, RunReport};
use serde::Serialize;

use crate::run::{FileOutcome, FileRun, OutputFormat, request_name};

/// Render every file's result in `format`. `exit_code` is the aggregate code
/// the process will exit with (its severity is reflected in the output).
pub fn render(format: OutputFormat, runs: &[FileRun], exit_code: i32) -> String {
    let files: Vec<FileView> = runs.iter().map(FileView::build).collect();
    match format {
        OutputFormat::Pretty => render_pretty(&files, runs.len() > 1),
        OutputFormat::Json => render_json(&files, exit_code),
        OutputFormat::Junit => render_junit(&files),
    }
}

// ---------------------------------------------------------------------------
// Normalised view, shared by all three renderers
// ---------------------------------------------------------------------------

/// One file, normalised for rendering.
struct FileView {
    path: String,
    /// Parse errors as `(line, col, message)`; empty unless the file failed to
    /// parse.
    parse_errors: Vec<(usize, usize, String)>,
    /// A pre-flight error not tied to a request (e.g. unknown `--env`).
    preflight_error: Option<String>,
    /// True when `--request` chose a request the run never reached.
    not_reached: bool,
    requests: Vec<RequestView>,
}

#[derive(Serialize)]
struct RequestView {
    index: usize,
    name: String,
    method: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    assertions: Vec<AssertionView>,
    captures: Vec<CaptureView>,
}

#[derive(Serialize)]
struct AssertionView {
    description: String,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
struct CaptureView {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl FileView {
    fn build(run: &FileRun) -> FileView {
        let path = run.path.display().to_string();
        match &run.outcome {
            FileOutcome::ParseErrors(errors) => {
                let parse_errors = errors
                    .iter()
                    .map(|e| {
                        let (line, col) = e.span.line_col(&run.source);
                        (line, col, e.kind.to_string())
                    })
                    .collect();
                FileView {
                    path,
                    parse_errors,
                    preflight_error: None,
                    not_reached: false,
                    requests: Vec::new(),
                }
            }
            FileOutcome::Ran {
                doc,
                report,
                selected,
            } => {
                let indices = displayed_indices(report, *selected);
                let not_reached = selected.is_some() && indices.is_empty();
                let requests = indices
                    .iter()
                    .map(|&i| {
                        request_view(i, &report.outcomes[i], request_name(doc, &run.source, i))
                    })
                    .collect();
                FileView {
                    path,
                    parse_errors: Vec::new(),
                    preflight_error: report.error.as_ref().map(|e| e.to_string()),
                    not_reached,
                    requests,
                }
            }
        }
    }
}

/// Which outcome indices to display: just the selected one (if it was reached),
/// otherwise every outcome.
fn displayed_indices(report: &RunReport, selected: Option<usize>) -> Vec<usize> {
    match selected {
        Some(i) if i < report.outcomes.len() => vec![i],
        Some(_) => Vec::new(),
        None => (0..report.outcomes.len()).collect(),
    }
}

fn request_view(index: usize, outcome: &RequestOutcome, name: String) -> RequestView {
    RequestView {
        index,
        name,
        method: outcome.method.as_str().to_string(),
        url: outcome.url.clone(),
        status: outcome.status,
        passed: outcome.passed(),
        elapsed_ms: outcome.response.as_ref().map(|r| r.elapsed.as_millis()),
        error: outcome.error.as_ref().map(|e| e.to_string()),
        assertions: outcome
            .assertions
            .iter()
            .map(|a| AssertionView {
                description: a.description.clone(),
                passed: a.passed,
                detail: a.detail.clone(),
            })
            .collect(),
        captures: outcome
            .captures
            .iter()
            .map(|c| CaptureView {
                name: c.name.clone(),
                value: c.value.clone(),
                error: c.error.clone(),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// pretty
// ---------------------------------------------------------------------------

const GREEN: &str = "32";
const RED: &str = "31";
const YELLOW: &str = "33";
const BOLD: &str = "1";

fn colors_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn paint(text: &str, code: &str, on: bool) -> String {
    if on {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn render_pretty(files: &[FileView], show_headers: bool) -> String {
    let color = colors_enabled();
    let mut out = String::new();
    let mut total = 0usize;
    let mut failed = 0usize;
    let mut parse_failed = 0usize;

    for (fi, file) in files.iter().enumerate() {
        if show_headers {
            out.push_str(&paint(&file.path, BOLD, color));
            out.push('\n');
        }

        if !file.parse_errors.is_empty() {
            for (line, col, msg) in &file.parse_errors {
                out.push_str(&format!("{}:{line}:{col}: {msg}\n", file.path));
            }
            parse_failed += 1;
        }
        if let Some(err) = &file.preflight_error {
            out.push_str(&paint(&format!("error: {err}\n"), RED, color));
        }
        if file.not_reached {
            out.push_str("request not reached (an earlier request failed)\n");
        }

        for req in &file.requests {
            total += 1;
            if !req.passed {
                failed += 1;
            }
            pretty_request(&mut out, req, color);
        }

        if show_headers && fi + 1 < files.len() {
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&format!(
        "{} request{}, {} failed",
        total,
        if total == 1 { "" } else { "s" },
        failed
    ));
    if parse_failed > 0 {
        out.push_str(&format!(", {parse_failed} file(s) with parse errors"));
    }
    out.push('\n');
    out
}

fn pretty_request(out: &mut String, req: &RequestView, color: bool) {
    let mut head = format!("{} {}", req.method, req.url);
    if let Some(status) = req.status {
        let code = if (200..400).contains(&status) {
            GREEN
        } else {
            YELLOW
        };
        head.push_str(&format!("  {}", paint(&status.to_string(), code, color)));
    }
    if let Some(ms) = req.elapsed_ms {
        head.push_str(&format!("  {ms}ms"));
    }
    out.push_str(&head);
    out.push('\n');

    if let Some(err) = &req.error {
        out.push_str(&paint(&format!("  ✗ {err}\n"), RED, color));
        return;
    }

    for a in &req.assertions {
        let (glyph, c) = if a.passed {
            ("✓", GREEN)
        } else {
            ("✗", RED)
        };
        let mut line = format!("  {glyph} {}", a.description);
        if let Some(detail) = &a.detail {
            line.push_str(&format!("  ({detail})"));
        }
        out.push_str(&paint(&line, c, color));
        out.push('\n');
    }
    for cap in &req.captures {
        let line = match (&cap.value, &cap.error) {
            (_, Some(err)) => paint(&format!("  ✗ capture {}  ({err})", cap.name), RED, color),
            (Some(v), None) => format!("  ✓ capture {} = {v}", cap.name),
            (None, None) => format!("  capture {}", cap.name),
        };
        out.push_str(&line);
        out.push('\n');
    }
}

// ---------------------------------------------------------------------------
// json
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct RunReportJson<'a> {
    passed: bool,
    exit_code: i32,
    files: Vec<FileReportJson<'a>>,
}

#[derive(Serialize)]
struct FileReportJson<'a> {
    file: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parse_errors: Vec<ParseErrorJson>,
    requests: &'a [RequestView],
}

#[derive(Serialize)]
struct ParseErrorJson {
    line: usize,
    col: usize,
    message: String,
}

fn render_json(files: &[FileView], exit_code: i32) -> String {
    let report = RunReportJson {
        passed: exit_code == 0,
        exit_code,
        files: files
            .iter()
            .map(|f| FileReportJson {
                file: &f.path,
                error: f.preflight_error.as_deref(),
                parse_errors: f
                    .parse_errors
                    .iter()
                    .map(|(line, col, message)| ParseErrorJson {
                        line: *line,
                        col: *col,
                        message: message.clone(),
                    })
                    .collect(),
                requests: &f.requests,
            })
            .collect(),
    };
    let mut s = serde_json::to_string_pretty(&report).expect("serialize run report");
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// junit
// ---------------------------------------------------------------------------

fn render_junit(files: &[FileView]) -> String {
    let suites: Vec<String> = files.iter().map(junit_suite).collect();
    let tests: usize = files
        .iter()
        .map(|f| {
            f.requests
                .len()
                .max(if f.parse_errors.is_empty() { 0 } else { 1 })
        })
        .sum();
    let failures: usize = files
        .iter()
        .flat_map(|f| f.requests.iter())
        .filter(|r| r.error.is_none() && !r.passed)
        .count();
    let errors: usize = files
        .iter()
        .map(|f| {
            let req_errors = f.requests.iter().filter(|r| r.error.is_some()).count();
            req_errors + if f.parse_errors.is_empty() { 0 } else { 1 }
        })
        .sum();

    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuites name=\"restmd\" tests=\"{tests}\" failures=\"{failures}\" errors=\"{errors}\">\n"
    ));
    for suite in suites {
        out.push_str(&suite);
    }
    out.push_str("</testsuites>\n");
    out
}

fn junit_suite(file: &FileView) -> String {
    let mut cases = String::new();
    let mut tests = 0usize;
    let mut failures = 0usize;
    let mut errors = 0usize;

    if !file.parse_errors.is_empty() {
        tests += 1;
        errors += 1;
        let detail: String = file
            .parse_errors
            .iter()
            .map(|(line, col, msg)| format!("{}:{line}:{col}: {msg}", file.path))
            .collect::<Vec<_>>()
            .join("\n");
        cases.push_str(&format!(
            "    <testcase name=\"parse\" classname=\"{}\">\n      <error message=\"parse error\">{}</error>\n    </testcase>\n",
            xml_escape(&file.path),
            xml_escape(&detail),
        ));
    }

    let mut time = 0f64;
    for req in &file.requests {
        tests += 1;
        let secs = req.elapsed_ms.unwrap_or(0) as f64 / 1000.0;
        time += secs;
        let name = xml_escape(&format!("{} {}", req.method, req.url));
        let classname = xml_escape(&file.path);
        if let Some(err) = &req.error {
            errors += 1;
            cases.push_str(&format!(
                "    <testcase name=\"{name}\" classname=\"{classname}\" time=\"{secs:.3}\">\n      <error message=\"{}\"/>\n    </testcase>\n",
                xml_escape(err),
            ));
            continue;
        }

        let mut bodies = String::new();
        for a in req.assertions.iter().filter(|a| !a.passed) {
            let detail = a.detail.clone().unwrap_or_default();
            bodies.push_str(&format!(
                "      <failure message=\"{}\">{}</failure>\n",
                xml_escape(&a.description),
                xml_escape(&detail),
            ));
        }
        for cap in req.captures.iter().filter(|c| c.error.is_some()) {
            bodies.push_str(&format!(
                "      <failure message=\"capture {}\">{}</failure>\n",
                xml_escape(&cap.name),
                xml_escape(cap.error.as_deref().unwrap_or("")),
            ));
        }
        if bodies.is_empty() {
            cases.push_str(&format!(
                "    <testcase name=\"{name}\" classname=\"{classname}\" time=\"{secs:.3}\"/>\n"
            ));
        } else {
            failures += 1;
            cases.push_str(&format!(
                "    <testcase name=\"{name}\" classname=\"{classname}\" time=\"{secs:.3}\">\n{bodies}    </testcase>\n"
            ));
        }
    }

    format!(
        "  <testsuite name=\"{}\" tests=\"{tests}\" failures=\"{failures}\" errors=\"{errors}\" time=\"{time:.3}\">\n{cases}  </testsuite>\n",
        xml_escape(&file.path),
    )
}

/// Escape the five XML special characters so attributes and text are well-formed.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
