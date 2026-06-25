//! Zed extension: wire the restmd language server into Zed.
//!
//! The server ships inside the `restmd` CLI and runs as `restmd lsp`. Per Zed's
//! guidelines the binary is not bundled — it is located in the user's
//! environment. Install it with `cargo install --path crates/restmd`.

use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct RestmdExtension;

impl zed::Extension for RestmdExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let command = worktree.which("restmd").ok_or_else(|| {
            "`restmd` not found on PATH — run `cargo install --path crates/restmd`".to_string()
        })?;
        Ok(Command {
            command,
            args: vec!["lsp".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(RestmdExtension);
