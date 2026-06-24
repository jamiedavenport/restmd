//! `restmd format` — canonicalize the restmd-specific syntax in `.md` files.
//!
//! Formatting is *surgical*: it edits the original source through the byte
//! spans the parser records and never re-serializes the document model (which
//! drops the prose between requests). Three things are normalized:
//!
//! * **restmd syntax** — request headings (`##  GET` → `## GET`), header lines
//!   (`Name:value` → `Name: value`), and directives (`>capture x=$.y` →
//!   `> capture x = $.y`).
//! * **JSON bodies** — ` ```json ` fences whose content is valid JSON are
//!   re-indented with two spaces. Fences containing `{{templates}}` (and thus
//!   not valid JSON) are left untouched.
//! * **Whitespace** — trailing whitespace is stripped outside body fences, and
//!   the file ends with exactly one newline.
//!
//! A file with parse errors is left untouched; `restmd check` explains why.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use restmd_core::{Body, BodyLang, CaptureSource, Directive, Document, Header, Request, parse};

use crate::files;

/// Run `format` over `paths`. With `check_only`, files are not written and the
/// exit code reports whether any would change. Exit codes: `2` if any file was
/// skipped for parse errors, `1` if `--check` found a file that would change,
/// `0` otherwise.
pub fn run(paths: &[PathBuf], check_only: bool) -> Result<ExitCode> {
    let files = files::collect(paths)?;
    if files.is_empty() {
        println!("No .md files to format.");
        return Ok(ExitCode::SUCCESS);
    }

    let mut changed = 0usize;
    let mut skipped = 0usize;
    for path in &files {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        match format_source(&source) {
            None => {
                eprintln!(
                    "{}: skipped (parse errors — run `restmd check`)",
                    path.display()
                );
                skipped += 1;
            }
            Some(formatted) if formatted == source => {}
            Some(formatted) => {
                changed += 1;
                if check_only {
                    println!("would reformat {}", path.display());
                } else {
                    std::fs::write(path, formatted)
                        .with_context(|| format!("writing {}", path.display()))?;
                    println!("formatted {}", path.display());
                }
            }
        }
    }

    if skipped > 0 {
        Ok(ExitCode::from(2))
    } else if check_only && changed > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Produce the formatted form of `src`, or `None` if the source has parse
/// errors (in which case the caller leaves the file untouched).
fn format_source(src: &str) -> Option<String> {
    let parsed = parse(src);
    if !parsed.errors.is_empty() {
        return None;
    }
    let edits = collect_edits(src, &parsed.document);
    let (rebuilt, protected) = apply_edits(src, &edits);
    Some(hygiene(&rebuilt, &protected))
}

/// A replacement of `src[start..end]` with `text`. `protect`ed edits carry body
/// content and are exempt from trailing-whitespace stripping.
struct Edit {
    start: usize,
    end: usize,
    text: String,
    protect: bool,
}

/// Gather every edit implied by the document, in source order. The spans the
/// parser records for headings, headers, directives, and bodies never overlap,
/// so the edits compose cleanly.
fn collect_edits(src: &str, doc: &Document) -> Vec<Edit> {
    let mut edits = Vec::new();
    for req in &doc.requests {
        edits.push(heading_edit(src, req));
        for header in &req.headers {
            edits.push(header_edit(src, header));
        }
        for directive in &req.directives {
            edits.push(directive_edit(src, directive));
        }
        if let Some(body) = &req.body {
            edits.push(body_edit(src, body));
        }
    }
    edits.sort_by_key(|e| e.start);
    edits
}

/// `##  GET  /x` → `## GET /x`. The target is sliced verbatim so interpolations
/// are preserved exactly.
fn heading_edit(src: &str, req: &Request) -> Edit {
    let target = req.target.span.slice(src);
    Edit {
        start: req.heading_span.start,
        end: req.heading_span.end,
        text: format!("## {} {target}", req.method.as_str()),
        protect: false,
    }
}

/// `Name:value` → `Name: value`. The value is sliced verbatim.
fn header_edit(src: &str, header: &Header) -> Edit {
    let value = header.value.span.slice(src);
    Edit {
        start: header.span.start,
        end: header.span.end,
        text: format!("{}: {value}", header.name),
        protect: false,
    }
}

/// Normalize a directive's prefix and operator spacing. `capture` and `set` are
/// rebuilt around a single ` = ` (their operands never contain spaces, bar a
/// `set` value which is sliced verbatim); `assert` keeps its operand text
/// verbatim because it may contain quoted strings or `/regex/` literals.
fn directive_edit(src: &str, directive: &Directive) -> Edit {
    let (span, text) = match directive {
        Directive::Capture { name, source, span } => (
            *span,
            format!("> capture {name} = {}", capture_source(source)),
        ),
        Directive::Set { name, value, span } => {
            (*span, format!("> set {name} = {}", value.span.slice(src)))
        }
        Directive::Assert { span, .. } => {
            let body = span.slice(src).trim_start_matches('>').trim_start();
            let rest = body["assert".len()..].trim_start();
            let text = if rest.is_empty() {
                "> assert".to_string()
            } else {
                format!("> assert {rest}")
            };
            (*span, text)
        }
    };
    Edit {
        start: span.start,
        end: span.end,
        text,
        protect: false,
    }
}

/// The canonical source text of a capture's value source.
fn capture_source(source: &CaptureSource) -> String {
    match source {
        CaptureSource::JsonPath(path) => path.clone(),
        CaptureSource::Header(name) => format!("response.headers.{name}"),
        CaptureSource::Status => "response.status".to_string(),
    }
}

/// Replace a body fence. A `json` fence with valid-JSON content is re-indented;
/// every other body is emitted verbatim. Either way the edit is `protect`ed so
/// trailing whitespace inside the payload survives.
fn body_edit(src: &str, body: &Body) -> Edit {
    let fence = body.span.slice(src);
    let text = if body.lang == BodyLang::Json {
        pretty_json_fence(fence, &body.content).unwrap_or_else(|| fence.to_string())
    } else {
        fence.to_string()
    };
    Edit {
        start: body.span.start,
        end: body.span.end,
        text,
        protect: true,
    }
}

/// Re-indent the content of a JSON `fence` with two-space indentation, keeping
/// the opening and closing fence lines verbatim. Returns `None` if the content
/// is not valid JSON (e.g. it contains `{{templates}}`).
fn pretty_json_fence(fence: &str, content: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let pretty = serde_json::to_string_pretty(&value).ok()?;
    let open = fence.split('\n').next()?.trim_end_matches('\r');
    let close = fence.rsplit('\n').next()?.trim_end_matches('\r');
    Some(format!("{open}\n{pretty}\n{close}"))
}

/// Apply `edits` (sorted, non-overlapping) to `src`, returning the rebuilt
/// string and the output byte ranges of `protect`ed replacements.
fn apply_edits(src: &str, edits: &[Edit]) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(src.len());
    let mut protected = Vec::new();
    let mut cursor = 0;
    for edit in edits {
        out.push_str(&src[cursor..edit.start]);
        let start = out.len();
        out.push_str(&edit.text);
        if edit.protect {
            protected.push((start, out.len()));
        }
        cursor = edit.end;
    }
    out.push_str(&src[cursor..]);
    (out, protected)
}

