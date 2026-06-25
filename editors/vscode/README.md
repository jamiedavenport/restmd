# restmd — VS Code extension

Completion, diagnostics, document symbols, and hover for `.restmd` request files,
backed by the restmd language server (`restmd lsp`). The server engages only
inside `.restmd/` directories, so files still render as plain markdown everywhere
else.

## Try it (development)

From the repo root:

```sh
cargo build -p restmd               # build the CLI (ships the language server)
cd editors/vscode
npm install && npm run compile      # build the extension
```

Then open `editors/vscode` in VS Code and press **F5**. This launches an
Extension Development Host that opens the repo's `demo/` folder with the server
wired up (via `RESTMD_PATH`). Open `demo/.restmd/auth.md` and:

- type `{{` to get variable + builtin completion (e.g. `token`, `userId`, `uuid()`);
- start a heading `## ` for method completion; open a ` ``` ` fence for body languages;
- mistype a variable to see an "unknown variable" warning;
- use the outline view to jump between requests.

## Installed use

Put the `restmd` CLI on your `PATH` (e.g. `cargo install --path crates/restmd`),
package the extension with `vsce package`, and install the `.vsix`. Override the
server location with the `restmd.serverPath` setting if needed.
