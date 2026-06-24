//! Behavioural tests for variable resolution.
//!
//! Templates are obtained by parsing tiny request snippets and pulling out the
//! parsed `target` — that keeps these tests honest end-to-end (parser →
//! resolver) without reaching into crate internals. Non-deterministic builtins
//! are pinned with a fixed clock / id generator.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use restmd_core::*;

/// Parse `## GET {path}` and return the request target template.
fn tmpl(path: &str) -> Template {
    let doc = parse(&format!("## GET {path}\n")).document;
    doc.requests.into_iter().next().unwrap().target
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A clock fixed at 2023-11-14T22:13:20Z (Unix 1_700_000_000).
struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }
}

struct FixedId;
impl IdGen for FixedId {
    fn uuid(&self) -> String {
        "11111111-1111-4111-8111-111111111111".to_string()
    }
}

// ---------------------------------------------------------------------------
// Lookup tiers and precedence
// ---------------------------------------------------------------------------

#[test]
fn resolves_from_each_tier() {
    // Tier 2: --var
    let ctx = Context::builder().vars(map(&[("id", "42")])).build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/u/{{id}}")).unwrap(),
        "/u/42"
    );

    // Tier 3: RESTMD_VAR_*
    let ctx = Context::builder()
        .os_env(map(&[("RESTMD_VAR_id", "99")]))
        .build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/u/{{id}}")).unwrap(),
        "/u/99"
    );

    // Tier 4: frontmatter environment block (typed ConfigValue)
    let env = BTreeMap::from([
        ("id".to_string(), ConfigValue::Int(7)),
        ("on".to_string(), ConfigValue::Bool(true)),
    ]);
    let ctx = Context::builder().environment(&env).build();
    let r = Resolver::new(&ctx);
    assert_eq!(r.resolve(&tmpl("/u/{{id}}")).unwrap(), "/u/7");
    assert_eq!(r.resolve(&tmpl("/f/{{on}}")).unwrap(), "/f/true");
}

#[test]
fn precedence_is_capture_then_var_then_env_then_block() {
    let env = BTreeMap::from([("k".to_string(), ConfigValue::String("block".into()))]);
    let full = Context::builder()
        .captures(map(&[("k", "cap")]))
        .vars(map(&[("k", "var")]))
        .os_env(map(&[("RESTMD_VAR_k", "osenv")]))
        .environment(&env)
        .build();
    assert_eq!(
        Resolver::new(&full).resolve(&tmpl("/{{k}}")).unwrap(),
        "/cap"
    );

    let no_cap = Context::builder()
        .vars(map(&[("k", "var")]))
        .os_env(map(&[("RESTMD_VAR_k", "osenv")]))
        .environment(&env)
        .build();
    assert_eq!(
        Resolver::new(&no_cap).resolve(&tmpl("/{{k}}")).unwrap(),
        "/var"
    );

    let no_var = Context::builder()
        .os_env(map(&[("RESTMD_VAR_k", "osenv")]))
        .environment(&env)
        .build();
    assert_eq!(
        Resolver::new(&no_var).resolve(&tmpl("/{{k}}")).unwrap(),
        "/osenv"
    );

    let block_only = Context::builder().environment(&env).build();
    assert_eq!(
        Resolver::new(&block_only).resolve(&tmpl("/{{k}}")).unwrap(),
        "/block"
    );
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

#[test]
fn required_variable_undefined_errors() {
    let ctx = Context::builder().build();
    let err = Resolver::new(&ctx)
        .resolve(&tmpl("/{{missing}}"))
        .unwrap_err();
    assert_eq!(
        err.kind,
        ResolveErrorKind::UndefinedVariable("missing".into())
    );
}

#[test]
fn optional_variable_is_empty_when_undefined() {
    let ctx = Context::builder().build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/a/{{x?}}/b")).unwrap(),
        "/a//b"
    );
}

#[test]
fn default_modifier_falls_back_then_prefers_value() {
    let ctx = Context::builder().build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/p/{{page!1}}")).unwrap(),
        "/p/1"
    );
    let ctx = Context::builder().vars(map(&[("page", "5")])).build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/p/{{page!1}}")).unwrap(),
        "/p/5"
    );
}

