//! The `restmd` binary.
//!
//! With no subcommand it opens the TUI on a directory of `.restmd` request files
//! (default `./.restmd`). `restmd init` scaffolds that directory. `run`/`check`
//! are planned and will slot in as further subcommands.

mod init;

use std::path::PathBuf;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Init { dir }) => init::run(&dir),
        None => {
            let dir = cli.dir.unwrap_or_else(|| PathBuf::from(".restmd"));
            restmd_tui::run(dir)
        }
    }
}
