//! Templated strings: the `{{ ... }}` interpolation syntax.
//!
//! Paths and header values are parsed into a [`Template`] — a sequence of
//! literal and interpolation [`TemplatePart`]s. This pass only *records* that
//! an interpolation exists and what shape it has; it does not resolve anything.
//! Resolution (captures, `--var`, env, builtins) is an executor concern and a
//! later change.

use crate::error::{ParseError, ParseErrorKind};
use crate::span::Span;

/// A string that may contain `{{ ... }}` interpolations.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub parts: Vec<TemplatePart>,
    pub span: Span,
}

impl Template {
    /// True if the template is a single literal (or empty) with no
    /// interpolations — useful for callers that want a fast path.
    pub fn is_literal(&self) -> bool {
        self.parts
            .iter()
            .all(|p| matches!(p, TemplatePart::Literal(_)))
    }
}

/// One piece of a [`Template`].
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    /// Raw text, sent verbatim.
    Literal(String),
    /// A `{{ name }}` variable reference, with an optional modifier.
    Var {
        name: String,
        modifier: VarModifier,
        span: Span,
    },
    /// A `{{ func(args) }}` builtin call, e.g. `{{uuid()}}` or
    /// `{{base64(value)}}`.
    Call {
        func: String,
        args: Vec<String>,
        span: Span,
    },
}

/// The modifier suffix on a variable reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarModifier {
    /// `{{var}}` — required; resolution fails if undefined.
    None,
    /// `{{var?}}` — optional; empty string if undefined.
    Optional,
    /// `{{var!default}}` — fall back to the given literal if undefined.
    Default(String),
}

/// Parse `raw` (whose first byte sits at `base` in the original source) into a
/// [`Template`]. Any problems are pushed onto `errors`; parsing always returns
/// a usable template (malformed interpolations degrade to literals).
pub(crate) fn parse_template(raw: &str, base: usize, errors: &mut Vec<ParseError>) -> Template {
    let span = Span::new(base, base + raw.len());
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut rest = raw;
    let mut offset = base;

    while let Some(open) = rest.find("{{") {
        // Flush literal text before the interpolation.
        literal.push_str(&rest[..open]);
        let interp_start = offset + open;
        let after_open = &rest[open + 2..];

        let Some(close) = after_open.find("}}") else {
            errors.push(ParseError::new(
                ParseErrorKind::UnterminatedTemplate,
                Span::new(interp_start, span.end),
            ));
            // Treat the unterminated remainder as literal text.
            literal.push_str(&rest[open..]);
            rest = "";
            break;
        };

        let inner = &after_open[..close];
        let interp_end = interp_start + 2 + close + 2;
        let interp_span = Span::new(interp_start, interp_end);

        // An empty interpolation records an error and contributes no part.
        if let Some(part) = parse_interpolation(inner, interp_span, errors) {
            if !literal.is_empty() {
                parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
            }
            parts.push(part);
        }

        rest = &after_open[close + 2..];
        offset = interp_end;
    }

    literal.push_str(rest);
    if !literal.is_empty() || parts.is_empty() {
        parts.push(TemplatePart::Literal(literal));
    }

    Template { parts, span }
}

/// Parse the text *between* the braces into a [`TemplatePart`]. Returns `None`
/// only for an empty interpolation (which records an error and contributes no
/// part).
fn parse_interpolation(
    inner: &str,
    span: Span,
    errors: &mut Vec<ParseError>,
) -> Option<TemplatePart> {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        errors.push(ParseError::new(ParseErrorKind::EmptyInterpolation, span));
        return None;
    }

    // Function call: `name( args )`.
    if let Some(open) = trimmed.find('(')
        && trimmed.ends_with(')')
    {
        let func = trimmed[..open].trim().to_string();
        let arg_str = &trimmed[open + 1..trimmed.len() - 1];
        let args = if arg_str.trim().is_empty() {
            Vec::new()
        } else {
            arg_str.split(',').map(|a| a.trim().to_string()).collect()
        };
        return Some(TemplatePart::Call { func, args, span });
    }

    // Default modifier: `name!default` (everything after the first `!`).
    if let Some(bang) = trimmed.find('!') {
        let name = trimmed[..bang].trim().to_string();
        let default = trimmed[bang + 1..].to_string();
        return Some(TemplatePart::Var {
            name,
            modifier: VarModifier::Default(default),
            span,
        });
    }

    // Optional modifier: `name?`.
    if let Some(name) = trimmed.strip_suffix('?') {
        return Some(TemplatePart::Var {
            name: name.trim().to_string(),
            modifier: VarModifier::Optional,
            span,
        });
    }

    Some(TemplatePart::Var {
        name: trimmed.to_string(),
        modifier: VarModifier::None,
        span,
    })
}
