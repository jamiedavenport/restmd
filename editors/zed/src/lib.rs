//! Zed extension: wire the `restmd-lsp` language server into Zed.
//!
//! Per Zed's guidelines the server binary is not bundled — it is located in the
//! user's environment. Install it with `cargo install --path crates/restmd-lsp`.

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
        let command = worktree.which("restmd-lsp").ok_or_else(|| {
            "`restmd-lsp` not found on PATH — run `cargo install --path crates/restmd-lsp`"
                .to_string()
        })?;
        Ok(Command {
            command,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(RestmdExtension);
