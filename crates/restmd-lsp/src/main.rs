//! The `restmd-lsp` binary: speak LSP over stdio.

use lsp_server::Connection;

fn main() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();
    restmd_lsp::server::run(connection)?;
    io_threads.join()?;
    Ok(())
}
