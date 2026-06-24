//! Building a wire [`HttpRequest`] from a parsed [`Request`]: resolve templates,
//! join the URL, merge headers, serialize the body.

use std::collections::BTreeMap;

use reqwest::Url;

use super::error::ExecError;
use super::transport::HttpRequest;
use crate::model::{Body, BodyLang, Request};
use crate::resolve::Resolver;

/// Build the request to send. `base` is the (still-templated) frontmatter base
/// URL; `defaults` the frontmatter default headers. All templates are resolved
/// through `resolver`.
pub(crate) fn build_request(
    request: &Request,
    base: Option<&str>,
    defaults: &BTreeMap<String, String>,
    resolver: &Resolver,
) -> Result<HttpRequest, ExecError> {
    let target = resolver.resolve(&request.target)?;
    let resolved_base = match base {
        Some(b) => Some(resolver.resolve_str(b)?),
        None => None,
    };
    let url = join_url(resolved_base.as_deref(), &target)?;

    // Defaults first, then request headers override case-insensitively.
    let mut headers: Vec<(String, String)> = Vec::new();
    for (name, value) in defaults {
        set_header(&mut headers, name, resolver.resolve_str(value)?);
    }
    for header in &request.headers {
        set_header(&mut headers, &header.name, resolver.resolve(&header.value)?);
    }

    let body = match &request.body {
        Some(b) => Some(serialize_body(b, resolver, &mut headers)?),
        None => None,
    };

    Ok(HttpRequest {
        method: request.method,
        url,
        headers,
        body,
    })
}

/// Join a relative target onto the base URL. An absolute target (with scheme)
/// bypasses the base; a relative target with no base is a config error.
fn join_url(base: Option<&str>, target: &str) -> Result<String, ExecError> {
    if target.starts_with("http://") || target.starts_with("https://") {
        return Ok(target.to_string());
    }
    match base {
        Some(b) => {
            let base_url = Url::parse(b)
                .map_err(|e| ExecError::Config(format!("invalid base URL `{b}`: {e}")))?;
            base_url
                .join(target)
                .map(String::from)
                .map_err(|e| ExecError::Config(format!("could not join `{target}` onto base: {e}")))
        }
        None => Err(ExecError::Config(format!(
            "relative target `{target}` but no base URL is set"
        ))),
    }
}

/// Resolve and serialize a body, setting a default Content-Type when the request
/// did not specify one.
fn serialize_body(
    body: &Body,
    resolver: &Resolver,
    headers: &mut Vec<(String, String)>,
) -> Result<Vec<u8>, ExecError> {
    let content = resolver.resolve_str(&body.content)?;
    match body.lang {
        BodyLang::Json => {
            serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|e| ExecError::Config(format!("body is not valid JSON: {e}")))?;
            default_content_type(headers, "application/json");
            Ok(content.into_bytes())
        }
        BodyLang::Xml => {
            default_content_type(headers, "application/xml");
            Ok(content.into_bytes())
        }
        BodyLang::Text => {
            if !has_header(headers, "content-type") {
                return Err(ExecError::Config(
                    "a `text` body requires an explicit Content-Type header".to_string(),
                ));
            }
            Ok(content.into_bytes())
        }
        BodyLang::Form | BodyLang::Graphql => Err(ExecError::Config(format!(
            "`{}` bodies are not supported yet",
            lang_name(body.lang)
        ))),
    }
}

fn lang_name(lang: BodyLang) -> &'static str {
    match lang {
        BodyLang::Json => "json",
        BodyLang::Xml => "xml",
        BodyLang::Form => "form",
        BodyLang::Text => "text",
        BodyLang::Graphql => "graphql",
    }
}

fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

/// Insert or replace a header case-insensitively, preserving the original
/// spelling of an existing entry's name.
fn set_header(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    match headers
        .iter_mut()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
    {
        Some(slot) => slot.1 = value,
        None => headers.push((name.to_string(), value)),
    }
}

fn default_content_type(headers: &mut Vec<(String, String)>, value: &str) {
    if !has_header(headers, "content-type") {
        headers.push(("Content-Type".to_string(), value.to_string()));
    }
}
