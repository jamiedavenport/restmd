# `restmd`

A markdown-native REST client (Cargo workspace). See [`spec.md`](./spec.md) for
the design and [`readme.md`](./readme.md) for common commands.

## Crates

| Crate | Path | Status | Responsibility |
|-------|------|--------|----------------|
| `restmd-core` | `crates/restmd-core` | active | Parser, document model, spans, errors, variable resolution, and the executor (behind the default `exec` feature). Source of truth all surfaces build on. |
| `restmd-tui` | `crates/restmd-tui` | active | `restmd-tui` binary (ratatui): three-pane navigate/run/inspect client + file watching. Also ships `restmd-devserver` (a tiny_http dev server) and the `devserver` module. |
| `restmd-cli` | `crates/restmd-cli` | active | `restmd` binary (clap): currently `restmd [DIR]` opens the TUI; `run`/`check` planned. |
| `restmd-lsp` | `crates/restmd-lsp` | planned | `restmd-lsp` binary (tower-lsp): completion, hover, diagnostics. Does not execute requests. |

Demo: `cargo run -p restmd-tui --bin restmd-devserver` + `cargo run -p restmd-cli -- demo/.restmd` (see [`demo/README.md`](./demo/README.md)).

## Code Style

- Keep files to a maximum of 500 lines.
