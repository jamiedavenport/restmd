//! The parser: `&str` in, [`Parsed`] out.
//!
//! restmd's surface syntax is line-oriented (headings, header lines, fenced
//! bodies, `>` directives), so this is a hand-written line scanner rather than a
//! general markdown parse. That buys two things the project needs from day one:
//! exact byte spans, and *collect-and-continue* recovery — a malformed line
//! becomes a [`ParseError`] and the scan moves on, so a half-written file still
//! yields a usable tree.

use std::str::FromStr;

use crate::Parsed;
use crate::error::{ParseError, ParseErrorKind};
use crate::model::*;
use crate::span::Span;
use crate::template::parse_template;

/// Parse a restmd document. Never fails: problems are reported in
/// [`Parsed::errors`] alongside a best-effort [`Parsed::document`].
pub fn parse(src: &str) -> Parsed {
    let mut errors = Vec::new();
    let lines = split_lines(src);

    let (frontmatter, body_start) = parse_frontmatter(src, &lines, &mut errors);
    let requests = parse_requests(src, &lines, body_start, &mut errors);

    Parsed {
        document: Document {
            frontmatter,
            requests,
        },
        errors,
    }
}

/// A source line with its byte offsets. `text` excludes the line terminator;
/// `start..end` is the span of `text` within the source.
struct Line<'src> {
    text: &'src str,
    start: usize,
    end: usize,
}

fn split_lines(src: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut offset = 0;
    for raw in src.split_inclusive('\n') {
        let text = raw.strip_suffix('\n').unwrap_or(raw);
        let text = text.strip_suffix('\r').unwrap_or(text);
        lines.push(Line {
            text,
            start: offset,
            end: offset + text.len(),
        });
        offset += raw.len();
    }
    lines
}

/// Byte offset of subslice `sub` within `parent`. `sub` must be a subslice of
/// `parent` (same allocation), which is always true for slices we carve off a
/// line's text.
fn offset_in(parent: &str, sub: &str) -> usize {
    sub.as_ptr() as usize - parent.as_ptr() as usize
}

