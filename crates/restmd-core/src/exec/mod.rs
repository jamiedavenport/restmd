//! The executor: run a parsed [`Document`] over HTTP.
//!
//! [`Runner::run`] sends each request in order, applies its `capture` / `assert`
//! / `set` directives, threads captured state forward into later requests, and
//! returns a [`RunReport`]. All network access goes through an [`HttpTransport`]
//! so the logic is testable against a real local server (see `tests/exec.rs`).
//!
//! This is a vertical slice: `json`/`text`/`xml` bodies and status/JSONPath
//! assertions. `form`/`graphql` bodies, `timeout`, and `retries` are not handled
//! yet.

mod assert;
mod build;
mod capture;
mod error;
mod report;
mod transport;

pub use error::ExecError;
pub use report::{AssertionResult, CaptureResult, RequestOutcome, RunReport};
pub use transport::{HttpRequest, HttpResponse, HttpTransport, ReqwestTransport, TransportError};

use std::collections::BTreeMap;

use crate::model::{ConfigValue, Directive, Document, Request};
use crate::resolve::{Context, Resolver};

/// Inputs that vary per run: the selected environment, CLI `--var` values, and
/// whether to expose the process environment (tier 3 + the `env()` builtin).
pub struct RunOptions {
    pub env: Option<String>,
    pub vars: BTreeMap<String, String>,
    pub include_os_env: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            env: None,
            vars: BTreeMap::new(),
            include_os_env: true,
        }
    }
}

/// Runs documents against a given [`HttpTransport`].
pub struct Runner<T: HttpTransport> {
    transport: T,
}

impl<T: HttpTransport> Runner<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Execute every request in `doc` sequentially.
    pub fn run(&self, doc: &Document, opts: &RunOptions) -> RunReport {
        let frontmatter = doc.frontmatter.as_ref();
        let empty_env = BTreeMap::new();

        // Resolve the selected environment block; an unknown name is a config error.
        let env_block: &BTreeMap<String, ConfigValue> = match &opts.env {
            Some(name) => match frontmatter.and_then(|f| f.environments.get(name)) {
                Some(block) => block,
                None => {
                    return RunReport {
                        outcomes: Vec::new(),
                        error: Some(ExecError::Config(format!("unknown environment `{name}`"))),
                    };
                }
            },
            None => &empty_env,
        };

        let os_env: BTreeMap<String, String> = if opts.include_os_env {
            std::env::vars().collect()
        } else {
            BTreeMap::new()
        };
        let base = frontmatter.and_then(|f| f.base.as_deref());
        let defaults = frontmatter.map(|f| f.defaults.clone()).unwrap_or_default();

        // Tier 1: captures and `set` values accumulated across the run.
        let mut runtime: BTreeMap<String, String> = BTreeMap::new();
        let mut outcomes = Vec::new();

        for request in &doc.requests {
            let ctx = Context::builder()
                .captures(runtime.clone())
                .vars(opts.vars.clone())
                .os_env(os_env.clone())
                .environment(env_block)
                .build();
            let resolver = Resolver::new(&ctx);

            let outcome = self.run_one(request, base, &defaults, &resolver, &mut runtime);
            let abort = outcome.error.is_some();
            outcomes.push(outcome);
            if abort {
                break;
            }
        }

        RunReport {
            outcomes,
            error: None,
        }
    }

    fn run_one(
        &self,
        request: &Request,
        base: Option<&str>,
        defaults: &BTreeMap<String, String>,
        resolver: &Resolver,
        runtime: &mut BTreeMap<String, String>,
    ) -> RequestOutcome {
        let fail = |url: String, error: ExecError| RequestOutcome {
            method: request.method,
            url,
            status: None,
            captures: Vec::new(),
            assertions: Vec::new(),
            error: Some(error),
        };

        let http_req = match build::build_request(request, base, defaults, resolver) {
            Ok(r) => r,
            Err(e) => return fail(String::new(), e),
        };
        let url = http_req.url.clone();
        let response = match self.transport.send(&http_req) {
            Ok(r) => r,
            Err(e) => return fail(url, ExecError::Network(e)),
        };

        // Captures feed later requests; record each result.
        let mut captures = Vec::new();
        for directive in &request.directives {
            if let Directive::Capture { name, source, .. } = directive {
                match capture::apply_capture(source, &response) {
                    Ok(value) => {
                        runtime.insert(name.clone(), value.clone());
                        captures.push(CaptureResult {
                            name: name.clone(),
                            value: Some(value),
                            error: None,
                        });
                    }
                    Err(reason) => captures.push(CaptureResult {
                        name: name.clone(),
                        value: None,
                        error: Some(reason),
                    }),
                }
            }
        }

        let assertions = request
            .directives
            .iter()
            .filter_map(|d| match d {
                Directive::Assert { assertion, .. } => {
                    Some(assert::eval_assertion(assertion, &response))
                }
                _ => None,
            })
            .collect();

        // `set` binds a downstream variable; ignore a value that fails to resolve.
        for directive in &request.directives {
            if let Directive::Set { name, value, .. } = directive
                && let Ok(resolved) = resolver.resolve(value)
            {
                runtime.insert(name.clone(), resolved);
            }
        }

        RequestOutcome {
            method: request.method,
            url,
            status: Some(response.status),
            captures,
            assertions,
            error: None,
        }
    }
}
