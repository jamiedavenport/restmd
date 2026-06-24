# restmd — Specification (v0.1 draft)

> A markdown-native REST client. Requests live in `.md` files, version-controlled, diffable, and executable from the CLI, a TUI, or any LSP-aware editor.

## 1. Goals and non-goals

### Goals
- A REST client where the source of truth is plain markdown that renders cleanly in any viewer (GitHub, Obsidian, VS Code preview).
- One DSL, three surfaces: CLI for CI and ad-hoc use, TUI for interactive exploration, LSP for editor integration.
- First-class OpenAPI integration: point a file at a spec, get completion, validation, and request scaffolding for free.
- Files-on-disk as the only state. No database, no cloud sync, no account.
- Reliable for CI: deterministic output, machine-readable result formats, non-zero exit on assertion failure.

### Non-goals (v1)
- GUI application. Editor extensions cover the visual editing surface.
- Embedded scripting language (Lua, JS, etc.). Captures and assertions cover the 80% case; users who need scripting can shell out.
- Protocols beyond HTTP/1.1 and HTTP/2 (no gRPC, GraphQL-specific tooling, WebSockets, MQTT) in v1.
- Team collaboration features. Git is the collaboration layer.
- Mocking or stub servers.

### Anti-goals
- Becoming Postman. Feature parity is not the target; the wedge is text-first and editor-native.
- Database-backed history or workspaces. Response history, if added, is a directory of files.

## 2. The wedge

Existing alternatives and what we beat them on:

- **Postman / Insomnia**: GUI-locked, cloud-coupled, opaque storage. We win on version control, editor integration, and CI.
- **Bruno**: text-first but custom `.bru` format requires their app to render. We render in any markdown viewer.
- **REST Client (VS Code), httpYac, JetBrains HTTP**: editor-bound `.http` format, no standalone CLI/TUI story, weaker OpenAPI integration. We win on portability across editors and surfaces.
- **Hurl**: CLI-only, separate format, no interactive surface. We win on editor integration and the markdown-as-documentation property.

The sharpest single differentiator: **literate API requests**. A `.md` file is simultaneously executable requests and human documentation.

## 3. Architecture overview

Single Cargo workspace, multiple crates:

```
restmd/
├── crates/
│   ├── restmd-core/      # Parser, types, env resolution, executor
│   ├── restmd/           # `restmd` binary (clap)
│   ├── restmd-tui/       # `restmd-tui` binary (ratatui)
│   └── restmd-lsp/       # `restmd-lsp` binary (tower-lsp)
├── editors/
│   ├── vscode/           # TypeScript extension, bundles restmd-lsp
│   └── jetbrains/        # Plugin (post-v1)
└── Cargo.toml
```

The core crate is the single source of truth for parsing, type definitions, environment resolution, and HTTP execution. The three binaries are thin surfaces over it. When the DSL evolves, only the core changes.

## 4. The DSL

### 4.1 File structure

A restmd file is a markdown document with three layers:
1. **Frontmatter** (YAML) — file-level config: base URL, environments, defaults, OpenAPI reference.
2. **Prose** — standard markdown. Ignored by the executor, present for humans.
3. **Requests** — H2 headings of the form `## METHOD /path`, optionally followed by headers, a body fence, and directive blockquotes.

### 4.2 Worked example

````markdown
---
base: https://api.{{env}}.example.com
openapi: ./openapi.yaml
environments:
  dev:
    env: dev-api
    workspace_id: ws_dev_abc123
  prod:
    env: api
    workspace_id: ws_prod_xyz789
defaults:
  Accept: application/json
  User-Agent: restmd/0.1
  Authorization: Bearer {{token}}
---

# Project Management API

Auth flow first, then CRUD on projects within a workspace.

## POST /auth/login
Content-Type: application/json

```json
{
  "email": "{{email}}",
  "password": "{{password}}"
}
```

> capture token  = $.access_token
> capture userId = $.user.id
> assert  status == 200
> assert  $.access_token exists

## GET /workspaces/{{workspace_id}}/projects?status=active&limit=50

## POST /workspaces/{{workspace_id}}/projects
Content-Type: application/json
Idempotency-Key: {{uuid()}}

```json
{ "name": "Q4 Launch", "members": ["{{userId}}"] }
```

> capture projectId = $.id

## DELETE /workspaces/{{workspace_id}}/projects/{{projectId}}

> assert status == 204
````

### 4.3 Frontmatter schema

| Key            | Type                       | Required | Meaning                                                  |
|----------------|----------------------------|----------|----------------------------------------------------------|
| `base`         | string (templated)         | no       | Base URL prepended to relative request paths.            |
| `openapi`      | string (path or URL)       | no       | OpenAPI spec for completion and validation.              |
| `environments` | map<string, map<string,_>> | no       | Named variable sets, selected with `--env <name>`.       |
| `defaults`     | map<string, string>        | no       | Headers merged into every request unless overridden.     |
| `timeout`      | duration (e.g. `30s`)      | no       | Default per-request timeout.                             |
| `retries`      | int                        | no       | Default retry count for idempotent methods.              |

### 4.4 Request grammar (informal)

