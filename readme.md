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

## Writing requests

A restmd file is an ordinary markdown `.md` file (kept in a `.restmd/`
directory). It has three layers: an optional **frontmatter** block of file-level
config, free-form **prose** that the runner ignores, and one or more
**requests**. Here is a complete file:

````markdown
---
base: http://127.0.0.1:8787
defaults:
  Accept: application/json
---

# Auth flow

Log in, capture the token, then make an authenticated request that depends on
it. Running the second request runs the first one too.

## POST /auth/login
Content-Type: application/json

```json
{ "email": "ada@example.com", "password": "hunter2" }
```

> capture token  = $.access_token
> capture userId = $.user.id
> assert  status == 200
> assert  $.access_token exists

## GET /users/{{userId}}
Authorization: Bearer {{token}}

> assert status == 200
> assert $.active == true
````

### Frontmatter

An optional YAML block at the very top, fenced by `---`. Unknown keys are
rejected, so typos surface as errors. Recognized keys:

| Key            | Meaning                                                        |
|----------------|---------------------------------------------------------------|
| `base`         | Base URL prepended to relative request paths (may be templated). |
| `defaults`     | Headers merged into every request unless the request overrides them. |
| `environments` | Named variable sets, e.g. `dev:` / `prod:`, selectable per run. |
| `openapi`      | Path or URL of an OpenAPI spec, for completion/validation.     |
| `timeout`      | Default per-request timeout, e.g. `30s`.                       |
| `retries`      | Default retry count for idempotent methods.                   |

### Requests

Each request is an H2 heading of the form `## METHOD /path`, where `METHOD` is
one of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, or `OPTIONS`. An H2 whose
first word isn't a method is treated as prose, not a broken request. The path is
relative to `base` and may carry a query and fragment. A request runs from its
heading until the next H2, the next H1, or end of file.

Directly under the heading, optional **header lines** (`Name: value`) and an
optional **body fence** make up the request:

- ` ```json ` — parsed and re-sent; defaults `Content-Type: application/json`.
- ` ```xml ` — sent as-is; defaults `Content-Type: application/xml`.
- ` ```form ` — `key: value` lines sent as form data.
- ` ```text ` — sent verbatim (set `Content-Type` yourself).
- ` ```graphql ` — wrapped into a GraphQL JSON payload.

A fence in any other language (e.g. ` ```rust `) is left as prose, so
documentation snippets don't get sent.

### Directives

Lines beginning with `>` (markdown blockquotes) attach to the request above
them. They run after the response comes back:

- `> capture NAME = $.json.path` — save a value for later requests. The source
  can also be `response.headers.HeaderName` or `response.status`.
- `> assert status == 200` — assert on the status code.
- `> assert $.json.path OP value` — assert on the body. Operators: `==`, `!=`,
  `<`, `>`, `<=`, `>=`, `exists`, and `matches /regex/`.
- `> set NAME = value` — bind a variable without sending a request.

### Variables

`{{name}}` interpolates a variable anywhere in a path, header, or body. Lookup
is first-match-wins in this order:

1. Values `capture`d (or `set`) by earlier requests in the run.
2. `RESTMD_VAR_<NAME>` environment variables.
3. The selected `environments` block from the frontmatter.

Two modifiers help with missing values: `{{name?}}` resolves to an empty string
instead of erroring, and `{{name!fallback}}` uses `fallback` when unset.

Built-in functions are also available: `{{uuid()}}`, `{{now()}}`,
`{{timestamp()}}`, `{{base64(var)}}`, and `{{env(NAME)}}` (reads `NAME` straight
from the OS environment).

## Contributing

Building from source, tests, and releases are covered in
[`CONTRIBUTING.md`](./CONTRIBUTING.md).
