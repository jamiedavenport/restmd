//! Diagnostics: parse errors plus unknown / forward-referenced variables.

use line_index::LineIndex;
use lsp_types::{Diagnostic, DiagnosticSeverity, Range};
use restmd_core::Parsed;

use crate::analysis::{self, RefStatus};
use crate::convert::span_to_range;

pub fn diagnostics(parsed: &Parsed, index: &LineIndex) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for error in &parsed.errors {
        out.push(diag(
            span_to_range(index, error.span),
            DiagnosticSeverity::ERROR,
            error.kind.to_string(),
        ));
    }

    // `--var` and `RESTMD_VAR_*` are out-of-band, so an unresolved reference is a
    // warning, not an error.
    let defs = analysis::definitions(&parsed.document);
    for reference in analysis::references(&parsed.document) {
        let message = match analysis::classify(&reference.name, reference.span.start, &defs) {
            RefStatus::Known => continue,
            RefStatus::Forward => {
                format!("variable `{}` is used before it is defined", reference.name)
            }
            RefStatus::Unknown => format!(
                "unknown variable `{}` (not captured, set, or in any environment)",
                reference.name
            ),
        };
        out.push(diag(
            span_to_range(index, reference.span),
            DiagnosticSeverity::WARNING,
            message,
        ));
    }

    out
}

fn diag(range: Range, severity: DiagnosticSeverity, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("restmd".to_string()),
        message,
        ..Default::default()
    }
}
