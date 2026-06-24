# restmd — Zed extension

Wires the `restmd-lsp` language server into Zed.

## Install (development)

1. Put the server on your `PATH`:

   ```sh
   cargo install --path crates/restmd-lsp
   ```

2. In Zed: **`zed: install dev extension`** (command palette) and select this
   `editors/zed` directory. Zed compiles the extension to wasm and loads it.

3. Open a file under a `.restmd/` directory (e.g. `demo/.restmd/auth.md`) and try
   completion (`{{`, `## `, ` ``` `), diagnostics, and the outline.

## Scoping note

Zed attaches language servers by *language*, so this extension registers
`restmd-lsp` for **Markdown** — it sees every markdown file. That's fine: the
server self-scopes to files under a `.restmd/` directory and stays inert
elsewhere (no diagnostics, no completion). See the server's `is_restmd` filter.

If a future Zed version needs the server's language declared by the extension,
add a `languages/restmd/config.toml` with `grammar = "markdown"`.
