//! `restmd init` — scaffold a `.restmd` directory with an example request.

use std::path::Path;

use anyhow::{Context, Result, bail};

/// The example written by `init`: a health check against jxd.dev.
const EXAMPLE: &str = "\
---
base: https://jxd.dev
---

# Example requests

A starting point. Select a request and run it; this one expects a 200.

## GET /

> assert status == 200
";

/// Create `dir` (and parents) and write `example.md` into it. Refuses to
/// overwrite an existing example.
pub fn run(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("creating directory {}", dir.display()))?;

    let example = dir.join("example.md");
    if example.exists() {
        bail!("{} already exists — not overwriting", example.display());
    }
    std::fs::write(&example, EXAMPLE).with_context(|| format!("writing {}", example.display()))?;

    println!("Created {}", example.display());
    println!("Open it with `restmd {}`.", dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_valid_example() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        run(&dir).unwrap();

        let example = dir.join("example.md");
        let source = std::fs::read_to_string(&example).unwrap();
        assert!(source.contains("jxd.dev"));
        assert!(source.contains("assert status == 200"));

        // The scaffolded file must parse cleanly into one request.
        let parsed = restmd_core::parse(&source);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.document.requests.len(), 1);
    }

    #[test]
    fn refuses_to_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        run(&dir).unwrap();
        assert!(run(&dir).is_err(), "second init should fail");
    }
}
