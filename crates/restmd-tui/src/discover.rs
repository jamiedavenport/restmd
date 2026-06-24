//! Discovering and parsing `.md` request files in a directory.

use std::path::{Path, PathBuf};

use restmd_core::{Document, ParseError, parse};

/// A discovered request file, parsed into a [`Document`].
pub struct LoadedFile {
    pub path: PathBuf,
    /// File name for display, e.g. `auth.md`.
    pub name: String,
    /// Raw source, kept so request sources can be sliced by span.
    pub source: String,
    pub document: Document,
    pub parse_errors: Vec<ParseError>,
}

/// Read and parse every `*.md` file directly inside `dir`, sorted by path. A
/// missing directory yields an empty list rather than an error.
pub fn discover(dir: &Path) -> std::io::Result<Vec<LoadedFile>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    paths.sort();

    for path in paths {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let parsed = parse(&source);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        files.push(LoadedFile {
            path,
            name,
            source,
            document: parsed.document,
            parse_errors: parsed.errors,
        });
    }

    Ok(files)
}
