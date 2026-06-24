//! Variable resolution: turn a [`Template`] into a concrete `String`.
//!
//! This is the layer between parsing and execution. It applies the spec §4.5
//! lookup chain and builtins to each [`TemplatePart`], using a [`Context`] for
//! the variable sources. It is pure (no HTTP, no async) and *single-pass*: a
//! substituted value is emitted literally and never re-scanned for `{{…}}`,
//! which rules out resolution cycles and template injection.

mod builtins;
mod context;
mod error;

pub use context::{Clock, Context, ContextBuilder, IdGen, RandomIdGen, SystemClock};
pub use error::{ResolveError, ResolveErrorKind};

use crate::template::{Template, TemplatePart, VarModifier, parse_template};

/// Resolves templates against a borrowed [`Context`].
///
/// ```
/// use std::collections::BTreeMap;
/// use restmd_core::{parse, Context, Resolver};
///
/// let doc = parse("## GET /users/{{id}}\n").document;
/// let ctx = Context::builder()
///     .vars(BTreeMap::from([("id".to_string(), "42".to_string())]))
///     .build();
/// let resolver = Resolver::new(&ctx);
/// assert_eq!(resolver.resolve(&doc.requests[0].target).unwrap(), "/users/42");
/// ```
pub struct Resolver<'ctx> {
    ctx: &'ctx Context,
}

impl<'ctx> Resolver<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx }
    }

    /// Resolve a parsed [`Template`] to a string, failing fast on the first
    /// problem (undefined variable, unknown function, …).
    pub fn resolve(&self, template: &Template) -> Result<String, ResolveError> {
        let mut out = String::new();
        for part in &template.parts {
            self.resolve_part(part, &mut out)?;
        }
        Ok(out)
    }

    /// Parse `raw` as a template and resolve it. A convenience for templated
    /// strings that were never parsed into a [`Template`] (e.g. the frontmatter
    /// `base` URL). Spans in any error are relative to `raw`.
    pub fn resolve_str(&self, raw: &str) -> Result<String, ResolveError> {
        let mut parse_errors = Vec::new();
        let template = parse_template(raw, 0, &mut parse_errors);
        if let Some(first) = parse_errors.into_iter().next() {
            return Err(ResolveError::new(
                ResolveErrorKind::MalformedTemplate(first.kind.to_string()),
                first.span,
            ));
        }
        self.resolve(&template)
    }

    /// Resolve a single part, appending to `out`. This is the shared core a
    /// future collect-all (LSP) variant would reuse.
    fn resolve_part(&self, part: &TemplatePart, out: &mut String) -> Result<(), ResolveError> {
        match part {
            TemplatePart::Literal(s) => out.push_str(s),
            TemplatePart::Var {
                name,
                modifier,
                span,
            } => match self.ctx.lookup(name) {
                Some(value) => out.push_str(value),
                None => match modifier {
                    VarModifier::Optional => {}
                    VarModifier::Default(default) => out.push_str(default),
                    VarModifier::None => {
                        return Err(ResolveError::new(
                            ResolveErrorKind::UndefinedVariable(name.clone()),
                            *span,
                        ));
                    }
                },
            },
            TemplatePart::Call { func, args, span } => {
                out.push_str(&builtins::call_builtin(self.ctx, func, args, *span)?);
            }
        }
        Ok(())
    }
}
