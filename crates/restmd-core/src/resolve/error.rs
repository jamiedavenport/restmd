//! Resolution errors.
//!
//! Resolution is *fail-fast*: [`super::Resolver::resolve`] returns the first
//! [`ResolveError`] it hits. (The shared per-part logic is factored so a
//! collect-all variant can be added for the LSP without changing this type.)
//! Errors carry a [`Span`] into the original source, mirroring `ParseError`.

use crate::span::Span;

/// A resolution problem, located at a [`Span`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} (at bytes {}..{})", .span.start, .span.end)]
pub struct ResolveError {
    pub kind: ResolveErrorKind,
    pub span: Span,
}

impl ResolveError {
    pub(crate) fn new(kind: ResolveErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of a [`ResolveError`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveErrorKind {
    #[error("undefined variable `{0}`")]
    UndefinedVariable(String),

    #[error("unknown function `{0}`")]
    UnknownFunction(String),

    #[error("`{func}` takes {expected} argument(s), got {got}")]
    WrongArgCount {
        func: &'static str,
        expected: usize,
        got: usize,
    },

    #[error("environment variable `{0}` is not set")]
    EnvVarNotSet(String),

    #[error("malformed template: {0}")]
    MalformedTemplate(String),
}
