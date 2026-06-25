# restmd request files

This directory contains restmd request files. They are ordinary Markdown `.md`
files, so keep prose readable and put executable requests in H2 headings.

## Structure

- Request files should live in this `.restmd/` directory.
- A request starts with `## METHOD /path`, for example `## GET /health`.
- Supported methods are `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, and
  `OPTIONS`.
- Header lines go directly under the request heading as `Name: value`.
- Request bodies use fenced code blocks labelled `json`, `xml`, `form`, `text`,
  or `graphql`.
- Directives are Markdown blockquotes attached to the request above them:
  `> capture token = $.access_token`, `> assert status == 200`, or
  `> set name = value`.
- Variables use `{{name}}`. Provide values with captures, `--var name=value`,
  `RESTMD_VAR_<NAME>` environment variables, or frontmatter environments.

## Commands

- Validate request files without sending HTTP: `restmd check .`
- Check formatting without writing files: `restmd format . --check`
- Run requests only when execution is intended: `restmd run . --format json`
- Start the interactive TUI from the parent project: `restmd .restmd`
- Editor integrations launch the language server with `restmd lsp`.

## Agent guidance

- Do not invent secrets, tokens, hosts, or credentials. Ask for them or use
  documented variables.
- Prefer adding examples that are safe to run against local, test, or documented
  endpoints.
- Keep restmd files valid Markdown and avoid changing unrelated prose while
  editing requests.