/// Strip trailing whitespace from every line not inside a `protected` range and
/// ensure the result ends with exactly one newline.
fn hygiene(text: &str, protected: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut offset = 0;
    for raw in text.split_inclusive('\n') {
        let start = offset;
        offset += raw.len();
        let (content, newline) = match raw.strip_suffix('\n') {
            Some(c) => (c, true),
            None => (raw, false),
        };
        let end = start + content.len();
        let is_protected = protected.iter().any(|&(s, e)| start < e && s < end);
        if is_protected {
            out.push_str(content);
        } else {
            out.push_str(content.trim_end());
        }
        if newline {
            out.push('\n');
        }
    }

    let trimmed = out.trim_end_matches(['\n', '\r', ' ', '\t']);
    let mut result = trimmed.to_string();
    if !result.is_empty() {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::format_source;

    /// Formatting is idempotent: formatting an already-formatted file is a
    /// no-op.
    fn assert_idempotent(formatted: &str) {
        assert_eq!(
            format_source(formatted).as_deref(),
            Some(formatted),
            "not idempotent"
        );
    }

    #[test]
    fn normalizes_heading_header_and_directive_spacing() {
        let src = "##  GET   /health\nAccept:application/json\n\n>capture id=$.id\n";
        let out = format_source(src).unwrap();
        assert_eq!(
            out,
            "## GET /health\nAccept: application/json\n\n> capture id = $.id\n"
        );
        assert_idempotent(&out);
    }

    #[test]
    fn pretty_prints_json_bodies() {
        let src = "## POST /x\n\n```json\n{\"a\":1,\"b\":[2,3]}\n```\n";
        let out = format_source(src).unwrap();
        assert!(out.contains("```json\n{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ]\n}\n```"));
        assert_idempotent(&out);
    }

    #[test]
    fn leaves_non_json_templated_bodies_alone() {
        // An unquoted interpolation is not valid JSON, so the body is preserved
        // verbatim rather than risk mangling it.
        let src = "## POST /x\n\n```json\n{ \"id\": {{userId}} }\n```\n";
        assert_eq!(format_source(src).as_deref(), Some(src));
    }

    #[test]
    fn preserves_interpolations_inside_pretty_printed_json() {
        // A quoted interpolation *is* valid JSON, so the body is re-indented and
        // the `{{userId}}` token round-trips untouched inside the string.
        let src = "## POST /x\n\n```json\n{\"id\":\"{{userId}}\"}\n```\n";
        let out = format_source(src).unwrap();
        assert!(out.contains("```json\n{\n  \"id\": \"{{userId}}\"\n}\n```"));
        assert_idempotent(&out);
    }

    #[test]
    fn preserves_prose_and_trailing_whitespace_in_text_bodies() {
        let src = "# Docs\n\nSome prose.   \n\n## POST /x\n\n```text\nkeep me   \n```\n";
        let out = format_source(src).unwrap();
        // Prose trailing whitespace is stripped; body content is preserved.
        assert!(out.contains("Some prose.\n"));
        assert!(out.contains("keep me   \n"));
        assert_idempotent(&out);
    }

    #[test]
    fn collapses_trailing_blank_lines_to_one_newline() {
        let src = "## GET /\n\n\n\n";
        assert_eq!(format_source(src).as_deref(), Some("## GET /\n"));
    }

    #[test]
    fn skips_files_with_parse_errors() {
        // An unterminated frontmatter block is a parse error.
        assert_eq!(format_source("---\nbase: x\n"), None);
    }

    #[test]
    fn normalizes_set_and_status_assert() {
        let src = "## GET /\n>set token=abc\n> assert  status == 200\n";
        let out = format_source(src).unwrap();
        assert_eq!(out, "## GET /\n> set token = abc\n> assert status == 200\n");
        assert_idempotent(&out);
    }
}
