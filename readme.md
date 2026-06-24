# `restmd`

A markdown-native REST client. Requests live in `.md` files — version-controlled,
diffable, and executable. See [`spec.md`](./spec.md) for the full design and
[`CLAUDE.md`](./CLAUDE.md) for the crate layout.

> Status: early but usable. `restmd-core` parses, resolves variables, and
> executes requests (captures, assertions). `restmd-tui` is an interactive
> three-pane client; `restmd` (CLI) opens the TUI. `restmd-lsp` provides editor
> completion/diagnostics/symbols/hover — see [`editors/`](./editors/README.md)
> for VS Code, Zed, Neovim, and Helix setup.

## Quick start

```sh
cargo run -p restmd-cli -- init   # scaffold ./.restmd with an example request
cargo run -p restmd-cli            # open the TUI on ./.restmd
```

## Try the TUI

A runnable playground lives in [`demo/`](./demo/README.md). In two terminals:

```sh
cargo run -p restmd-tui --bin restmd-devserver   # 1. local server for the demo requests
cargo run -p restmd-cli -- demo/.restmd          # 2. open the TUI
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
