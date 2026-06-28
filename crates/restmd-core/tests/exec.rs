//! End-to-end executor tests against a **real** local HTTP server.
//!
//! Each test boots a `tiny_http` server (see `support/server.rs`), points the
//! document's `base` at it, and runs the genuine `ReqwestTransport`. We assert
//! both the run report and — via the server's recorded requests — exactly what
//! went over the wire.

mod support;

use std::collections::BTreeMap;

use restmd_core::{Document, ExecError, ReqwestTransport, RunOptions, RunReport, Runner, parse};
use support::server::{TestServer, closed_port};

/// Parse a document with the given frontmatter (no braces) and body.
fn parse_doc(frontmatter: &str, body: &str) -> Document {
    let src = format!("---\n{frontmatter}\n---\n\n{body}");
    let parsed = parse(&src);
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    parsed.document
}

fn run(doc: &Document) -> RunReport {
    Runner::new(ReqwestTransport::new()).run(doc, &RunOptions::default())
}

// ---------------------------------------------------------------------------
// Status assertions & exit codes
// ---------------------------------------------------------------------------

#[test]
fn get_with_passing_status_assertion() {
    let s = TestServer::start();
    let doc = parse_doc(
        &format!("base: {}", s.base),
        "## GET /data\n\n> assert status == 200\n",
    );
    let report = run(&doc);
    assert_eq!(report.exit_code(), 0);
    let outcome = &report.outcomes[0];
    assert_eq!(outcome.status, Some(200));
    assert!(outcome.assertions[0].passed);
}

#[test]
fn failing_status_assertion_exits_1_without_aborting() {
    let s = TestServer::start();
    let doc = parse_doc(
        &format!("base: {}", s.base),
        "## GET /status/500\n\n> assert status == 200\n",
    );
    let report = run(&doc);
    assert_eq!(report.exit_code(), 1);
    let outcome = &report.outcomes[0];
    assert_eq!(outcome.status, Some(500));
    assert!(!outcome.assertions[0].passed);
    assert!(outcome.error.is_none()); // assertion failure is not a fatal error
}

// ---------------------------------------------------------------------------
// URL building & headers (verified server-side)
// ---------------------------------------------------------------------------

#[test]
fn relative_target_joins_base_and_preserves_query() {
    let s = TestServer::start();
    let doc = parse_doc(&format!("base: {}", s.base), "## GET /data?x=1&y=2\n");
    let report = run(&doc);
    assert_eq!(report.exit_code(), 0);
    let reqs = s.requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[0].path, "/data?x=1&y=2");
}

#[test]
fn headers_merge_defaults_under_request_and_default_content_type() {
    let s = TestServer::start();
    let fm = format!(
        "base: {}\ndefaults:\n  Accept: application/json\n  X-Default: yes",
        s.base
    );
    let body = "## POST /data\nX-Custom: hi\n\n```json\n{\"a\":1}\n```\n";
    let report = run(&parse_doc(&fm, body));
    assert_eq!(report.exit_code(), 0);

    let reqs = s.requests();
    let r = &reqs[0];
    assert_eq!(r.method, "POST");
    assert_eq!(r.header("Accept"), Some("application/json")); // from defaults
    assert_eq!(r.header("X-Default"), Some("yes")); // from defaults
    assert_eq!(r.header("X-Custom"), Some("hi")); // from request
    assert_eq!(r.header("Content-Type"), Some("application/json")); // defaulted for json
    assert_eq!(r.body, "{\"a\":1}");
}

#[test]
fn cli_vars_resolve_into_the_url() {
    let s = TestServer::start();
    let doc = parse_doc(&format!("base: {}", s.base), "## GET /thing/{{id}}\n");
    let opts = RunOptions {
        env: None,
        vars: BTreeMap::from([("id".to_string(), "42".to_string())]),
        include_os_env: false,
    };
    let report = Runner::new(ReqwestTransport::new()).run(&doc, &opts);
    assert_eq!(report.exit_code(), 0);
    assert_eq!(s.requests()[0].path, "/thing/42");
}

// ---------------------------------------------------------------------------
// Captures
// ---------------------------------------------------------------------------

