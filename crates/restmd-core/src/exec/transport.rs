//! The HTTP transport seam.
//!
//! The executor talks to the network only through [`HttpTransport`], so its
//! logic (URL building, header merge, body serialization, captures, assertions)
//! can be driven against a real local server in tests while staying decoupled
//! from `reqwest`. [`ReqwestTransport`] is the production implementation.

use crate::model::Method;

/// A fully-resolved request ready to send.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

/// A received response.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// First header value matching `name`, case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// A transport-level failure (connection refused, timeout, TLS, …) — distinct
/// from an HTTP error status, which is a normal response.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct TransportError(pub String);

/// Sends an [`HttpRequest`] and returns the [`HttpResponse`].
pub trait HttpTransport {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Production transport backed by a blocking `reqwest` client.
///
/// Each transport owns an in-memory cookie store. Reuse one transport for the
/// requests in a single session, and create a new transport for an independent
/// document run.
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .cookie_store(true)
                .build()
                .expect("ReqwestTransport client"),
        }
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let method = reqwest::Method::from_bytes(req.method.as_str().as_bytes())
            .map_err(|e| TransportError(e.to_string()))?;
        let mut builder = self.client.request(method, &req.url);
        for (name, value) in &req.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = &req.body {
            builder = builder.body(body.clone());
        }

        let resp = builder.send().map_err(|e| TransportError(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .bytes()
            .map_err(|e| TransportError(e.to_string()))?
            .to_vec();

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
