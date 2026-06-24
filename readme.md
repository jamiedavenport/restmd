# `restmd`

A markdown-native REST client. Requests live in `.md` files — version-controlled,
diffable, and executable. See [`spec.md`](./spec.md) for the full design.

> Status: early. `restmd-core` parses documents into a typed tree; execution and
> the CLI/TUI/LSP surfaces are not built yet.

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