#[test]
fn jsonpath_capture_threads_into_a_later_request() {
    let s = TestServer::start();
    let body = "## POST /auth/login\n\n```json\n{\"e\":\"x\"}\n```\n\n\
> capture token = $.access_token\n\
> capture uid = $.user.id\n\n\
## GET /projects/{{uid}}\n\n> assert status == 200\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);

    let caps = &report.outcomes[0].captures;
    assert!(
        caps.iter()
            .any(|c| c.name == "token" && c.value.as_deref() == Some("tok123"))
    );
    assert!(
        caps.iter()
            .any(|c| c.name == "uid" && c.value.as_deref() == Some("u1"))
    );

    let reqs = s.requests();
    assert_eq!(reqs[1].path, "/projects/u1");
}

#[test]
fn captures_header_and_status() {
    let s = TestServer::start();
    let body = "## GET /data\n\n\
> capture etag = response.headers.ETag\n\
> capture code = response.status\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    let caps = &report.outcomes[0].captures;
    assert_eq!(
        caps.iter()
            .find(|c| c.name == "etag")
            .unwrap()
            .value
            .as_deref(),
        Some("etag-xyz")
    );
    assert_eq!(
        caps.iter()
            .find(|c| c.name == "code")
            .unwrap()
            .value
            .as_deref(),
        Some("200")
    );
}

// ---------------------------------------------------------------------------
// Body assertions (every operator)
// ---------------------------------------------------------------------------

#[test]
fn body_assertion_operators_against_real_json() {
    let s = TestServer::start();
    let body = "## GET /data\n\n\
> assert $.name == \"Q4 Launch\"\n\
> assert $.count >= 3\n\
> assert $.count < 10\n\
> assert $.active == true\n\
> assert $.items exists\n\
> assert $.email matches /.+@.+/\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0, "{:?}", report.outcomes[0].assertions);
    assert!(report.outcomes[0].assertions.iter().all(|a| a.passed));
}

#[test]
fn set_directive_threads_a_downstream_variable() {
    let s = TestServer::start();
    let body =
        "## GET /data\n\n> set region = eu\n\n## GET /r/{{region}}\n\n> assert status == 200\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);
    assert_eq!(s.requests()[1].path, "/r/eu");
}

// ---------------------------------------------------------------------------
// Cookie sessions
// ---------------------------------------------------------------------------

#[test]
fn response_cookies_are_sent_to_later_matching_requests() {
    let s = TestServer::start();
    let body = "## GET /cookies/set\n\n\
## GET /cookies/check\n\n\
## GET /outside\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);

    let set_cookie_count = report.outcomes[0]
        .response
        .as_ref()
        .unwrap()
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .count();
    assert_eq!(set_cookie_count, 2);

    let reqs = s.requests();
    let matching = reqs[1].header("Cookie").expect("matching cookies");
    assert!(matching.contains("session=abc"), "{matching}");
    assert!(matching.contains("scoped=yes"), "{matching}");

    let outside = reqs[2].header("Cookie").expect("root cookie");
    assert!(outside.contains("session=abc"), "{outside}");
    assert!(!outside.contains("scoped=yes"), "{outside}");
}

#[test]
fn cookie_deletion_is_respected() {
    let s = TestServer::start();
    let body = "## GET /cookies/set\n\n\
## GET /cookies/delete\n\n\
## GET /cookies/check\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);

    let reqs = s.requests();
    let final_cookie = reqs[2].header("Cookie").expect("scoped cookie remains");
    assert!(final_cookie.contains("scoped=yes"), "{final_cookie}");
    assert!(!final_cookie.contains("session="), "{final_cookie}");
}

#[test]
fn explicit_cookie_header_overrides_the_session_store() {
    let s = TestServer::start();
    let body = "## GET /cookies/set\n\n\
## GET /cookies/check\nCookie: manual=1\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);

    assert_eq!(s.requests()[1].header("Cookie"), Some("manual=1"));
}

#[test]
fn run_through_threads_cookies_from_the_prefix() {
    let s = TestServer::start();
    let body = "## GET /cookies/set\n\n\
## GET /cookies/check\n\n\
## GET /outside\n";
    let doc = parse_doc(&format!("base: {}", s.base), body);

    let report = Runner::new(ReqwestTransport::new()).run_through(&doc, 1, &RunOptions::default());
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.outcomes.len(), 2);

    let reqs = s.requests();
    assert_eq!(reqs.len(), 2);
    let cookie = reqs[1].header("Cookie").expect("cookie from prefix");
    assert!(cookie.contains("session=abc"), "{cookie}");
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn invalid_json_body_is_a_config_error_before_sending() {
    let s = TestServer::start();
    let body = "## POST /data\n\n```json\n{ not valid\n```\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 4);
    assert!(matches!(
        report.outcomes[0].error,
        Some(ExecError::Config(_))
    ));
    assert!(s.requests().is_empty(), "nothing should have been sent");
}

