# `restmd`

[![CI](https://github.com/jamiedavenport/restmd/actions/workflows/ci.yml/badge.svg)](https://github.com/jamiedavenport/restmd/actions/workflows/ci.yml)

A markdown-native REST client. Requests live in `.md` files — version-controlled,
diffable, and executable. See [`spec.md`](./spec.md) for the full design.

> Status: early but usable. `restmd-core` parses, resolves variables, and
> executes requests (captures, assertions). `restmd-tui` is an interactive
> three-pane client; `restmd` (CLI) opens the TUI. `restmd lsp` runs the bundled
> language server (completion/diagnostics/symbols/hover) — see
> [`editors/`](./editors/README.md) for VS Code, Zed, Neovim, and Helix setup.

## Install

```sh
# macOS / Linux
curl -LsSf https://github.com/jamiedavenport/restmd/releases/latest/download/restmd-installer.sh | sh
# Homebrew (macOS / Linux)
brew install jamiedavenport/tap/restmd
# Windows (PowerShell)
powershell -c "irm https://github.com/jamiedavenport/restmd/releases/latest/download/restmd-installer.ps1 | iex"
```

Update later with `restmd-update`, `brew upgrade`, or by re-running the installer.
The `restmd` binary also bundles the language server (`restmd lsp`), so editor
support needs nothing extra — see [`editors/`](./editors/README.md) for setup.

## Quick start

```sh
restmd init   # scaffold ./.restmd with an example request
restmd        # open the TUI on ./.restmd
```

Navigate with `Tab`/`h`/`l` and `j`/`k`, press `Enter` to run a request (and the
earlier ones it depends on), `o` to open the current file in `$EDITOR`, `q` to
quit. Editing a file under `.restmd/` refreshes the TUI live. A runnable
playground lives in [`demo/`](./demo/README.md).

## Contributing

Building from source, tests, and releases are covered in
[`CONTRIBUTING.md`](./CONTRIBUTING.md).
