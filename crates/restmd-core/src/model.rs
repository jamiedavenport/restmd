//! The parsed document model.
//!
//! These types are the single source of truth that every surface (executor,
//! CLI, TUI, LSP) consumes. They follow two rules:
//!
//! * **Make invalid states unrepresentable.** Methods and body languages are
//!   closed enums, not strings; directives are typed, not free text.
//! * **Carry spans.** Every node remembers where it came from so diagnostics
//!   can point at it.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Deserialize;

use crate::span::Span;
use crate::template::Template;

/// A whole parsed restmd file.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// File-level config, if a frontmatter block was present and valid.
    pub frontmatter: Option<Frontmatter>,
    /// Requests, in source order.
    pub requests: Vec<Request>,
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

/// File-level configuration from the YAML frontmatter block.
///
/// Unknown keys are rejected so typos surface as errors rather than being
/// silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    /// Base URL prepended to relative request paths. May itself be templated.
    #[serde(default)]
    pub base: Option<String>,
    /// Path or URL of an OpenAPI spec for completion/validation.
    #[serde(default)]
    pub openapi: Option<String>,
    /// Named variable sets, selected with `--env <name>`.
    #[serde(default)]
    pub environments: BTreeMap<String, BTreeMap<String, ConfigValue>>,
    /// Headers merged into every request unless overridden.
    #[serde(default)]
    pub defaults: BTreeMap<String, String>,
    /// Default per-request timeout (e.g. `30s`), kept raw until execution.
    #[serde(default)]
    pub timeout: Option<String>,
    /// Default retry count for idempotent methods.
    #[serde(default)]
    pub retries: Option<u32>,
}

/// A scalar config value in an environment block. Modelled explicitly rather
/// than leaking the YAML library's value type into the public API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// A single HTTP request: an `## METHOD /path` heading plus its headers, body,
/// and directives.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    /// The request target (path + optional query + fragment), templated. Kept
    /// whole; splitting into path/query/fragment is deferred because a `{{var}}`
    /// can straddle a `?` and naive splitting would be wrong.
    pub target: Template,
    pub headers: Vec<Header>,
    pub body: Option<Body>,
    pub directives: Vec<Directive>,
    /// Span of the entire request section, heading through last line.
    pub span: Span,
    /// Span of just the `## ...` heading line.
    pub heading_span: Span,
}

/// HTTP methods restmd understands. A closed set: an H2 whose first token is
/// not one of these is treated as prose, not a malformed request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            _ => return Err(()),
        })
    }
}

/// A request header. The name is kept verbatim; HTTP header semantics
/// (case-insensitivity) are applied at execution time.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub name: String,
    pub value: Template,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Bodies
// ---------------------------------------------------------------------------

/// A request body from a fenced code block. The content is kept raw; how it is
/// serialized (JSON re-emission, form encoding, GraphQL wrapping) is decided at
/// execution time from [`Body::lang`].
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub lang: BodyLang,
    pub content: String,
    /// Span of the whole fence, including the ``` delimiters.
    pub span: Span,
}

/// Fence languages that denote a request body. A code fence with any other info
/// string is treated as ordinary prose, so illustrative ```` ```rust ```` blocks
/// in documentation are left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLang {
    Json,
    Xml,
    Form,
    Text,
    Graphql,
}

impl FromStr for BodyLang {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "json" => BodyLang::Json,
            "xml" => BodyLang::Xml,
            "form" => BodyLang::Form,
            "text" => BodyLang::Text,
            "graphql" => BodyLang::Graphql,
            _ => return Err(()),
        })
    }
}

// ---------------------------------------------------------------------------
// Directives
// ---------------------------------------------------------------------------

/// A `>`-prefixed directive that applies to the preceding request.
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// `> capture NAME = <source>` — store a value for later requests.
    Capture {
        name: String,
        source: CaptureSource,
        span: Span,
    },
    /// `> assert <expr>` — fail the run if the condition does not hold.
    Assert { assertion: Assertion, span: Span },
    /// `> set NAME = VALUE` — bind a variable without running a request.
    Set {
        name: String,
        value: Template,
        span: Span,
    },
}

/// Where a `capture` pulls its value from.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureSource {
    /// A JSONPath into the response body, e.g. `$.access_token`.
    JsonPath(String),
    /// A response header, e.g. `response.headers.Location`.
    Header(String),
    /// The response status code (`response.status`).
    Status,
}

/// A parsed assertion condition.
#[derive(Debug, Clone, PartialEq)]
pub enum Assertion {
    /// `assert status <op> <code>`.
    Status { op: CompareOp, code: u16 },
    /// `assert <jsonpath> <op>`.
    Body { path: String, op: AssertOp },
}

/// Ordering/equality operators shared by status and body assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl CompareOp {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "==" => CompareOp::Eq,
            "!=" => CompareOp::Ne,
            "<" => CompareOp::Lt,
            ">" => CompareOp::Gt,
            "<=" => CompareOp::Le,
            ">=" => CompareOp::Ge,
            _ => return None,
        })
    }
}

/// The operation applied in a body assertion.
#[derive(Debug, Clone, PartialEq)]
pub enum AssertOp {
    /// Comparison against a literal value, e.g. `$.count >= 3`.
    Compare(CompareOp, Value),
    /// `exists` — the path resolves to something.
    Exists,
    /// `matches /regex/` — the regex source is kept raw, not compiled here.
    Matches(String),
}

/// A literal value on the right-hand side of a body assertion.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl Value {
    /// Parse an assertion right-hand side: number, `true`/`false`/`null`, a
    /// quoted string, or an unquoted bareword (kept as a string).
    pub(crate) fn parse(s: &str) -> Self {
        let s = s.trim();
        if let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            return Value::String(inner.to_string());
        }
        match s {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            "null" => return Value::Null,
            _ => {}
        }
        if let Ok(n) = s.parse::<f64>() {
            return Value::Number(n);
        }
        Value::String(s.to_string())
    }
}