```
request    := h2_heading newline header_line* body_fence? directive_block*
h2_heading := "##" SP method SP path query? fragment?
method     := "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
header_line:= header_name ":" SP header_value
body_fence := "```" lang newline body_content newline "```"
lang       := "json" | "xml" | "form" | "text" | "graphql"
directive  := "> " ("capture" | "assert" | "set") SP expression
```

A request ends at the next H2, the next H1, or EOF.

### 4.5 Variables and templating

`{{var}}` substitutes a variable. Lookup order:
1. Captured values from earlier requests in the same run.
2. CLI `--var key=value` flags.
3. Environment variables (`RESTMD_VAR_*`).
4. Current environment block from frontmatter.
5. Built-in functions: `{{uuid()}}`, `{{now()}}`, `{{timestamp()}}`, `{{base64(value)}}`, `{{env(NAME)}}`.

`{{var?}}` is optional — empty string if undefined, no error.
`{{var!default}}` provides a fallback.

### 4.6 Directives

- `> capture NAME = JSONPATH` — store a value from the response body for later requests.
- `> capture NAME = response.headers.HEADER` — pull from response headers.
- `> capture NAME = response.status` — pull status code.
- `> assert status == N` — assert response status.
- `> assert JSONPATH OP VALUE` — assert against response body. Operators: `==`, `!=`, `<`, `>`, `<=`, `>=`, `exists`, `matches /regex/`.
- `> set NAME = VALUE` — set a variable for downstream requests without running a request first.

Directives apply to the preceding request. Multiple directives stack.

### 4.7 Body fences

Fence language determines body handling:
- `json` — parsed and re-serialized; Content-Type defaults to `application/json`.
- `xml` — sent as-is; Content-Type defaults to `application/xml`.
- `form` — key:value lines parsed as `multipart/form-data`. `@./path` syntax attaches files.
- `text` — sent as-is; Content-Type required in headers.
- `graphql` — wrapped in `{"query": "...", "variables": {...}}`.

## 5. Surfaces

### 5.1 CLI (`restmd`)

```
restmd run FILE [REQUEST] [--env NAME] [--var k=v] [--format json|pretty|junit]
restmd ls FILE                                 # list requests in a file
restmd check FILE                              # parse + validate, no requests sent
restmd init [--openapi SPEC]                   # scaffold a new file
restmd env list FILE
```

Exit codes: `0` success, `1` assertion failure, `2` parse error, `3` network error, `4` config error.

### 5.2 TUI (`restmd-tui`)

A three-pane interface:
- Left: file tree and request list (parsed H2 headings).
- Center: current request source, editable inline.
- Right: response pane with status, headers, body (syntax-highlighted, foldable for JSON).

Keybindings follow vim conventions where natural. `Enter` runs the focused request. `gd` jumps to a captured variable's defining request.

### 5.3 LSP (`restmd-lsp`)

Capabilities:
- Completion: methods, header names/values, fence languages, variable names, JSONPath expressions, OpenAPI-driven path and body completion.
- Hover: variable origin (which request captured it), header documentation, OpenAPI endpoint summary.
- Diagnostics: unknown variables, forward references, malformed JSON bodies, OpenAPI schema violations.
- Code actions: "Run request" code lens above each H2, "Generate from OpenAPI endpoint", "Add missing required header".
- Document symbols: each H2 is a symbol for outline views.

The LSP does not execute requests. Editor extensions shell out to the `restmd` CLI for execution.

## 6. Milestones

### v0.1 — core + CLI
Parser, executor, variable resolution, captures, assertions, `restmd run` and `restmd check`. Hand-written test fixtures, snapshot tests via `insta`.

### v0.2 — LSP foundation
`restmd-lsp` with tier 1 completion (methods, headers, fences) and basic diagnostics (unknown variables, parse errors). VS Code extension published.

### v0.3 — TUI
`restmd-tui` with file navigation, request execution, response inspection. Read-only first; inline editing in v0.4.

### v0.4 — OpenAPI integration
Spec loading, path completion, body schema completion, request scaffolding from endpoints.

### v0.5 — distribution
`cargo-dist` release pipeline, Homebrew tap, Scoop manifest, signed binaries for macOS and Windows.

### v1.0 — stability
Frozen DSL, semver guarantees on core types, JetBrains plugin, full documentation site.

## 7. Open questions

- **Response persistence**: written back into the source file (inline, below the request), into a sidecar `.restmd-responses/` directory, or not at all by default? Affects snapshot-testing workflows.
- **Multiple variants of one endpoint**: two H2s with disambiguating prose, or a `### Variant: name` sub-heading convention? Affects how the TUI lists requests.
- **Secret handling**: `{{env(SECRET)}}` for env-var injection is straightforward, but do we want first-class secret-store integration (1Password, macOS Keychain) or leave that to shell wrappers?
- **Parallel execution**: should `restmd run FILE` execute requests sequentially (current assumption) or allow a `> parallel` directive to fan out independent requests? Sequential is simpler and matches the document-reading mental model.
- **Request reuse across files**: imports (`> include ../common.md`) or strictly file-scoped? Imports are powerful but complicate the parser and the LSP's symbol resolution.
