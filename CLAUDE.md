# `restmd`

A markdown-native REST client (Cargo workspace). See [`spec.md`](./spec.md) for
the design and [`readme.md`](./readme.md) for common commands.

## Crates

| Crate | Path | Status | Responsibility |
|-------|------|--------|----------------|
| `restmd-core` | `crates/restmd-core` | in progress | Parser, document model, spans, errors. Source of truth all surfaces build on. (Templating recorded but not resolved; no HTTP execution yet.) |
| `restmd-cli` | `crates/restmd-cli` | planned | `restmd` binary (clap): `run`, `check`, `ls`, `init`. |
| `restmd-tui` | `crates/restmd-tui` | planned | `restmd-tui` binary (ratatui): interactive three-pane client. |
| `restmd-lsp` | `crates/restmd-lsp` | planned | `restmd-lsp` binary (tower-lsp): completion, hover, diagnostics. Does not execute requests. |

## Code Style

- Keep files to a maximum of 500 lines.
