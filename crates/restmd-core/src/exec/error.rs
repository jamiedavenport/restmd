//! Execution errors and their severity, which maps to CLI exit codes.

use super::transport::TransportError;
use crate::resolve::ResolveError;

/// A fatal error for a single request (and, in this slice, for the run — these
/// abort the loop). Assertion/capture *failures* are not errors; they are
/// recorded as results.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExecError {
    /// Bad configuration: unknown environment, un-joinable URL, malformed body,
    /// missing required header.
    #[error("configuration error: {0}")]
    Config(String),

    /// A `{{var}}` could not be resolved.
    #[error("{0}")]
    Resolve(#[from] ResolveError),

    /// The request could not be sent or the response could not be read.
    #[error("network error: {0}")]
    Network(#[from] TransportError),
}

impl ExecError {
    /// Severity as an exit code: config/resolve → 4, network → 3.
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            ExecError::Config(_) | ExecError::Resolve(_) => 4,
            ExecError::Network(_) => 3,
        }
    }
}