/// Split `s` into its first whitespace-delimited word and the remainder, both
/// as subslices of `s` (so offsets stay computable). Leading whitespace is
/// trimmed first.
fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// The heading level of a markdown ATX heading (`#` → 1, `##` → 2, …), or
/// `None` if the line is not a heading. A run of `#` must be followed by
/// whitespace or end-of-line.
fn heading_level(text: &str) -> Option<usize> {
    let hashes = text.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 {
        return None;
    }
    let after = &text[hashes..];
    if after.is_empty() || after.starts_with(char::is_whitespace) {
        Some(hashes)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

/// Parse a leading `---` … `---` YAML block, if present. Returns the parsed
/// frontmatter (if valid) and the index of the first body line.
fn parse_frontmatter(
    src: &str,
    lines: &[Line<'_>],
    errors: &mut Vec<ParseError>,
) -> (Option<Frontmatter>, usize) {
    let Some(first) = lines.first() else {
        return (None, 0);
    };
    if first.text.trim_end() != "---" {
        return (None, 0);
    }

    // Find the closing delimiter line.
    let close = lines[1..].iter().position(|l| {
        let t = l.text.trim_end();
        t == "---" || t == "..."
    });

    let Some(rel) = close else {
        errors.push(ParseError::new(
            ParseErrorKind::UnterminatedFrontmatter,
            Span::new(first.start, src.len()),
        ));
        // Skip the opening `---` so requests after it are still parsed.
        return (None, 1);
    };
    let close_idx = rel + 1;

    let yaml_start = lines[1].start;
    let yaml_end = lines[close_idx].start;
    let yaml = &src[yaml_start..yaml_end];
    let fm_span = Span::new(first.start, lines[close_idx].end);

    let frontmatter = if yaml.trim().is_empty() {
        Some(Frontmatter::default())
    } else {
        match serde_yaml::from_str::<Frontmatter>(yaml) {
            Ok(fm) => Some(fm),
            Err(e) => {
                errors.push(ParseError::new(
                    ParseErrorKind::Frontmatter(e.to_string()),
                    fm_span,
                ));
                None
            }
        }
    };

    (frontmatter, close_idx + 1)
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

fn parse_requests(
    src: &str,
    lines: &[Line<'_>],
    body_start: usize,
    errors: &mut Vec<ParseError>,
) -> Vec<Request> {
    let mut requests = Vec::new();
    let mut i = body_start;

    while i < lines.len() {
        // A request begins at an H2 whose first token is a known method.
        if heading_level(lines[i].text) == Some(2)
            && let Some(method) = leading_method(lines[i].text)
        {
            let end = region_end(lines, i + 1);
            requests.push(parse_request(src, lines, i, end, method, errors));
            i = end;
            continue;
        }
        i += 1;
    }

    requests
}

/// The method named by an `## METHOD …` heading, if the first token is a known
/// HTTP method. An H2 with any other first token is ordinary prose.
fn leading_method(text: &str) -> Option<Method> {
    let content = text.trim_start_matches('#').trim_start();
    let (first, _) = split_first_word(content);
    Method::from_str(first).ok()
}

/// Index of the line that ends the request region started at `from`: the next
/// H1 or H2 heading, or end of input. H3+ headings stay inside the region.
fn region_end(lines: &[Line<'_>], from: usize) -> usize {
    lines[from..]
        .iter()
        .position(|l| matches!(heading_level(l.text), Some(1 | 2)))
        .map(|rel| from + rel)
        .unwrap_or(lines.len())
}

fn parse_request(
    src: &str,
    lines: &[Line<'_>],
    heading_idx: usize,
    end: usize,
    method: Method,
    errors: &mut Vec<ParseError>,
) -> Request {
    let heading = &lines[heading_idx];
    let heading_span = Span::new(heading.start, heading.end);
    let span = Span::new(heading.start, lines[end - 1].end);

    // Target: everything on the heading line after the method token.
    let content = heading.text.trim_start_matches('#').trim_start();
    let (_method_tok, after_method) = split_first_word(content);
    let target_str = after_method.trim_end();
    let target = if target_str.is_empty() {
        errors.push(ParseError::new(ParseErrorKind::MissingPath, heading_span));
        parse_template("", heading.end, errors)
    } else {
        let base = heading.start + offset_in(heading.text, target_str);
        parse_template(target_str, base, errors)
    };

    // Header lines: the contiguous block of `Name: value` lines immediately
    // after the heading. The block ends at the first blank line, fence,
    // directive, or non-header line.
    let mut headers = Vec::new();
    let mut idx = heading_idx + 1;
    while idx < end {
        let line = &lines[idx];
        let t = line.text;
        if t.trim().is_empty() || t.starts_with('>') || t.starts_with("```") {
            break;
        }
        match parse_header(line, errors) {
            Some(h) => headers.push(h),
            None => break, // not a header -> prose; header block is over
        }
        idx += 1;
    }

    // Remainder of the region: scan for one body fence and any directives,
    // skipping prose. A linear scan keeps fence contents (which may contain
    // `>` lines) from being mistaken for directives.
    let mut body = None;
    let mut directives = Vec::new();
    while idx < end {
        let line = &lines[idx];
        if line.text.starts_with("```") {
            idx = scan_fence(src, lines, idx, end, &mut body, errors);
        } else if line.text.starts_with('>') {
            if let Some(d) = parse_directive(line, errors) {
                directives.push(d);
            }
            idx += 1;
        } else {
            idx += 1;
        }
    }

    Request {
        method,
        target,
        headers,
        body,
        directives,
        span,
        heading_span,
    }
}

/// Parse a `Name: value` header line. Returns `None` if the line does not look
/// like a header (no colon, or an invalid name), which signals the end of the
/// header block rather than an error — the line is just prose.
fn parse_header(line: &Line<'_>, errors: &mut Vec<ParseError>) -> Option<Header> {
    let text = line.text;
    let colon = text.find(':')?;
    let name = &text[..colon];
    if name.is_empty() || !is_header_name(name) {
        return None;
    }

    // Value starts after the colon and one optional space.
    let after = &text[colon + 1..];
    let value_str = after.strip_prefix(' ').unwrap_or(after);
    let trimmed = value_str.trim_end();
    let base = line.start + offset_in(text, value_str);
    let value = parse_template(trimmed, base, errors);

    Some(Header {
        name: name.to_string(),
        value,
        span: Span::new(line.start, line.end),
    })
}

/// HTTP header field-name characters we accept (a practical subset of RFC 7230
/// token chars). Notably rules out spaces, so `Some prose: text` is treated as
/// prose, not a header.
fn is_header_name(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Handle a fenced code block beginning at `open`. If it is the first
/// recognized-language fence in the region and no body is set yet, it becomes
/// the body. Returns the index to continue scanning from.
fn scan_fence(
    src: &str,
    lines: &[Line<'_>],
    open: usize,
    end: usize,
    body: &mut Option<Body>,
    errors: &mut Vec<ParseError>,
) -> usize {
    let open_line = &lines[open];
    let fence_len = open_line.text.bytes().take_while(|&b| b == b'`').count();
    let info = open_line.text[fence_len..].trim();

    // Find the closing fence within the region.
    let close = lines[open + 1..end].iter().position(|l| {
        let t = l.text.trim();
        t.len() >= fence_len && t.bytes().all(|b| b == b'`')
    });

    let Some(rel) = close else {
        errors.push(ParseError::new(
            ParseErrorKind::UnterminatedFence,
            Span::new(open_line.start, lines[end - 1].end),
        ));
        return end;
    };
    let close_idx = open + 1 + rel;

    // Only recognized languages become bodies; everything else is prose
    // (illustrative code blocks in documentation stay untouched).
    if body.is_none()
        && let Ok(lang) = BodyLang::from_str(info)
    {
        let content_start = lines[open + 1].start;
        let content_end = lines[close_idx].start;
        let raw = &src[content_start..content_end];
        let content = raw
            .strip_suffix('\n')
            .map(|s| s.strip_suffix('\r').unwrap_or(s))
            .unwrap_or(raw)
            .to_string();
        *body = Some(Body {
            lang,
            content,
            span: Span::new(open_line.start, lines[close_idx].end),
        });
    }

    close_idx + 1
}

// ---------------------------------------------------------------------------
// Directives
// ---------------------------------------------------------------------------

/// Parse a single `>` directive line. Returns `None` (with an error recorded)
/// for a malformed or unknown directive.
fn parse_directive(line: &Line<'_>, errors: &mut Vec<ParseError>) -> Option<Directive> {
    let span = Span::new(line.start, line.end);
    let body = line.text.trim_start_matches('>').trim_start();
    let (keyword, rest) = split_first_word(body);

    match keyword {
        "capture" => parse_capture(line, rest, span, errors),
        "assert" => parse_assert(rest, span, errors),
        "set" => parse_set(line, rest, span, errors),
        other => {
            errors.push(ParseError::new(
                ParseErrorKind::UnknownDirective(other.to_string()),
                span,
            ));
            None
        }
    }
}

fn parse_capture(
    _line: &Line<'_>,
    rest: &str,
    span: Span,
    errors: &mut Vec<ParseError>,
) -> Option<Directive> {
    let Some((name, source)) = rest.split_once('=') else {
        errors.push(ParseError::new(
            ParseErrorKind::malformed("capture", "expected `NAME = <source>`"),
            span,
        ));
        return None;
    };
    let name = name.trim();
    if name.is_empty() {
        errors.push(ParseError::new(
            ParseErrorKind::malformed("capture", "missing capture name"),
            span,
        ));
        return None;
    }

    let source = source.trim();
    let source = if source == "response.status" {
        CaptureSource::Status
    } else if let Some(header) = source.strip_prefix("response.headers.") {
        CaptureSource::Header(header.to_string())
    } else if source.starts_with('$') {
        CaptureSource::JsonPath(source.to_string())
    } else {
        errors.push(ParseError::new(
            ParseErrorKind::malformed(
                "capture",
                "source must be a `$` JSONPath, `response.headers.NAME`, or `response.status`",
            ),
            span,
        ));
        return None;
    };

    Some(Directive::Capture {
        name: name.to_string(),
        source,
        span,
    })
}

fn parse_assert(rest: &str, span: Span, errors: &mut Vec<ParseError>) -> Option<Directive> {
    let malformed = |reason| ParseError::new(ParseErrorKind::malformed("assert", reason), span);

    let (lhs, after) = split_first_word(rest);
    let (op_tok, value_str) = split_first_word(after);

    let assertion = if lhs == "status" {
        let Some(op) = CompareOp::parse(op_tok) else {
            errors.push(malformed("expected a comparison operator after `status`"));
            return None;
        };
        let Ok(code) = value_str.trim().parse::<u16>() else {
            errors.push(malformed("status assertion needs a numeric code"));
            return None;
        };
        Assertion::Status { op, code }
    } else if lhs.starts_with('$') {
        let op = match op_tok {
            "exists" => AssertOp::Exists,
            "matches" => {
                let pat = value_str.trim();
                let Some(re) = pat.strip_prefix('/').and_then(|p| p.strip_suffix('/')) else {
                    errors.push(malformed("`matches` expects a /regex/"));
                    return None;
                };
                AssertOp::Matches(re.to_string())
            }
            _ => {
                let Some(op) = CompareOp::parse(op_tok) else {
                    errors.push(malformed("unknown operator in body assertion"));
                    return None;
                };
                if value_str.trim().is_empty() {
                    errors.push(malformed("comparison needs a right-hand value"));
                    return None;
                }
                AssertOp::Compare(op, Value::parse(value_str))
            }
        };
        Assertion::Body {
            path: lhs.to_string(),
            op,
        }
    } else {
        errors.push(malformed("left side must be `status` or a `$` JSONPath"));
        return None;
    };

    Some(Directive::Assert { assertion, span })
}

fn parse_set(
    line: &Line<'_>,
    rest: &str,
    span: Span,
    errors: &mut Vec<ParseError>,
) -> Option<Directive> {
    let Some((name, value_raw)) = rest.split_once('=') else {
        errors.push(ParseError::new(
            ParseErrorKind::malformed("set", "expected `NAME = VALUE`"),
            span,
        ));
        return None;
    };
    let name = name.trim();
    if name.is_empty() {
        errors.push(ParseError::new(
            ParseErrorKind::malformed("set", "missing variable name"),
            span,
        ));
        return None;
    }

    let value_str = value_raw.trim();
    let base = line.start + offset_in(line.text, value_str);
    let value = parse_template(value_str, base, errors);

    Some(Directive::Set {
        name: name.to_string(),
        value,
        span,
    })
}
