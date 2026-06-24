//! The `restmd-tui` binary: open the TUI on a directory (default `./.restmd`).

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".restmd"));
    restmd_tui::run(dir)
}
