//! The `restmd` binary.
//!
//! With no subcommand it opens the TUI on a directory of `.restmd` request files
//! (default `./.restmd`). `restmd init` scaffolds that directory, `restmd check`
//! validates files, `restmd format` canonicalizes them, and `restmd lsp` runs the
//! language server over stdio for editor integrations. `run` is planned and will
//! slot in as a further subcommand.

mod check;
mod files;
mod format;
mod init;

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
