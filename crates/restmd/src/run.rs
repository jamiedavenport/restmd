//! `restmd run` — send the requests in one or more `.md` files and report the
//! results, for use in scripts and CI.
//!
//! Like [`check`](crate::check) and [`format`](crate::format), the positional
//! argument is a list of files or directories (default `.restmd`); every `.md`
//! file found is run. The process exit code follows the spec: `0` success,
//! `1` assertion/capture failure, `2` parse error, `3` network error, `4`
//! config error (unknown environment, malformed `--var`, bad `--request`
//! selector, or an unresolved template). When several files run, the most
//! severe code wins.
//!
//! `--request` selects a single request within one file. Requests have no
//! names, so the selector is either a 1-based index or a case-insensitive
//! substring of the request heading. The *prefix* up to and including the
//! selected request is executed (so captures from earlier requests resolve,
//! matching the TUI), but only the selected request is reported.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use restmd_core::{Document, ParseError, ReqwestTransport, RunOptions, RunReport, Runner, parse};

use crate::files;
use crate::run_render;

/// How `restmd run` renders its results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable, coloured when stdout is a terminal.
    Pretty,
    /// Machine-readable JSON report.
    Json,
    /// JUnit XML test report, for CI.
    Junit,
}

/// What happened for one input file.
pub enum FileOutcome {
    /// The file parsed; `report` holds the execution result. `selected` is the
    /// request index chosen by `--request`, if any.
    Ran {
        doc: Box<Document>,
        report: Box<RunReport>,
        selected: Option<usize>,
    },
    /// The file failed to parse; no requests were sent.
    ParseErrors(Vec<ParseError>),
}

/// One file's result, paired with its source (needed for heading names and
/// parse-error line/column lookup).
pub struct FileRun {
    pub path: PathBuf,
    pub source: String,
    pub outcome: FileOutcome,
}

impl FileRun {
    /// This file's exit code in isolation. Parse errors are `2`; otherwise the
    /// report decides (`0`/`1`/`3`/`4`).
    pub fn exit_code(&self) -> i32 {
        match &self.outcome {
            FileOutcome::ParseErrors(_) => 2,
            FileOutcome::Ran { report, .. } => report.exit_code(),
        }
    }
}

/// Run `paths`, rendering results in `format`. See the module docs for the exit
/// code contract.
pub fn run(
    paths: &[PathBuf],
    request: Option<&str>,
    env: Option<String>,
    vars: &[String],
    format: OutputFormat,
) -> Result<ExitCode> {
    let files = files::collect(paths)?;
    if files.is_empty() {
        println!("No .md files to run.");
        return Ok(ExitCode::SUCCESS);
    }

    // `--request` only makes sense against a single file.
    if request.is_some() && files.len() != 1 {
        eprintln!(
            "error: --request selects a request within a single file, but {} files were found",
            files.len()
        );
        return Ok(ExitCode::from(4));
    }

    let vars = match parse_vars(vars) {
        Ok(vars) => vars,
        Err(msg) => {
            eprintln!("error: {msg}");
            return Ok(ExitCode::from(4));
        }
    };
    let opts = RunOptions {
        env,
        vars,
        include_os_env: true,
    };

    let mut runs = Vec::with_capacity(files.len());
    for path in &files {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parsed = parse(&source);
        let outcome = if !parsed.errors.is_empty() {
            FileOutcome::ParseErrors(parsed.errors)
        } else {
            let doc = parsed.document;
            // A transport instance is one in-memory cookie session. Keep that
            // session scoped to this document so unrelated files cannot share
            // authentication state.
            let runner = Runner::new(ReqwestTransport::new());
            match request {
                Some(sel) => match resolve_selector(sel, &doc, &source) {
                    Ok(index) => {
                        let report = runner.run_through(&doc, index, &opts);
                        FileOutcome::Ran {
                            doc: Box::new(doc),
                            report: Box::new(report),
                            selected: Some(index),
                        }
                    }
                    Err(msg) => {
                        eprintln!("error: {}: {msg}", path.display());
                        return Ok(ExitCode::from(4));
                    }
                },
                None => {
                    let report = runner.run(&doc, &opts);
                    FileOutcome::Ran {
                        doc: Box::new(doc),
                        report: Box::new(report),
                        selected: None,
                    }
                }
            }
        };
        runs.push(FileRun {
            path: path.clone(),
            source,
            outcome,
        });
    }

    let code = runs.iter().map(FileRun::exit_code).max().unwrap_or(0);
    let output = run_render::render(format, &runs, code);
    print!("{output}");

    Ok(ExitCode::from(code as u8))
}

/// Parse repeated `--var key=value` flags into a map. Returns an error message
/// (not an `anyhow::Error`, so the caller can map it to exit code `4`) when an
/// entry is missing its `=`.
fn parse_vars(raw: &[String]) -> std::result::Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    for item in raw {
        let (key, value) = item
            .split_once('=')
            .ok_or_else(|| format!("invalid --var `{item}`: expected key=value"))?;
        map.insert(key.to_string(), value.to_string());
    }
    Ok(map)
}

/// Resolve a `--request` selector to a request index. The selector is a 1-based
/// index or a case-insensitive substring of a request heading; an empty
/// document, an out-of-range index, or an ambiguous/absent substring is an
/// error (message describes why).
fn resolve_selector(
    selector: &str,
    doc: &Document,
    source: &str,
) -> std::result::Result<usize, String> {
    if doc.requests.is_empty() {
        return Err("no requests in file".to_string());
    }
    if let Ok(n) = selector.parse::<usize>() {
        if n == 0 || n > doc.requests.len() {
            return Err(format!(
                "request index {n} out of range (1..={})",
                doc.requests.len()
            ));
        }
        return Ok(n - 1);
    }

    let needle = selector.to_lowercase();
    let matches: Vec<usize> = (0..doc.requests.len())
        .filter(|&i| {
            request_name(doc, source, i)
                .to_lowercase()
                .contains(&needle)
        })
        .collect();
    match matches.as_slice() {
        [i] => Ok(*i),
        [] => Err(format!("no request matching `{selector}`")),
        many => {
            let names: Vec<String> = many.iter().map(|&i| request_name(doc, source, i)).collect();
            Err(format!(
                "`{selector}` is ambiguous; matches: {}",
                names.join(", ")
            ))
        }
    }
}

/// The display name of request `i`: its heading with the leading `#`s and
/// surrounding whitespace trimmed (matching the TUI request list).
pub fn request_name(doc: &Document, source: &str, i: usize) -> String {
    doc.requests[i]
        .heading_span
        .slice(source)
        .trim_start_matches('#')
        .trim()
        .to_string()
}
