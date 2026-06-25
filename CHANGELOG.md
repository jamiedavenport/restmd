# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
