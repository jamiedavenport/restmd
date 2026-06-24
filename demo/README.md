# restmd demo

A runnable playground for the TUI.

Open two terminals from the repo root:

```sh
# 1. start the dev server (serves the endpoints the demo files target)
cargo run -p restmd-tui --bin restmd-devserver

# 2. open the TUI on the demo requests
cargo run -p restmd-cli -- demo/.restmd
```

In the TUI:

- `Tab` / `h` `l` — move focus between Files, Requests, Response
- `j` / `k` — move within the focused pane (or scroll the response)
- `Enter` — run the selected request (and the earlier ones it depends on)
- `o` — open the current file in `$EDITOR`
- `g` — force a rescan; `q` — quit

Edit a file under `demo/.restmd/` while the TUI is open and it refreshes
automatically. Try running the second request in `auth.md` — the login request
runs first so the token is available.
