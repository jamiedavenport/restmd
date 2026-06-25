//! `restmd init` — scaffold a `.restmd` directory with an example request.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};

const EXAMPLE: &str = include_str!("../templates/example.md");
const AGENT_GUIDE: &str = include_str!("../templates/AGENTS.md");
const CLAUDE_GUIDE: &str = include_str!("../templates/CLAUDE.md");
const GEMINI_GUIDE: &str = include_str!("../templates/GEMINI.md");
const GEMINI_SETTINGS: &str = include_str!("../templates/gemini-settings.json");
const CURSOR_RULES: &str = include_str!("../templates/cursor-restmd.mdc");

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
    write_agent_context(dir, &detect_coding_agents(dir))?;
    println!("Open it with `restmd {}`.", dir.display());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CodingAgent {
    Codex,
    Claude,
    Gemini,
    Cursor,
}

fn detect_coding_agents(dir: &Path) -> BTreeSet<CodingAgent> {
    let mut agents = detect_coding_agents_from_env(std::env::vars());
    detect_coding_agents_from_files(dir, &mut agents);
    agents
}

fn detect_coding_agents_from_env<I, K, V>(vars: I) -> BTreeSet<CodingAgent>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut agents = BTreeSet::new();
    for (key, value) in vars {
        let key = key.as_ref();
        let value = value.as_ref();
        if key.starts_with("CODEX_") {
            agents.insert(CodingAgent::Codex);
        }
        if key.starts_with("CLAUDE") {
            agents.insert(CodingAgent::Claude);
        }
        if key.starts_with("GEMINI_CLI") || key == "GEMINI_SANDBOX" {
            agents.insert(CodingAgent::Gemini);
        }
        if key.starts_with("CURSOR_")
            || (key == "TERM_PROGRAM" && value.eq_ignore_ascii_case("cursor"))
        {
            agents.insert(CodingAgent::Cursor);
        }
    }
    agents
}

fn detect_coding_agents_from_files(dir: &Path, agents: &mut BTreeSet<CodingAgent>) {
    let Some(project_dir) = dir.parent() else {
        return;
    };
    for ancestor in project_dir.ancestors() {
        if ancestor.join("AGENTS.md").exists() {
            agents.insert(CodingAgent::Codex);
        }
        if ancestor.join("CLAUDE.md").exists() {
            agents.insert(CodingAgent::Claude);
        }
        if ancestor.join("GEMINI.md").exists()
            || ancestor.join(".gemini").join("settings.json").exists()
        {
            agents.insert(CodingAgent::Gemini);
        }
        if ancestor.join(".cursor").exists() {
            agents.insert(CodingAgent::Cursor);
        }
    }
}

fn write_agent_context(dir: &Path, agents: &BTreeSet<CodingAgent>) -> Result<()> {
    if agents.is_empty() {
        println!("No coding agent detected; skipped agent context.");
        return Ok(());
    }

    write_if_missing(&dir.join("AGENTS.md"), AGENT_GUIDE)?;
    if agents.contains(&CodingAgent::Claude) {
        write_if_missing(&dir.join("CLAUDE.md"), CLAUDE_GUIDE)?;
    }
    if agents.contains(&CodingAgent::Gemini) {
        write_if_missing(&dir.join("GEMINI.md"), GEMINI_GUIDE)?;
        write_if_missing(&dir.join(".gemini").join("settings.json"), GEMINI_SETTINGS)?;
    }
    if agents.contains(&CodingAgent::Cursor) {
        write_if_missing(
            &dir.join(".cursor").join("rules").join("restmd.mdc"),
            CURSOR_RULES,
        )?;
    }
    Ok(())
}

