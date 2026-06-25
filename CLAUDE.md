# `restmd`

A markdown-native REST client (Cargo workspace). See [`spec.md`](./spec.md) for
the design and [`readme.md`](./readme.md) for common commands.

## Crates

| Crate | Path | Status | Responsibility |
|-------|------|--------|----------------|
| `restmd-core` | `crates/restmd-core` | active | Parser, document model, spans, errors, variable resolution, and the executor (behind the default `exec` feature). Source of truth all surfaces build on. |
| `restmd-tui` | `crates/restmd-tui` | active | `restmd-tui` binary (ratatui): three-pane navigate/run/inspect client + file watching. Also ships `restmd-devserver` (a tiny_http dev server) and the `devserver` module. |
| `restmd` | `crates/restmd` | active | `restmd` binary (clap): `restmd [DIR]` opens the TUI, `restmd init [DIR]` scaffolds a `.restmd` dir, `restmd check [PATHS]` validates files, `restmd format [PATHS] [--check]` canonicalizes them, `restmd lsp` runs the bundled language server over stdio; `run` planned. |
| `restmd-lsp` | `crates/restmd-lsp` | active | Library crate (lsp-server, sync, no tokio): completion, diagnostics, document symbols, hover for `.restmd` files. Does not execute requests. Has no binary of its own — the `restmd` CLI depends on it and exposes it as `restmd lsp`. |

Editor support: `editors/vscode` — a minimal VS Code extension that launches `restmd lsp` (F5 → Extension Dev Host; see its README).

Demo: `cargo run -p restmd-tui --bin restmd-devserver` + `cargo run -p restmd -- demo/.restmd` (see [`demo/README.md`](./demo/README.md)).

## Code Style

- Keep files to a maximum of 500 lines.
