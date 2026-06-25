# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.4.0] - 2026-06-25

### Added

- `restmd run` sends requests headlessly and reports the results, for scripts
  and CI. With no path it runs every `.md` file in `./.restmd`; pass files or
  directories to narrow it down. Exit codes follow the spec (`0` success,
  `1` assertion failure, `2` parse error, `3` network error, `4` config error),
  and the most severe wins across files.
- Three output formats via `--format`: `pretty` (default, colored in a
  terminal), `json` (machine-readable report), and `junit` (XML test report for
  CI).
- `--env NAME` selects an `environments` block and `--var key=value` (repeatable)
  overrides variables — the first user-facing way to reach this machinery.
- `-r/--request` runs a single request by 1-based index or heading substring;
  the earlier requests it depends on still run so captures resolve.

## [0.3.1] - 2026-06-25

### Fixed

- `restmd lsp` now accepts the `--stdio` flag that LSP clients (e.g. VS Code)
  append to the server command, instead of exiting with a usage error. This
  unbreaks the VS Code integration (and any other standard LSP client).

## [0.3.0] - 2026-06-25

### Changed

- The language server now ships inside the `restmd` CLI and runs as `restmd lsp`.
  The standalone `restmd-lsp` binary is gone, so editor integrations only need
  the `restmd` binary installed. Editor configs that spawned `restmd-lsp` must
  switch to `restmd lsp` (VS Code, Zed, Neovim, and Helix examples are updated).
- Relicensed from `MIT OR Apache-2.0` to MIT only.

## [0.1.0] - 2026-06-24

### Added

- `restmd-core`: markdown parser, `{{var}}` resolution, and a blocking HTTP
  executor with captures and assertions.
- `restmd`: CLI that opens the TUI; `restmd init` scaffolds a `.restmd`
  directory with an example request.
- `restmd-tui`: interactive three-pane client (navigate, run, inspect) with live
  file watching; `o` opens the current file in `$EDITOR`.
- `restmd-lsp`: completion, diagnostics, document symbols, and hover for
  `.restmd` files, plus VS Code and Zed extensions.
- Distribution via cargo-dist: shell / PowerShell installers, a Homebrew tap,
  and a built-in self-updater.
