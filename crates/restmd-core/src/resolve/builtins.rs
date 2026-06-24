//! The five builtin template functions (spec §4.5 tier 5):
//! `uuid()`, `now()`, `timestamp()`, `base64(var)`, `env(NAME)`.
//!
//! `base64`'s argument is a *variable reference* resolved through the normal
//! tier 1–4 chain (so secrets held in variables can be encoded); `env`'s
//! argument is a *literal* OS environment-variable name.

use std::time::UNIX_EPOCH;

use base64::Engine as _;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::context::Context;
use super::error::{ResolveError, ResolveErrorKind};
use crate::span::Span;

/// Dispatch a `{{func(args)}}` call.
pub(crate) fn call_builtin(
    ctx: &Context,
    func: &str,
    args: &[String],
    span: Span,
) -> Result<String, ResolveError> {
    match func {
        "uuid" => {
            expect_args("uuid", args, 0, span)?;
            Ok(ctx.idgen.uuid())
        }
        "now" => {
            expect_args("now", args, 0, span)?;
            Ok(OffsetDateTime::from(ctx.clock.now())
                .format(&Rfc3339)
                .expect("RFC 3339 formatting of a valid datetime cannot fail"))
        }
        "timestamp" => {
            expect_args("timestamp", args, 0, span)?;
            let secs = ctx
                .clock
                .now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Ok(secs.to_string())
        }
        "base64" => {
            expect_args("base64", args, 1, span)?;
            let var = &args[0];
            let value = ctx.lookup(var).ok_or_else(|| {
                ResolveError::new(ResolveErrorKind::UndefinedVariable(var.clone()), span)
            })?;
            Ok(base64::engine::general_purpose::STANDARD.encode(value))
        }
        "env" => {
            expect_args("env", args, 1, span)?;
            let name = &args[0];
            ctx.os_env(name).map(str::to_string).ok_or_else(|| {
                ResolveError::new(ResolveErrorKind::EnvVarNotSet(name.clone()), span)
            })
        }
        other => Err(ResolveError::new(
            ResolveErrorKind::UnknownFunction(other.to_string()),
            span,
        )),
    }
}

fn expect_args(
    func: &'static str,
    args: &[String],
    expected: usize,
    span: Span,
) -> Result<(), ResolveError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(ResolveError::new(
            ResolveErrorKind::WrongArgCount {
                func,
                expected,
                got: args.len(),
            },
            span,
        ))
    }
}
