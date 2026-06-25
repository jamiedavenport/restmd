# Editor setup for restmd

The language server ships inside the `restmd` CLI and runs as `restmd lsp`,
speaking LSP over **stdio**. It is editor-agnostic; what differs per editor is
how you launch it and how you **scope** it.

Install the CLI first (or build it to `target/debug/restmd`):

```sh
cargo install --path crates/restmd   # put `restmd` on PATH
cargo build -p restmd                # -> target/debug/restmd
```

## A note on scoping

restmd request files are plain `.md` files living in a `.restmd/` directory, so
they still render as markdown on GitHub/Obsidian. The downside is that **the
server has no way, on its own, to tell a restmd file from any other markdown** —
it analyzes whatever document the editor opens against it. Scoping is therefore
the editor's responsibility:

- **Path-glob editors** (VS Code, Neovim) can scope precisely to
  `**/.restmd/**`, so the server only attaches to those files.
- **Language-keyed editors** (Zed, Helix) attach a server to a *language* (e.g.
  Markdown), so the server runs on **all** markdown files. This is mostly benign
  — a normal markdown file has no requests, so there are no diagnostics — but
  completion will still trigger on `## ` and `{{`. If that bothers you, open an
  issue; a `--restmd-only` path filter in the server is a small follow-up.

## VS Code

See [`editors/vscode`](./vscode) — a ready extension, F5-runnable. Scoped to
`{ language: 'markdown', pattern: '**/.restmd/**' }`.

## Zed

A dev extension is provided in [`editors/zed`](./zed). Install the CLI
(`cargo install --path crates/restmd`), then run **`zed: install dev
extension`** and select `editors/zed`. It attaches `restmd lsp` to Markdown, and
the server self-scopes to `.restmd/` (see the scoping note), so ordinary markdown
is unaffected. Details in [`editors/zed/README.md`](./zed/README.md).

Reference: <https://zed.dev/docs/extensions/developing-extensions>

## Neovim (nvim-lspconfig or vim.lsp)

Path-scoped to `.restmd/` via the autocmd pattern:

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "markdown",
  callback = function(args)
    local name = vim.api.nvim_buf_get_name(args.buf)
    if not name:match("/%.restmd/") then return end
    vim.lsp.start({
      name = "restmd",
      cmd = { "restmd", "lsp" }, -- or an absolute path to target/debug/restmd
      root_dir = vim.fs.dirname(name),
    })
  end,
})
```

## Helix

In `languages.toml`, define a language server and attach it to Markdown (runs on
all markdown — see the scoping note):

```toml
[language-server.restmd]
command = "restmd"
args = ["lsp"]

[[language]]
name = "markdown"
language-servers = ["restmd"]
```
