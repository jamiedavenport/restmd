//! The `restmd` binary.
//!
//! For now it has a single behaviour: open the TUI on a directory of `.restmd`
//! request files (default `./.restmd`). `run`/`check` subcommands are planned
//! and will slot in as variants here.

use std::path::PathBuf;

use clap::Parser;

/// A markdown-native REST client.
#[derive(Parser)]
#[command(name = "restmd", version, about)]
struct Cli {
    /// Directory of `.md` request files to open in the TUI.
    #[arg(default_value = ".restmd")]
    dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    restmd_tui::run(cli.dir)
}
