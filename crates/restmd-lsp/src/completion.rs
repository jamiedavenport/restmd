//! Positional completion: what to suggest given the cursor's surroundings.

use std::collections::BTreeSet;

use lsp_types::{CompletionItem, CompletionItemKind};
use restmd_core::Parsed;

use crate::analysis::{self, VarOrigin};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const FENCE_LANGS: &[&str] = &["json", "xml", "form", "text", "graphql"];
const DIRECTIVES: &[&str] = &["capture", "assert", "set"];
const FRONTMATTER_KEYS: &[&str] = &[
    "base",
    "openapi",
    "environments",
    "defaults",
    "timeout",
    "retries",
];
const HEADERS: &[&str] = &[
    "Accept",
    "Accept-Encoding",
    "Authorization",
    "Cache-Control",
    "Content-Type",
    "Cookie",
    "If-None-Match",
    "Idempotency-Key",
    "User-Agent",
    "X-Request-Id",
];

/// Compute completions for the cursor at byte `offset` in `text`.
pub fn completion(text: &str, offset: usize, parsed: &Parsed) -> Vec<CompletionItem> {
    let line_start = text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let prefix = &text[line_start..offset];

    // 1. Inside an unclosed `{{ … }}` → variables and builtins (works anywhere,
    //    including frontmatter like `base: https://api.{{env}}…`).
    if let Some(open) = prefix.rfind("{{")
        && !prefix[open + 2..].contains("}}")
    {
        return variable_items(parsed, offset);
    }

    // 2. In the frontmatter block → top-level config keys.
    if in_frontmatter(text, offset) {
        if is_key_prefix(prefix) {
            return keyword_items(FRONTMATTER_KEYS, CompletionItemKind::FIELD);
        }
        return Vec::new();
    }

    // 3. `## METHOD` heading, still typing the method.
    if let Some(rest) = prefix.strip_prefix("##")
        && rest.starts_with(char::is_whitespace)
        && !rest.trim_start().contains(char::is_whitespace)
    {
        return keyword_items(METHODS, CompletionItemKind::KEYWORD);
    }

    // 3. Fenced body language, right after the opening ```.
    if let Some(rest) = prefix.strip_prefix("```")
        && !rest.contains(|c: char| c.is_whitespace() || c == '`')
    {
        return keyword_items(FENCE_LANGS, CompletionItemKind::ENUM_MEMBER);
    }

    // 4. Directive keyword after `> `.
    if let Some(rest) = prefix.strip_prefix('>')
        && rest.starts_with(char::is_whitespace)
        && !rest.trim_start().contains(char::is_whitespace)
    {
        return keyword_items(DIRECTIVES, CompletionItemKind::KEYWORD);
    }

    // 5. Header name: a bare identifier at line start, inside a request.
    if is_header_prefix(prefix) && in_request(parsed, offset) {
        return keyword_items(HEADERS, CompletionItemKind::FIELD);
    }

    Vec::new()
}

fn variable_items(parsed: &Parsed, offset: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();

    for def in analysis::definitions(&parsed.document) {
        let available = def.def_span.is_none_or(|span| span.start < offset);
        if available && seen.insert(def.name.clone()) {
            let detail = match def.origin {
                VarOrigin::Capture => "captured",
                VarOrigin::Set => "set",
                VarOrigin::Environment => "environment",
            };
            items.push(CompletionItem {
                label: def.name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }
    }

    for builtin in analysis::BUILTINS {
        items.push(CompletionItem {
            label: format!("{builtin}()"),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("builtin".to_string()),
            ..Default::default()
        });
    }

    items
}

fn keyword_items(words: &[&str], kind: CompletionItemKind) -> Vec<CompletionItem> {
    words
        .iter()
        .map(|w| CompletionItem {
            label: w.to_string(),
            kind: Some(kind),
            ..Default::default()
        })
        .collect()
}

/// Whether `offset` is inside the leading `---` … `---` frontmatter block.
fn in_frontmatter(text: &str, offset: usize) -> bool {
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return false;
    };
    if first.trim_end() != "---" {
        return false;
    }
    let mut pos = first.len();
    for line in lines {
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            return offset <= pos; // before the closing delimiter
        }
        pos += line.len();
    }
    true // unterminated frontmatter: treat the rest as frontmatter
}

/// A bare key being typed at the start of a line (optionally indented).
fn is_key_prefix(prefix: &str) -> bool {
    let key = prefix.trim_start();
    key.chars().all(|c| c.is_ascii_alphabetic() || c == '_')
}

fn is_header_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && prefix.starts_with(|c: char| c.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn in_request(parsed: &Parsed, offset: usize) -> bool {
    parsed
        .document
        .requests
        .iter()
        .any(|r| r.span.start <= offset && offset <= r.span.end)
}
