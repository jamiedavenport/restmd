//! `restmd check` — parse and validate `.md` request files without sending any
//! requests. Each [`ParseError`](restmd_core::ParseError) is reported as a
//! `path:line:col: message` line, and the process exits with code `2` (the
//! spec's "parse error") if any file has problems.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use restmd_core::parse;

use crate::files;

/// Run `check` over `paths`. Returns the process exit code: `0` if every file
/// parsed cleanly, `2` if any parse error was found.
pub fn run(paths: &[PathBuf]) -> Result<ExitCode> {
    let files = files::collect(paths)?;
    if files.is_empty() {
        println!("No .md files to check.");
        return Ok(ExitCode::SUCCESS);
    }

    let mut problems = 0usize;
    for path in &files {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parsed = parse(&source);
        for err in &parsed.errors {
            let (line, col) = err.span.line_col(&source);
            println!("{}:{line}:{col}: {}", path.display(), err.kind);
            problems += 1;
        }
    }

    let n = files.len();
    if problems == 0 {
        println!("Checked {n} file(s): no problems found.");
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("Found {problems} problem(s) across {n} file(s).");
        Ok(ExitCode::from(2))
    }
}