// ---------------------------------------------------------------------------
// Builtins
// ---------------------------------------------------------------------------

#[test]
fn uuid_now_timestamp_are_deterministic_under_injection() {
    let ctx = Context::builder().clock(FixedClock).idgen(FixedId).build();
    let r = Resolver::new(&ctx);
    assert_eq!(
        r.resolve(&tmpl("/{{uuid()}}")).unwrap(),
        "/11111111-1111-4111-8111-111111111111"
    );
    assert_eq!(r.resolve(&tmpl("/{{timestamp()}}")).unwrap(), "/1700000000");
    let now = r.resolve(&tmpl("/{{now()}}")).unwrap();
    assert_eq!(now, "/2023-11-14T22:13:20Z");
}

#[test]
fn base64_encodes_a_resolved_variable() {
    let ctx = Context::builder()
        .vars(map(&[("creds", "user:pass")]))
        .build();
    assert_eq!(
        Resolver::new(&ctx)
            .resolve(&tmpl("/{{base64(creds)}}"))
            .unwrap(),
        "/dXNlcjpwYXNz"
    );
}

#[test]
fn base64_of_undefined_variable_errors() {
    let ctx = Context::builder().build();
    let err = Resolver::new(&ctx)
        .resolve(&tmpl("/{{base64(nope)}}"))
        .unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::UndefinedVariable("nope".into()));
}

#[test]
fn env_reads_a_literal_os_var() {
    let ctx = Context::builder()
        .os_env(map(&[("HOME", "/home/jamie")]))
        .build();
    assert_eq!(
        Resolver::new(&ctx)
            .resolve(&tmpl("{{env(HOME)}}/x"))
            .unwrap(),
        "/home/jamie/x"
    );
}

#[test]
fn env_missing_var_errors() {
    let ctx = Context::builder().build();
    let err = Resolver::new(&ctx)
        .resolve(&tmpl("/{{env(NOPE)}}"))
        .unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::EnvVarNotSet("NOPE".into()));
}

#[test]
fn unknown_function_errors() {
    let ctx = Context::builder().build();
    let err = Resolver::new(&ctx)
        .resolve(&tmpl("/{{bogus()}}"))
        .unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::UnknownFunction("bogus".into()));
}

#[test]
fn wrong_argument_count_errors() {
    let ctx = Context::builder().build();
    // uuid takes no args
    let err = Resolver::new(&ctx)
        .resolve(&tmpl("/{{uuid(x)}}"))
        .unwrap_err();
    assert_eq!(
        err.kind,
        ResolveErrorKind::WrongArgCount {
            func: "uuid",
            expected: 0,
            got: 1
        }
    );
}

// ---------------------------------------------------------------------------
// Whole-template behaviour
// ---------------------------------------------------------------------------

#[test]
fn multipart_template_resolves_end_to_end() {
    let env = BTreeMap::from([("ws".to_string(), ConfigValue::String("acme".into()))]);
    let ctx = Context::builder()
        .vars(map(&[("limit", "50")]))
        .environment(&env)
        .build();
    let t = tmpl("/w/{{ws}}/projects?limit={{limit}}&q={{q?}}");
    assert_eq!(
        Resolver::new(&ctx).resolve(&t).unwrap(),
        "/w/acme/projects?limit=50&q="
    );
}

#[test]
fn resolution_is_single_pass() {
    // A value that itself looks like a template is emitted literally.
    let ctx = Context::builder().vars(map(&[("a", "{{b}}")])).build();
    assert_eq!(
        Resolver::new(&ctx).resolve(&tmpl("/{{a}}")).unwrap(),
        "/{{b}}"
    );
}

#[test]
fn resolve_str_parses_then_resolves() {
    let ctx = Context::builder().vars(map(&[("env", "dev")])).build();
    let out = Resolver::new(&ctx)
        .resolve_str("https://api.{{env}}.example.com")
        .unwrap();
    assert_eq!(out, "https://api.dev.example.com");
}

#[test]
fn resolve_str_reports_malformed_template() {
    let ctx = Context::builder().build();
    let err = Resolver::new(&ctx)
        .resolve_str("https://x/{{oops")
        .unwrap_err();
    assert!(matches!(err.kind, ResolveErrorKind::MalformedTemplate(_)));
}
