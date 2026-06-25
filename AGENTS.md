# restmd

restmd is a markdown-native REST client. Requests live in `.md` files under a
`.restmd/` directory and can be run from the CLI, TUI, CI, or any editor wired
to the bundled language server.

## Workspace

| Crate | Path | Responsibility |
|-------|------|----------------|
| `restmd-core` | `crates/restmd-core` | Parser, document model, spans, errors, variable resolution, and executor. Source of truth for every surface. |
| `restmd` | `crates/restmd` | Main CLI: TUI entrypoint, `init`, `check`, `format`, `run`, and `lsp`. |
| `restmd-tui` | `crates/restmd-tui` | Ratatui client, file watching, demo dev server, and TUI library used by the CLI. |
| `restmd-lsp` | `crates/restmd-lsp` | LSP library for completion, diagnostics, symbols, and hover. Exposed through `restmd lsp`. |

Editor integrations live under `editors/`. The VS Code and Zed integrations both
launch `restmd lsp`.

## Common Commands

```sh
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```

Demo flow:

```sh
cargo run -p restmd-tui --bin restmd-devserver
cargo run -p restmd -- demo/.restmd
```

## CLI Surface

- `restmd [DIR]` opens the TUI on a request directory, defaulting to `./.restmd`.
- `restmd init [DIR]` scaffolds an example request and default agent context.
- `restmd check [PATHS]` validates request files without sending HTTP.
- `restmd format [PATHS] [--check]` canonicalizes restmd syntax.
- `restmd run [PATHS] [-r REQUEST] [--env NAME] [--var k=v] [--format pretty|json|junit]` sends requests for scripts and CI.
- `restmd lsp` runs the language server over stdio for editor integrations.

## Project Conventions

- Keep source files below 500 lines when practical.
- Preserve the distinction between core model/parser behavior and CLI rendering.
- Do not make the LSP execute requests; execution belongs to the CLI/core runner.
- Do not overwrite user request or agent-context files from scaffolding commands.
- See `spec.md` for design intent and `README.md` for user-facing behavior.
