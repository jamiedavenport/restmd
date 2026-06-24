//! Resolving the path arguments shared by `check` and `format` into a concrete
//! list of `.md` files.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Expand the user-supplied `paths` into a sorted, de-duplicated list of files.
/// A directory contributes its direct `.md` children (matching the TUI's
/// discovery in `restmd-tui`); a path that is not a directory is taken as-is so
/// an explicit file argument is honoured even with a non-`.md` extension.
pub fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_dir(path, &mut files)?;
        } else {
            files.push(path.clone());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Append the direct `.md` children of `dir` to `files`.
fn collect_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?;
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading directory {}", dir.display()))?
            .path();
        if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    Ok(())
}
