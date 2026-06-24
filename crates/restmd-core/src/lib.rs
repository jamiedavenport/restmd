//! `restmd-core` — the parser and document model for restmd, a markdown-native
//! REST client.
//!
//! This crate is the single source of truth that every restmd surface (CLI,
//! TUI, LSP) is built on. At this stage it does one thing: turn a `.md` source
//! string into a typed [`Document`] via [`parse`], reporting any problems as
//! [`ParseError`]s without ever aborting.
//!
//! ```
//! let parsed = restmd_core::parse("## GET /health\n");
//! assert!(parsed.errors.is_empty());
//! assert_eq!(parsed.document.requests.len(), 1);
//! ```
//!
//! Templating (`{{var}}`) is parsed by [`parse`] and resolved by [`Resolver`].
//! HTTP execution does not live here yet — that is a later change.

#![warn(clippy::all)]

mod error;
mod model;
mod parser;
mod resolve;
mod span;
mod template;

pub use error::{ParseError, ParseErrorKind};
pub use model::{
    AssertOp, Assertion, Body, BodyLang, CaptureSource, CompareOp, ConfigValue, Directive,
    Document, Frontmatter, Header, Method, Request, Value,
};
pub use parser::parse;
pub use resolve::{
    Clock, Context, ContextBuilder, IdGen, RandomIdGen, ResolveError, ResolveErrorKind, Resolver,
    SystemClock,
};
pub use span::Span;
pub use template::{Template, TemplatePart, VarModifier};

/// The result of [`parse`]: a best-effort document plus every problem found.
///
/// `errors` being empty means a clean parse. A non-empty `errors` still comes
/// with a usable (partial) `document` — that is the whole point of
/// collect-and-continue.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub document: Document,
    pub errors: Vec<ParseError>,
}

impl Parsed {
    /// True if parsing found no problems.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}
