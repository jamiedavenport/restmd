//! Variable analysis shared by completion, diagnostics, and hover.
//!
//! Collects where variables are *defined* (captures, sets, frontmatter
//! environment keys) and where they are *referenced* (in request targets,
//! header values, and `set` values), and classifies each reference.
//!
//! References inside request *bodies* are not analyzed yet — that needs the core
//! to expose the body's content span.

use std::collections::BTreeSet;

use restmd_core::{Directive, Document, Span, Template, TemplatePart};

/// The builtin template functions (`{{uuid()}}` etc.).
pub const BUILTINS: &[&str] = &["uuid", "now", "timestamp", "base64", "env"];

/// Where a variable comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarOrigin {
    Capture,
    Set,
    Environment,
}

/// A variable definition.
#[derive(Debug, Clone)]
pub struct VarDef {
    pub name: String,
    pub origin: VarOrigin,
    /// Index of the request that defines it (for captures/sets).
    pub request_index: Option<usize>,
    /// Where it becomes available; `None` for environment keys (always).
    pub def_span: Option<Span>,
}

impl VarDef {
    /// Whether this definition is in scope for a reference at `offset`.
    fn available_at(&self, offset: usize) -> bool {
        match self.def_span {
            None => true,
            Some(span) => span.start < offset,
        }
    }
}

/// A variable reference (a `{{name}}` use).
#[derive(Debug, Clone)]
pub struct VarRef {
    pub name: String,
    pub span: Span,
}

/// How a reference resolves against the available definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefStatus {
    Known,
    /// Defined, but only by a later request.
    Forward,
    Unknown,
}

/// All variable definitions in a document, in source order.
pub fn definitions(doc: &Document) -> Vec<VarDef> {
    let mut defs = Vec::new();

    if let Some(frontmatter) = &doc.frontmatter {
        let mut seen = BTreeSet::new();
        for env in frontmatter.environments.values() {
            for key in env.keys() {
                if seen.insert(key.clone()) {
                    defs.push(VarDef {
                        name: key.clone(),
                        origin: VarOrigin::Environment,
                        request_index: None,
                        def_span: None,
                    });
                }
            }
        }
    }

    for (index, request) in doc.requests.iter().enumerate() {
        for directive in &request.directives {
            let (name, origin, span) = match directive {
                Directive::Capture { name, span, .. } => (name, VarOrigin::Capture, *span),
                Directive::Set { name, span, .. } => (name, VarOrigin::Set, *span),
                Directive::Assert { .. } => continue,
            };
            defs.push(VarDef {
                name: name.clone(),
                origin,
                request_index: Some(index),
                def_span: Some(span),
            });
        }
    }

    defs
}

/// All variable references in targets, header values, and `set` values.
pub fn references(doc: &Document) -> Vec<VarRef> {
    let mut refs = Vec::new();
    for request in &doc.requests {
        collect(&request.target, &mut refs);
        for header in &request.headers {
            collect(&header.value, &mut refs);
        }
        for directive in &request.directives {
            if let Directive::Set { value, .. } = directive {
                collect(value, &mut refs);
            }
        }
    }
    refs
}

fn collect(template: &Template, out: &mut Vec<VarRef>) {
    for part in &template.parts {
        if let TemplatePart::Var { name, span, .. } = part {
            out.push(VarRef {
                name: name.clone(),
                span: *span,
            });
        }
    }
}

/// Classify a reference at `offset` against the definitions.
pub fn classify(name: &str, offset: usize, defs: &[VarDef]) -> RefStatus {
    let matching: Vec<&VarDef> = defs.iter().filter(|d| d.name == name).collect();
    if matching.is_empty() {
        return RefStatus::Unknown;
    }
    if matching.iter().any(|d| d.available_at(offset)) {
        RefStatus::Known
    } else {
        RefStatus::Forward
    }
}
