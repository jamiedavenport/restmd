//! A tiny development HTTP server for exercising the TUI (and the e2e test).
//!
//! It serves a small fixed set of endpoints the demo `.restmd` files target.
//! [`serve`] blocks (used by the `restmd-devserver` binary); [`spawn`] runs it
//! on an ephemeral port for tests.

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::thread::JoinHandle;

use tiny_http::{Header, Request, Response, Server};

const JSON: &[u8] = b"application/json";

/// Serve forever on `addr` (blocking).
pub fn serve<A: ToSocketAddrs>(addr: A) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;
    for request in server.incoming_requests() {
        respond(request);
    }
    Ok(())
}

fn respond(request: Request) {
    let (status, body, content_type) = route(request.method().as_str(), request.url());
    let response = Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes(b"Content-Type", content_type).unwrap());
    let _ = request.respond(response);
}

/// The fixed routing table: `(status, body, content-type)`.
fn route(method: &str, url: &str) -> (u16, String, &'static [u8]) {
    let path = url.split('?').next().unwrap_or(url);
    match (method, path) {
        ("POST", "/auth/login") => (
            200,
            r#"{"access_token":"demo-token-abc123","user":{"id":"u-42","name":"Ada Lovelace"}}"#
                .to_string(),
            JSON,
        ),
        ("GET", "/users") => (
            200,
            r#"[{"id":"u-1","name":"Ada"},{"id":"u-2","name":"Linus"}]"#.to_string(),
            JSON,
        ),
        ("POST", "/users") => (
            201,
            r#"{"id":"u-99","name":"Grace","active":true}"#.to_string(),
            JSON,
        ),
        ("GET", p) if p.starts_with("/users/") => {
            let id = p.trim_start_matches("/users/");
            (
                200,
                format!(r#"{{"id":"{id}","name":"User {id}","active":true,"score":7}}"#),
                JSON,
            )
        }
        (_, p) if p.starts_with("/status/") => {
            let code = p.trim_start_matches("/status/").parse().unwrap_or(200);
            (code, String::new(), JSON)
        }
        _ => (200, r#"{"ok":true}"#.to_string(), JSON),
    }
}

/// A running dev server on an ephemeral port, for tests. Shuts down on drop.
pub struct DevServer {
    pub base: String,
    server: Arc<Server>,
    handle: Option<JoinHandle<()>>,
}

/// Start the dev server on `127.0.0.1:0` (an OS-assigned port).
pub fn spawn() -> std::io::Result<DevServer> {
    let server = Arc::new(Server::http("127.0.0.1:0").map_err(std::io::Error::other)?);
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(0);
    let base = format!("http://127.0.0.1:{port}");

    let srv = Arc::clone(&server);
    let handle = std::thread::spawn(move || {
        for request in srv.incoming_requests() {
            respond(request);
        }
    });

    Ok(DevServer {
        base,
        server,
        handle: Some(handle),
    })
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
