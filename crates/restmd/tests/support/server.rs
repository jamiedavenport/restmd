//! A real, throwaway HTTP server for `restmd run` CLI tests.
//!
//! A copy of `restmd-core`'s executor test server: `tiny_http`, bound to an
//! ephemeral port on a background thread, recording every request and serving a
//! small fixed routing table. Shuts down on drop.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use tiny_http::{Header, Response, Server};

/// A request the server received.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    /// Path including any query string, e.g. `/data?x=1`.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    /// First header value matching `name`, case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub struct TestServer {
    pub base: String,
    server: Arc<Server>,
    received: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: Option<JoinHandle<()>>,
}

impl TestServer {
    pub fn start() -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").expect("bind test server"));
        let port = server.server_addr().to_ip().expect("ip addr").port();
        let base = format!("http://127.0.0.1:{port}");
        let received = Arc::new(Mutex::new(Vec::new()));

        let srv = Arc::clone(&server);
        let rec = Arc::clone(&received);
        let handle = std::thread::spawn(move || {
            for mut request in srv.incoming_requests() {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let headers = request
                    .headers()
                    .iter()
                    .map(|h| (format!("{}", h.field), format!("{}", h.value)))
                    .collect();
                let recorded = RecordedRequest {
                    method: request.method().as_str().to_string(),
                    path: request.url().to_string(),
                    headers,
                    body,
                };
                rec.lock().unwrap().push(recorded.clone());

                let (status, payload, content_type, response_headers) = route(&recorded);
                let mut response = Response::from_string(payload)
                    .with_status_code(status)
                    .with_header(Header::from_bytes(b"Content-Type", content_type).unwrap())
                    .with_header(Header::from_bytes(b"ETag", b"etag-xyz").unwrap());
                for (name, value) in response_headers {
                    response.add_header(Header::from_bytes(name, value).unwrap());
                }
                let _ = request.respond(response);
            }
        });

        Self {
            base,
            server,
            received,
            handle: Some(handle),
        }
    }

    /// Every request received so far, in order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.received.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

type Route = (
    u16,
    String,
    &'static [u8],
    Vec<(&'static [u8], &'static [u8])>,
);

/// The fixed routing table: `(status, body, content-type, extra headers)`.
fn route(request: &RecordedRequest) -> Route {
    let path = request.path.split('?').next().unwrap_or(&request.path);
    match (request.method.as_str(), path) {
        ("POST", "/auth/login") => (
            200,
            r#"{"access_token":"tok123","user":{"id":"u1"}}"#.to_string(),
            b"application/json",
            vec![],
        ),
        (_, "/data") => (
            200,
            r#"{"name":"Q4 Launch","count":5,"active":true,"items":[1,2,3],"email":"user@example.com"}"#
                .to_string(),
            b"application/json",
            vec![],
        ),
        (_, "/text") => (200, "just text".to_string(), b"text/plain", vec![]),
        (_, "/cookies/set") => (
            200,
            r#"{"ok":true}"#.to_string(),
            b"application/json",
            vec![(b"Set-Cookie", b"session=abc; Path=/")],
        ),
        (_, "/cookies/absent") => {
            let status = if request.header("Cookie").is_none() {
                200
            } else {
                409
            };
            (status, String::new(), b"application/json", vec![])
        }
        (_, p) if p.starts_with("/status/") => {
            let code = p.trim_start_matches("/status/").parse().unwrap_or(200);
            (code, String::new(), b"application/json", vec![])
        }
        _ => (
            200,
            r#"{"ok":true}"#.to_string(),
            b"application/json",
            vec![],
        ),
    }
}

/// A port guaranteed to refuse connections: bind, read the port, then drop the
/// listener so nothing is listening there.
pub fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
}
