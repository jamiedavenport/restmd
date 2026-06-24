//! Document symbols: one entry per request, for the editor outline.

use line_index::LineIndex;
use lsp_types::{DocumentSymbol, SymbolKind};
use restmd_core::Document;

use crate::convert::span_to_range;

pub fn document_symbols(doc: &Document, text: &str, index: &LineIndex) -> Vec<DocumentSymbol> {
    doc.requests
        .iter()
        .map(|request| {
            let name = request
                .heading_span
                .slice(text)
                .trim_start_matches('#')
                .trim()
                .to_string();
            #[allow(deprecated)] // `deprecated` field is required but deprecated
            DocumentSymbol {
                name,
                detail: None,
                kind: SymbolKind::METHOD,
                tags: None,
                deprecated: None,
                range: span_to_range(index, request.span),
                selection_range: span_to_range(index, request.heading_span),
                children: None,
            }
        })
        .collect()
}
