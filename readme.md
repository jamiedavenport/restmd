# `restmd`

[![CI](https://github.com/jamiedavenport/restmd/actions/workflows/ci.yml/badge.svg)](https://github.com/jamiedavenport/restmd/actions/workflows/ci.yml)

A markdown-native REST client. Requests live in `.md` files — version-controlled,
diffable, and executable. See [`spec.md`](./spec.md) for the full design and
[`CLAUDE.md`](./CLAUDE.md) for the crate layout.

> Status: early but usable. `restmd-core` parses, resolves variables, and
> executes requests (captures, assertions). `restmd-tui` is an interactive
> three-pane client; `restmd` (CLI) opens the TUI. `restmd-lsp` provides editor
> completion/diagnostics/symbols/hover — see [`editors/`](./editors/README.md)
> for VS Code, Zed, Neovim, and Helix setup.

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
For editor support, also install the language server: `brew install jamiedavenport/tap/restmd-lsp`.

> The install commands work once the first release is published — see
> [Releasing](#releasing). Until then, build from source below.

## Quick start

```sh
cargo run -p restmd -- init   # scaffold ./.restmd with an example request
cargo run -p restmd            # open the TUI on ./.restmd
```

## Try the TUI

A runnable playground lives in [`demo/`](./demo/README.md). In two terminals:

```sh
cargo run -p restmd-tui --bin restmd-devserver   # 1. local server for the demo requests
cargo run -p restmd -- demo/.restmd          # 2. open the TUI
```

Navigate with `Tab`/`h`/`l` and `j`/`k`, press `Enter` to run a request (and the
earlier ones it depends on), `o` to open the current file in `$EDITOR`, `q` to
quit. Editing a file under `demo/.restmd/` refreshes the TUI live.

## Common commands

```sh
cargo build                 # build the workspace
cargo test                  # run all tests (unit + snapshot + doctests)
cargo test --test parse     # behavioural unit tests only
cargo clippy --all-targets  # lint
cargo fmt                   # format
```

### Snapshot tests

Parser output is locked down with [`insta`](https://insta.rs) snapshots under
`crates/restmd-core/tests/snapshots/`. After an intentional parser change:

```sh
cargo insta review          # review & accept changed snapshots (needs cargo-insta)
cargo insta test            # run tests and stage pending snapshots
```

Without `cargo-insta` installed, accept by re-running with `INSTA_UPDATE=always cargo test`.

## Releasing

Releases are built and published by [cargo-dist](https://opensource.axo.dev/cargo-dist/)
when a version tag is pushed. To cut a release:

```sh
# bump `version` under [workspace.package] in Cargo.toml, then:
git commit -am "Release 0.2.0"
git tag v0.2.0
git push origin main --tags
```

The tag triggers `.github/workflows/release.yml`, which cross-compiles `restmd`
and `restmd-lsp` for macOS/Linux/Windows, builds the installers, creates the
GitHub Release, and updates the Homebrew tap. Add a `## [0.2.0]` section to
`CHANGELOG.md` and cargo-dist uses it as the release notes. Preview a release
locally with `dist plan`.

One-time setup for the Homebrew publish: create the `jamiedavenport/homebrew-tap`
repository and add a `HOMEBREW_TAP_TOKEN` secret (a PAT with `repo` scope) so the
release job can push the formula.

> `cargo install cargo-release` lets you do bump + commit + tag + push in one
> `cargo release 0.2.0`.
