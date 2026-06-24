//! Extracting `capture` values from a response.

use serde_json::Value as Json;
use serde_json_path::JsonPath;

use super::transport::HttpResponse;
use crate::model::CaptureSource;

/// Extract the value for one capture. Returns `Err(reason)` if the source could
/// not be read (e.g. a JSONPath with no match, or a missing header).
pub(crate) fn apply_capture(
    source: &CaptureSource,
    response: &HttpResponse,
) -> Result<String, String> {
    match source {
        CaptureSource::Status => Ok(response.status.to_string()),
        CaptureSource::Header(name) => response
            .header(name)
            .map(str::to_string)
            .ok_or_else(|| format!("response has no `{name}` header")),
        CaptureSource::JsonPath(path) => capture_jsonpath(path, response),
    }
}

fn capture_jsonpath(path: &str, response: &HttpResponse) -> Result<String, String> {
    let json: Json = serde_json::from_slice(&response.body)
        .map_err(|_| "response body is not JSON".to_string())?;
    let query = JsonPath::parse(path).map_err(|e| format!("invalid JSONPath `{path}`: {e}"))?;
    let node = query
        .query(&json)
        .first()
        .ok_or_else(|| format!("JSONPath `{path}` matched nothing"))?;
    Ok(stringify(node))
}

/// Stringify a JSON node for storage as a variable: strings keep their raw
/// value; everything else is its compact JSON form.
pub(super) fn stringify(node: &Json) -> String {
    match node {
        Json::String(s) => s.clone(),
        other => other.to_string(),
    }
}
