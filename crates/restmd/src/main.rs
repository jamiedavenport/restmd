//! The `restmd` binary.
//!
//! With no subcommand it opens the TUI on a directory of `.restmd` request files
//! (default `./.restmd`). `restmd init` scaffolds that directory, `restmd check`
//! validates files, `restmd format` canonicalizes them, `restmd run` sends the
//! requests and reports results for scripts/CI, and `restmd lsp` runs the
//! language server over stdio for editor integrations.

mod check;
mod files;
mod format;
mod init;
mod run;
mod run_render;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// A markdown-native REST client.
#[derive(Parser)]
#[command(name = "restmd", version, about)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Directory of `.md` request files to open in the TUI (default `./.restmd`).
    dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a `.restmd` directory with an example request.
    Init {
        /// Directory to create (default `./.restmd`).
        #[arg(default_value = ".restmd")]
        dir: PathBuf,
    },
    /// Parse and validate request files without sending requests.
    Check {
        /// Files or directories to check (default `.restmd`).
        #[arg(default_value = ".restmd")]
        paths: Vec<PathBuf>,
    },
    /// Canonicalize the formatting of request files in place.
    Format {
        /// Files or directories to format (default `.restmd`).
        #[arg(default_value = ".restmd")]
        paths: Vec<PathBuf>,
        /// Report whether files are formatted without writing changes; exits
        /// non-zero if any file would change.
        #[arg(long)]
        check: bool,
    },
    /// Send requests and report results (headless / CI).
    Run {
        /// Files or directories to run (default `.restmd`).
        #[arg(default_value = ".restmd")]
        paths: Vec<PathBuf>,
        /// Run a single request: a 1-based index or a heading substring (only
        /// valid when exactly one file is resolved).
        #[arg(short = 'r', long = "request")]
        request: Option<String>,
        /// Environment block to select from frontmatter.
        #[arg(long)]
        env: Option<String>,
        /// Override a variable (repeatable): `--var key=value`.
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = run::OutputFormat::Pretty)]
        format: run::OutputFormat,
    },
    /// Run the language server over stdio (used by editor extensions).
    Lsp {
        /// Accepted for compatibility with LSP clients (e.g. VS Code) that pass
        /// `--stdio`. stdio is the only transport, so this is a no-op.
        #[arg(long)]
        stdio: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(Command::Init { dir }) => init::run(&dir).map(|()| ExitCode::SUCCESS),
        Some(Command::Check { paths }) => check::run(&paths),
        Some(Command::Format { paths, check }) => format::run(&paths, check),
        Some(Command::Run {
            paths,
            request,
            env,
            vars,
            format,
        }) => run::run(&paths, request.as_deref(), env, &vars, format),
        Some(Command::Lsp { .. }) => restmd_lsp::run_stdio().map(|()| ExitCode::SUCCESS),
        None => {
            let dir = cli.dir.unwrap_or_else(|| PathBuf::from(".restmd"));
            restmd_tui::run(dir).map(|()| ExitCode::SUCCESS)
        }
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
