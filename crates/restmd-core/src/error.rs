//! Parse errors.
//!
//! The parser never aborts on malformed input: it accumulates a
//! [`ParseError`] for each problem and keeps producing a (partial) tree. This
//! is what lets the LSP show diagnostics on a file that is still being typed.
//! See [`crate::Parsed`].

use crate::span::Span;

/// A single parse problem, located at a [`Span`] in the source.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} (at bytes {}..{})", .span.start, .span.end)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub(crate) fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of a [`ParseError`]. User-facing and actionable: each variant
/// names a concrete construct the user can fix.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseErrorKind {
    #[error("frontmatter is not valid YAML: {0}")]
    Frontmatter(String),

    #[error("frontmatter block is opened with `---` but never closed")]
    UnterminatedFrontmatter,

    #[error("request heading is missing a path")]
    MissingPath,

    #[error("code fence is opened but never closed")]
    UnterminatedFence,

    #[error("unknown directive `{0}` (expected `capture`, `assert`, or `set`)")]
    UnknownDirective(String),

    #[error("malformed `{directive}` directive: {reason}")]
    MalformedDirective {
        directive: &'static str,
        reason: &'static str,
    },

    #[error("template is opened with `{{{{` but never closed")]
    UnterminatedTemplate,

    #[error("empty interpolation `{{{{}}}}`")]
    EmptyInterpolation,
}

impl ParseErrorKind {
    pub(crate) fn malformed(directive: &'static str, reason: &'static str) -> Self {
        Self::MalformedDirective { directive, reason }
    }
}
