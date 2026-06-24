//! The run report: per-request outcomes and the overall exit code.

use super::error::ExecError;
use crate::model::Method;

/// Result of one `capture` directive.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureResult {
    pub name: String,
    /// The captured value, or `None` if extraction failed.
    pub value: Option<String>,
    /// Why extraction failed, if it did.
    pub error: Option<String>,
}

impl CaptureResult {
    pub fn ok(&self) -> bool {
        self.error.is_none()
    }
}

/// Result of one `assert` directive.
#[derive(Debug, Clone, PartialEq)]
pub struct AssertionResult {
    /// Human-readable rendering of the assertion, e.g. `status == 200`.
    pub description: String,
    pub passed: bool,
    /// Extra context on failure, e.g. the actual value seen.
    pub detail: Option<String>,
}

/// What happened for a single request.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestOutcome {
    pub method: Method,
    pub url: String,
    /// Response status, or `None` if the request never completed.
    pub status: Option<u16>,
    pub captures: Vec<CaptureResult>,
    pub assertions: Vec<AssertionResult>,
    /// A fatal error for this request (aborts the run).
    pub error: Option<ExecError>,
}

impl RequestOutcome {
    /// True if the request sent, every capture extracted, and every assertion
    /// passed.
    pub fn passed(&self) -> bool {
        self.error.is_none()
            && self.captures.iter().all(CaptureResult::ok)
            && self.assertions.iter().all(|a| a.passed)
    }
}

/// The result of running a whole document.
#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    pub outcomes: Vec<RequestOutcome>,
    /// A pre-flight error not tied to a request (e.g. unknown `--env`).
    pub error: Option<ExecError>,
}

impl RunReport {
    /// The process exit code (spec §5.1): `0` success, `1` assertion/capture
    /// failure, `3` network error, `4` config error. The most severe wins.
    pub fn exit_code(&self) -> i32 {
        let mut worst = 0;
        if let Some(e) = &self.error {
            worst = worst.max(e.exit_code());
        }
        for outcome in &self.outcomes {
            let code = match &outcome.error {
                Some(e) => e.exit_code(),
                None if !outcome.passed() => 1,
                None => 0,
            };
            worst = worst.max(code);
        }
        worst
    }

    pub fn passed(&self) -> bool {
        self.exit_code() == 0
    }
}