#[test]
fn text_body_without_content_type_is_a_config_error() {
    let s = TestServer::start();
    let body = "## POST /text\n\n```text\nhello\n```\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 4);
    assert!(matches!(
        report.outcomes[0].error,
        Some(ExecError::Config(_))
    ));
}

#[test]
fn undefined_variable_is_a_config_error() {
    let s = TestServer::start();
    let doc = parse_doc(&format!("base: {}", s.base), "## GET /u/{{missing}}\n");
    let report = run(&doc);
    assert_eq!(report.exit_code(), 4);
    assert!(matches!(
        report.outcomes[0].error,
        Some(ExecError::Resolve(_))
    ));
}

#[test]
fn connection_refused_is_a_network_error() {
    let port = closed_port();
    let doc = parse_doc(
        &format!("base: http://127.0.0.1:{port}"),
        "## GET /x\n\n> assert status == 200\n",
    );
    let report = run(&doc);
    assert_eq!(report.exit_code(), 3);
    assert!(matches!(
        report.outcomes[0].error,
        Some(ExecError::Network(_))
    ));
}

#[test]
fn unknown_environment_is_a_preflight_config_error() {
    let s = TestServer::start();
    let doc = parse_doc(&format!("base: {}", s.base), "## GET /data\n");
    let opts = RunOptions {
        env: Some("nope".to_string()),
        vars: BTreeMap::new(),
        include_os_env: false,
    };
    let report = Runner::new(ReqwestTransport::new()).run(&doc, &opts);
    assert_eq!(report.exit_code(), 4);
    assert!(matches!(report.error, Some(ExecError::Config(_))));
    assert!(report.outcomes.is_empty());
}

// ---------------------------------------------------------------------------
// Full flow
// ---------------------------------------------------------------------------

#[test]
fn login_use_delete_flow() {
    let s = TestServer::start();
    let body = "## POST /auth/login\n\n```json\n{\"e\":\"x\"}\n```\n\n\
> capture token = $.access_token\n\
> assert status == 200\n\n\
## GET /projects/{{token}}\n\n> assert status == 200\n\n\
## DELETE /projects/{{token}}\n\n> assert status == 200\n";
    let report = run(&parse_doc(&format!("base: {}", s.base), body));
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.outcomes.len(), 3);
    assert!(report.outcomes.iter().all(|o| o.passed()));

    let reqs = s.requests();
    let methods: Vec<_> = reqs.iter().map(|r| r.method.as_str()).collect();
    assert_eq!(methods, ["POST", "GET", "DELETE"]);
    assert_eq!(reqs[2].path, "/projects/tok123");
}

// ---------------------------------------------------------------------------
// Response snapshot & prefix run
// ---------------------------------------------------------------------------

#[test]
fn outcome_carries_the_response_snapshot() {
    let s = TestServer::start();
    let doc = parse_doc(&format!("base: {}", s.base), "## GET /data\n");
    let report = run(&doc);
    let response = report.outcomes[0]
        .response
        .as_ref()
        .expect("response present");
    assert_eq!(response.status, 200);
    assert!(response.body_text().contains("Q4 Launch"));
    assert!(
        response
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("etag"))
    );
}

#[test]
fn run_through_runs_only_the_prefix_but_threads_captures() {
    let s = TestServer::start();
    // Three requests; running through index 1 should execute the first two only,
    // and the second must still see the capture from the first.
    let body = "## POST /auth/login\n\n```json\n{\"e\":\"x\"}\n```\n\n\
> capture token = $.access_token\n\n\
## GET /projects/{{token}}\n\n\
## DELETE /projects/{{token}}\n";
    let doc = parse_doc(&format!("base: {}", s.base), body);

    let report = Runner::new(ReqwestTransport::new()).run_through(&doc, 1, &RunOptions::default());
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.outcomes.len(), 2); // the DELETE was not run

    let reqs = s.requests();
    let methods: Vec<_> = reqs.iter().map(|r| r.method.as_str()).collect();
    assert_eq!(methods, ["POST", "GET"]);
    assert_eq!(reqs[1].path, "/projects/tok123");
}
