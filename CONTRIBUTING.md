# Contributing to `restmd`

Thanks for hacking on `restmd`! This covers building from source, testing, and
cutting releases. See [`spec.md`](./spec.md) for the design and
[`CLAUDE.md`](./CLAUDE.md) for the crate layout.

## Development

Work against the source checkout with `cargo run` instead of an installed binary:

```sh
cargo run -p restmd -- init   # scaffold ./.restmd with an example request
cargo run -p restmd           # open the TUI on ./.restmd
```

A runnable playground lives in [`demo/`](./demo/README.md). In two terminals:

```sh
cargo run -p restmd-tui --bin restmd-devserver   # 1. local server for the demo requests
cargo run -p restmd -- demo/.restmd              # 2. open the TUI
```

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