fn write_if_missing(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        println!("Skipped existing {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    println!("Created {}", path.display());
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
    fn creates_agent_context_for_detected_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        std::fs::create_dir_all(&dir).unwrap();
        let agents = BTreeSet::from([
            CodingAgent::Codex,
            CodingAgent::Claude,
            CodingAgent::Gemini,
            CodingAgent::Cursor,
        ]);
        write_agent_context(&dir, &agents).unwrap();

        assert!(dir.join("AGENTS.md").exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
            "@AGENTS.md\n"
        );
        assert!(dir.join("GEMINI.md").exists());
        assert!(dir.join(".gemini").join("settings.json").exists());
        assert!(
            dir.join(".cursor")
                .join("rules")
                .join("restmd.mdc")
                .exists()
        );

        let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents.contains("restmd check ."));
        assert!(agents.contains("Do not invent secrets"));
    }

    #[test]
    fn skips_agent_context_when_no_agent_is_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        std::fs::create_dir_all(&dir).unwrap();

        write_agent_context(&dir, &BTreeSet::new()).unwrap();

        assert!(!dir.join("AGENTS.md").exists());
        assert!(!dir.join("CLAUDE.md").exists());
        assert!(!dir.join("GEMINI.md").exists());
        assert!(!dir.join(".cursor").exists());
    }

    #[test]
    fn codex_detection_creates_only_portable_agent_context() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        std::fs::create_dir_all(&dir).unwrap();

        write_agent_context(&dir, &BTreeSet::from([CodingAgent::Codex])).unwrap();

        assert!(dir.join("AGENTS.md").exists());
        assert!(!dir.join("CLAUDE.md").exists());
        assert!(!dir.join("GEMINI.md").exists());
        assert!(!dir.join(".gemini").exists());
        assert!(!dir.join(".cursor").exists());
    }

    #[test]
    fn does_not_overwrite_existing_agent_context() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("AGENTS.md"), "custom guidance\n").unwrap();
        std::fs::create_dir_all(dir.join(".cursor").join("rules")).unwrap();
        std::fs::write(
            dir.join(".cursor").join("rules").join("restmd.mdc"),
            "custom cursor rule\n",
        )
        .unwrap();
        let agents = BTreeSet::from([CodingAgent::Codex, CodingAgent::Cursor]);

        write_agent_context(&dir, &agents).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.join("AGENTS.md")).unwrap(),
            "custom guidance\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(".cursor").join("rules").join("restmd.mdc")).unwrap(),
            "custom cursor rule\n"
        );
    }

    #[test]
    fn detects_agents_from_environment() {
        let agents = detect_coding_agents_from_env([
            ("CODEX_SANDBOX", "seatbelt"),
            ("CLAUDECODE", "1"),
            ("GEMINI_CLI", "1"),
            ("TERM_PROGRAM", "cursor"),
            ("OPENAI_API_KEY", "not-an-agent-signal"),
            ("ANTHROPIC_API_KEY", "not-an-agent-signal"),
            ("GEMINI_API_KEY", "not-an-agent-signal"),
        ]);

        assert!(agents.contains(&CodingAgent::Codex));
        assert!(agents.contains(&CodingAgent::Claude));
        assert!(agents.contains(&CodingAgent::Gemini));
        assert!(agents.contains(&CodingAgent::Cursor));
        assert_eq!(agents.len(), 4);
    }

    #[test]
    fn detects_agents_from_existing_project_files() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        let dir = project.join(".restmd");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(project.join("AGENTS.md"), "").unwrap();
        std::fs::write(project.join("CLAUDE.md"), "").unwrap();
        std::fs::create_dir_all(project.join(".cursor")).unwrap();

        let mut agents = BTreeSet::new();
        detect_coding_agents_from_files(&dir, &mut agents);

        assert!(agents.contains(&CodingAgent::Codex));
        assert!(agents.contains(&CodingAgent::Claude));
        assert!(agents.contains(&CodingAgent::Cursor));
        assert!(!agents.contains(&CodingAgent::Gemini));
    }

    #[test]
    fn refuses_to_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".restmd");
        run(&dir).unwrap();
        assert!(run(&dir).is_err(), "second init should fail");
    }
}
