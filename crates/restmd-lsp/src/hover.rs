//! Hover: explain where a `{{variable}}` under the cursor comes from.

use line_index::LineIndex;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use restmd_core::Document;

use crate::analysis::{self, VarDef, VarOrigin};
use crate::convert::span_to_range;

pub fn hover(doc: &Document, text: &str, index: &LineIndex, offset: usize) -> Option<Hover> {
    let reference = analysis::references(doc)
        .into_iter()
        .find(|r| r.span.start <= offset && offset <= r.span.end)?;

    let defs = analysis::definitions(doc);
    let value = describe(&reference.name, reference.span.start, &defs, doc, text);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(span_to_range(index, reference.span)),
    })
}

fn describe(name: &str, offset: usize, defs: &[VarDef], doc: &Document, text: &str) -> String {
    let matching: Vec<&VarDef> = defs.iter().filter(|d| d.name == name).collect();
    let Some(def) = matching
        .iter()
        .find(|d| d.def_span.is_none_or(|s| s.start < offset))
        .or_else(|| matching.first())
    else {
        return format!("`{name}` — **unknown variable**");
    };

    match def.origin {
        VarOrigin::Environment => format!("`{name}` — from a frontmatter environment block"),
        VarOrigin::Capture | VarOrigin::Set => {
            let verb = if def.origin == VarOrigin::Capture {
                "captured"
            } else {
                "set"
            };
            match def.request_index.and_then(|i| doc.requests.get(i)) {
                Some(req) => {
                    let heading = req.heading_span.slice(text).trim_start_matches('#').trim();
                    format!("`{name}` — {verb} by `{heading}`")
                }
                None => format!("`{name}` — {verb}"),
            }
        }
    }
}
