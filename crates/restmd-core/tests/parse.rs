//! Behavioural unit tests for the parser.
//!
//! These assert exact structure — spans, template shapes, directive contents,
//! error recovery — so they document the contract precisely and fail loudly on
//! regressions, complementing the coarser snapshot tests.

use restmd_core::*;

/// Parse and assert there were no errors, returning the document.
fn ok(src: &str) -> Document {
    let parsed = parse(src);
    assert!(
        parsed.is_ok(),
        "unexpected parse errors: {:?}",
        parsed.errors
    );
    parsed.document
}

/// Render a template's parts to compact, span-free strings for easy asserting.
fn summary(t: &Template) -> Vec<String> {
    t.parts
        .iter()
        .map(|p| match p {
            TemplatePart::Literal(s) => format!("L:{s}"),
            TemplatePart::Var { name, modifier, .. } => match modifier {
                VarModifier::None => format!("V:{name}"),
                VarModifier::Optional => format!("V:{name}?"),
                VarModifier::Default(d) => format!("V:{name}!{d}"),
            },
            TemplatePart::Call { func, args, .. } => {
                format!("C:{func}({})", args.join(","))
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Structure
// ---------------------------------------------------------------------------

#[test]
fn empty_input_is_an_empty_document() {
    let parsed = parse("");
    assert!(parsed.is_ok());
    assert!(parsed.document.frontmatter.is_none());
    assert!(parsed.document.requests.is_empty());
}

#[test]
fn minimal_request() {
    let doc = ok("## GET /health\n");
    assert_eq!(doc.requests.len(), 1);
    let req = &doc.requests[0];
    assert_eq!(req.method, Method::Get);
    assert_eq!(summary(&req.target), ["L:/health"]);
    assert!(req.headers.is_empty());
    assert!(req.body.is_none());
    assert!(req.directives.is_empty());
}

#[test]
fn all_methods_recognized() {
    let src = "\
## GET /a
## POST /b
## PUT /c
## PATCH /d
## DELETE /e
## HEAD /f
## OPTIONS /g
";
    let doc = ok(src);
    let methods: Vec<_> = doc.requests.iter().map(|r| r.method).collect();
    assert_eq!(
        methods,
        [
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Patch,
            Method::Delete,
            Method::Head,
            Method::Options,
        ]
    );
}

#[test]
fn non_method_h2_is_prose_not_a_request() {
    let doc = ok("## Overview\n\nSome docs.\n\n## GET /real\n");
    assert_eq!(doc.requests.len(), 1);
    assert_eq!(summary(&doc.requests[0].target), ["L:/real"]);
}

#[test]
fn h3_stays_inside_the_preceding_request_region() {
    // The H3 must not start a new request nor end the region prematurely.
    let doc = ok("## GET /x\n\n### A subsection\n\n> assert status == 200\n");
    assert_eq!(doc.requests.len(), 1);
    assert_eq!(doc.requests[0].directives.len(), 1);
}

#[test]
fn h1_ends_a_request_region() {
    let doc = ok("## GET /x\n\n# New chapter\n\n> assert status == 200\n");
    // The assert lives under the H1 (prose), not the request.
    assert_eq!(doc.requests.len(), 1);
    assert!(doc.requests[0].directives.is_empty());
}

// ---------------------------------------------------------------------------
// Spans
// ---------------------------------------------------------------------------

#[test]
fn target_span_points_at_the_path() {
    let src = "## GET /workspaces/{{id}}/projects?limit=50\n";
    let doc = ok(src);
    let target = &doc.requests[0].target;
    assert_eq!(
        target.span.slice(src),
        "/workspaces/{{id}}/projects?limit=50"
    );
    assert_eq!(
        summary(target),
        ["L:/workspaces/", "V:id", "L:/projects?limit=50"]
    );
}

#[test]
fn heading_and_request_spans() {
    let src = "## GET /x\nAccept: application/json\n\n> assert status == 200\n";
    let doc = ok(src);
    let req = &doc.requests[0];
    assert_eq!(req.heading_span.slice(src), "## GET /x");
    // Request span runs from the heading through the last line.
    assert!(req.span.slice(src).starts_with("## GET /x"));
    assert!(req.span.slice(src).ends_with("> assert status == 200"));
}

#[test]
fn header_value_span_and_template() {
    let src = "## GET /x\nAuthorization: Bearer {{token}}\n";
    let doc = ok(src);
    let header = &doc.requests[0].headers[0];
    assert_eq!(header.name, "Authorization");
    assert_eq!(header.value.span.slice(src), "Bearer {{token}}");
    assert_eq!(summary(&header.value), ["L:Bearer ", "V:token"]);
}

#[test]
fn body_span_includes_the_fences_content_excludes_them() {
    let src = "## POST /x\n\n```json\n{\n  \"a\": 1\n}\n```\n";
    let doc = ok(src);
    let body = doc.requests[0].body.as_ref().unwrap();
    assert_eq!(body.lang, BodyLang::Json);
    assert_eq!(body.content, "{\n  \"a\": 1\n}");
    assert!(body.span.slice(src).starts_with("```json"));
    assert!(body.span.slice(src).ends_with("```"));
}

#[test]
fn line_col_is_one_based() {
    let src = "## GET /x\nAccept: text/plain\n";
    let doc = ok(src);
    let header = &doc.requests[0].headers[0];
    assert_eq!(header.span.line_col(src), (2, 1));
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

#[test]
fn template_modifiers() {
    let doc = ok("## GET /s?q={{query?}}&p={{page!1}}&x={{plain}}\n");
    assert_eq!(
        summary(&doc.requests[0].target),
        [
            "L:/s?q=", "V:query?", "L:&p=", "V:page!1", "L:&x=", "V:plain"
        ]
    );
}

#[test]
fn template_calls_with_and_without_args() {
    let doc = ok("## GET /x\nA: {{uuid()}}\nB: {{base64(creds)}}\nC: {{env(API_KEY)}}\n");
    let h = &doc.requests[0].headers;
    assert_eq!(summary(&h[0].value), ["C:uuid()"]);
    assert_eq!(summary(&h[1].value), ["C:base64(creds)"]);
    assert_eq!(summary(&h[2].value), ["C:env(API_KEY)"]);
}

#[test]
fn unterminated_template_is_recovered_as_literal() {
    let parsed = parse("## GET /x/{{oops\n");
    assert_eq!(parsed.errors.len(), 1);
    assert_eq!(parsed.errors[0].kind, ParseErrorKind::UnterminatedTemplate);
    // The request still exists.
    assert_eq!(parsed.document.requests.len(), 1);
}

#[test]
fn empty_interpolation_is_an_error() {
    let parsed = parse("## GET /x/{{}}\n");
    assert_eq!(parsed.errors.len(), 1);
    assert_eq!(parsed.errors[0].kind, ParseErrorKind::EmptyInterpolation);
}

// ---------------------------------------------------------------------------
// Directives
// ---------------------------------------------------------------------------

#[test]
fn capture_sources() {
    let doc = ok("## GET /x\n\n\
> capture a = $.user.id\n\
> capture b = response.headers.ETag\n\
> capture c = response.status\n");
    let d = &doc.requests[0].directives;
    assert!(
        matches!(&d[0], Directive::Capture { name, source: CaptureSource::JsonPath(p), .. } if name == "a" && p == "$.user.id")
    );
    assert!(
        matches!(&d[1], Directive::Capture { source: CaptureSource::Header(h), .. } if h == "ETag")
    );
    assert!(matches!(
        &d[2],
        Directive::Capture {
            source: CaptureSource::Status,
            ..
        }
    ));
}

#[test]
fn assert_status_and_body_operators() {
    let doc = ok("## GET /x\n\n\
> assert status == 200\n\
> assert status >= 500\n\
> assert $.items exists\n\
> assert $.name == \"Q4 Launch\"\n\
> assert $.count >= 3\n\
> assert $.ok == true\n\
> assert $.email matches /.+@.+/\n");
    let d = &doc.requests[0].directives;

    assert!(matches!(
        &d[0],
        Directive::Assert {
            assertion: Assertion::Status {
                op: CompareOp::Eq,
                code: 200
            },
            ..
        }
    ));
    assert!(matches!(
        &d[1],
        Directive::Assert {
            assertion: Assertion::Status {
                op: CompareOp::Ge,
                code: 500
            },
            ..
        }
    ));

    assert!(
        matches!(&d[2], Directive::Assert { assertion: Assertion::Body { path, op: AssertOp::Exists }, .. } if path == "$.items")
    );

    match &d[3] {
        Directive::Assert {
            assertion:
                Assertion::Body {
                    path,
                    op: AssertOp::Compare(CompareOp::Eq, Value::String(s)),
                },
            ..
        } => {
            assert_eq!(path, "$.name");
            assert_eq!(s, "Q4 Launch");
        }
        other => panic!("unexpected: {other:?}"),
    }

    assert!(
        matches!(&d[4], Directive::Assert { assertion: Assertion::Body { op: AssertOp::Compare(CompareOp::Ge, Value::Number(n)), .. }, .. } if *n == 3.0)
    );
    assert!(matches!(
        &d[5],
        Directive::Assert {
            assertion: Assertion::Body {
                op: AssertOp::Compare(CompareOp::Eq, Value::Bool(true)),
                ..
            },
            ..
        }
    ));
    assert!(
        matches!(&d[6], Directive::Assert { assertion: Assertion::Body { op: AssertOp::Matches(re), .. }, .. } if re == ".+@.+")
    );
}

#[test]
fn set_directive_holds_a_template() {
    let doc = ok("## GET /x\n\n> set greeting = hello {{name}}\n");
    match &doc.requests[0].directives[0] {
        Directive::Set { name, value, .. } => {
            assert_eq!(name, "greeting");
            assert_eq!(summary(value), ["L:hello ", "V:name"]);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Bodies and prose
// ---------------------------------------------------------------------------

#[test]
fn only_recognized_fence_languages_become_bodies() {
    // A ```sh block is prose; the following ```json block is the body.
    let src = "## POST /x\n\n```sh\necho hi\n```\n\n```json\n{}\n```\n";
    let doc = ok(src);
    let body = doc.requests[0].body.as_ref().unwrap();
    assert_eq!(body.lang, BodyLang::Json);
    assert_eq!(body.content, "{}");
}

#[test]
fn first_recognized_fence_wins() {
    let src = "## POST /x\n\n```json\n{\"first\":true}\n```\n\n```json\n{\"second\":true}\n```\n";
    let doc = ok(src);
    assert_eq!(
        doc.requests[0].body.as_ref().unwrap().content,
        "{\"first\":true}"
    );
}

#[test]
fn directive_inside_a_body_fence_is_not_parsed_as_a_directive() {
    let src = "## POST /x\n\n```text\n> not a directive\n```\n\n> assert status == 200\n";
    let doc = ok(src);
    let req = &doc.requests[0];
    assert_eq!(req.body.as_ref().unwrap().content, "> not a directive");
    assert_eq!(req.directives.len(), 1);
}

#[test]
fn empty_body_fence() {
    let doc = ok("## POST /x\n\n```json\n```\n");
    assert_eq!(doc.requests[0].body.as_ref().unwrap().content, "");
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

#[test]
fn frontmatter_is_parsed() {
    let src = "\
---
base: https://api.{{env}}.example.com
openapi: ./openapi.yaml
timeout: 30s
retries: 2
environments:
  dev:
    env: dev-api
    workspace_id: ws_dev
  prod:
    env: api
    enabled: true
    weight: 3
defaults:
  Accept: application/json
---

## GET /x
";
    let doc = ok(src);
    let fm = doc.frontmatter.unwrap();
    assert_eq!(fm.base.as_deref(), Some("https://api.{{env}}.example.com"));
    assert_eq!(fm.openapi.as_deref(), Some("./openapi.yaml"));
    assert_eq!(fm.timeout.as_deref(), Some("30s"));
    assert_eq!(fm.retries, Some(2));
    assert_eq!(
        fm.defaults.get("Accept").map(String::as_str),
        Some("application/json")
    );
    assert_eq!(
        fm.environments["dev"]["workspace_id"],
        ConfigValue::String("ws_dev".into())
    );
    assert_eq!(fm.environments["prod"]["enabled"], ConfigValue::Bool(true));
    assert_eq!(fm.environments["prod"]["weight"], ConfigValue::Int(3));
    // And the request after the frontmatter parsed too.
    assert_eq!(doc.requests.len(), 1);
}

#[test]
fn empty_frontmatter_is_default_not_an_error() {
    let parsed = parse("---\n---\n\n## GET /x\n");
    assert!(parsed.is_ok());
    assert_eq!(parsed.document.frontmatter, Some(Frontmatter::default()));
    assert_eq!(parsed.document.requests.len(), 1);
}

#[test]
fn no_frontmatter_is_none() {
    let doc = ok("## GET /x\n");
    assert!(doc.frontmatter.is_none());
}

#[test]
fn invalid_frontmatter_is_reported_but_requests_still_parse() {
    let parsed = parse("---\nbase: [unclosed\n---\n\n## GET /x\n");
    assert_eq!(parsed.errors.len(), 1);
    assert!(matches!(
        parsed.errors[0].kind,
        ParseErrorKind::Frontmatter(_)
    ));
    assert!(parsed.document.frontmatter.is_none());
    assert_eq!(parsed.document.requests.len(), 1);
}

#[test]
fn unknown_frontmatter_key_is_rejected() {
    let parsed = parse("---\nbogus: 1\n---\n\n## GET /x\n");
    assert!(matches!(
        parsed.errors[0].kind,
        ParseErrorKind::Frontmatter(_)
    ));
}

#[test]
fn unterminated_frontmatter_is_reported() {
    let parsed = parse("---\nbase: https://x\n\n## GET /x\n");
    assert!(
        parsed
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnterminatedFrontmatter)
    );
    // Requests after the (unterminated) block are still recovered.
    assert_eq!(parsed.document.requests.len(), 1);
}

// ---------------------------------------------------------------------------
// Error recovery
// ---------------------------------------------------------------------------

#[test]
fn missing_path_is_reported_but_request_kept() {
    let parsed = parse("## GET\nAccept: application/json\n");
    assert_eq!(parsed.document.requests.len(), 1);
    assert!(
        parsed
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::MissingPath)
    );
}

#[test]
fn unterminated_fence_is_reported() {
    let parsed = parse("## POST /x\n\n```json\n{ \"open\": true\n");
    assert!(
        parsed
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnterminatedFence)
    );
    assert_eq!(parsed.document.requests.len(), 1);
    assert!(parsed.document.requests[0].body.is_none());
}

#[test]
fn unknown_directive_is_reported() {
    let parsed = parse("## GET /x\n\n> teardown stuff\n");
    assert!(matches!(
        parsed.errors[0].kind,
        ParseErrorKind::UnknownDirective(ref d) if d == "teardown"
    ));
    // The bad directive is dropped, not kept as a malformed node.
    assert!(parsed.document.requests[0].directives.is_empty());
}

#[test]
fn malformed_directives_each_report_and_skip() {
    let src = "## GET /x\n\n\
> capture\n\
> capture = $.id\n\
> capture id = nowhere\n\
> assert\n\
> assert status == abc\n\
> set\n\
> set = 5\n";
    let parsed = parse(src);
    // Seven malformed directives -> seven errors, zero surviving directives.
    let malformed = parsed
        .errors
        .iter()
        .filter(|e| matches!(e.kind, ParseErrorKind::MalformedDirective { .. }))
        .count();
    assert_eq!(malformed, 7, "errors: {:?}", parsed.errors);
    assert!(parsed.document.requests[0].directives.is_empty());
}

#[test]
fn good_and_bad_directives_coexist() {
    let parsed = parse("## GET /x\n\n> assert status == 200\n> assert\n> capture id = $.id\n");
    assert_eq!(parsed.document.requests[0].directives.len(), 2);
    assert_eq!(parsed.errors.len(), 1);
}

// ---------------------------------------------------------------------------
// Line endings
// ---------------------------------------------------------------------------

#[test]
fn crlf_line_endings() {
    let src = "## POST /x\r\nContent-Type: application/json\r\n\r\n```json\r\n{}\r\n```\r\n";
    let doc = ok(src);
    let req = &doc.requests[0];
    assert_eq!(req.headers[0].name, "Content-Type");
    assert_eq!(summary(&req.headers[0].value), ["L:application/json"]);
    assert_eq!(req.body.as_ref().unwrap().content, "{}");
}

// ---------------------------------------------------------------------------
// Small type-level checks
// ---------------------------------------------------------------------------

#[test]
fn method_roundtrips() {
    use std::str::FromStr;
    for m in [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
        Method::Head,
        Method::Options,
    ] {
        assert_eq!(Method::from_str(m.as_str()), Ok(m));
    }
}
