//! Unit tests for the analysis modules, plus a real end-to-end test that drives
//! the server over an in-process `lsp-server` connection.

use line_index::LineIndex;
use lsp_types::DiagnosticSeverity;
use restmd_core::{Parsed, parse};

use crate::{completion::completion, convert, diagnostics::diagnostics, hover, symbols};

fn parsed(text: &str) -> Parsed {
    parse(text)
}

fn index(text: &str) -> LineIndex {
    LineIndex::new(text)
}

// --- completion ------------------------------------------------------------

#[test]
fn completes_variables_inside_braces() {
    let text = "## POST /a\n> capture token = $.t\n\n## GET /b/{{X";
    let items = completion(text, text.len(), &parsed(text));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"token"),
        "expected the captured var: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.starts_with("uuid")),
        "expected builtins"
    );
}

#[test]
fn completes_methods_in_heading() {
    let text = "## GE";
    let items = completion(text, text.len(), &parsed(text));
    assert!(items.iter().any(|i| i.label == "GET"));
    assert!(items.iter().any(|i| i.label == "POST"));
}

#[test]
fn completes_fence_languages() {
    let text = "## POST /a\n\n```js";
    let items = completion(text, text.len(), &parsed(text));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"json"));
    assert!(labels.contains(&"graphql"));
}

#[test]
fn completes_directive_keywords() {
    let text = "## GET /a\n> ca";
    let items = completion(text, text.len(), &parsed(text));
    assert!(items.iter().any(|i| i.label == "capture"));
}

#[test]
fn completes_header_names_inside_a_request() {
    let text = "## GET /a\nAcc";
    let items = completion(text, text.len(), &parsed(text));
    assert!(items.iter().any(|i| i.label == "Accept"));
}

#[test]
fn completes_frontmatter_keys() {
    let text = "---\nba";
    let items = completion(text, text.len(), &parsed(text));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"base"), "{labels:?}");
    assert!(labels.contains(&"environments"));
}

#[test]
fn completes_variables_in_a_templated_frontmatter_value() {
    let text = "---\nbase: https://api.{{";
    let items = completion(text, text.len(), &parsed(text));
    assert!(items.iter().any(|i| i.label.starts_with("env")));
}

#[test]
fn no_completion_in_plain_prose() {
    let text = "Some documentation here";
    let items = completion(text, text.len(), &parsed(text));
    assert!(items.is_empty());
}

// --- diagnostics -----------------------------------------------------------

#[test]
fn flags_unknown_variable() {
    let text = "## GET /a/{{missing}}\n";
    let p = parsed(text);
    let diags = diagnostics(&p, &index(text));
    assert!(
        diags
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::WARNING)
                && d.message.contains("missing"))
    );
}

#[test]
fn accepts_a_captured_variable() {
    let text = "## POST /a\n> capture token = $.t\n\n## GET /b/{{token}}\n";
    let p = parsed(text);
    let diags = diagnostics(&p, &index(text));
    assert!(!diags.iter().any(|d| d.message.contains("token")));
}

#[test]
fn flags_forward_reference() {
    let text = "## GET /a/{{token}}\n\n## POST /b\n> capture token = $.t\n";
    let p = parsed(text);
    let diags = diagnostics(&p, &index(text));
    assert!(diags.iter().any(|d| d.message.contains("used before")));
}

#[test]
fn surfaces_parse_errors_as_errors() {
    let text = "## GET\n"; // missing path
    let p = parsed(text);
    let diags = diagnostics(&p, &index(text));
    assert!(
        diags
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR))
    );
}

// --- symbols & hover -------------------------------------------------------

#[test]
fn one_symbol_per_request() {
    let text = "## GET /a\n## POST /b\n";
    let p = parsed(text);
    let syms = symbols::document_symbols(&p.document, text, &index(text));
    assert_eq!(syms.len(), 2);
    assert_eq!(syms[0].name, "GET /a");
    assert_eq!(syms[1].name, "POST /b");
}

#[test]
fn hover_explains_a_captured_variable() {
    let text = "## POST /a\n> capture token = $.t\n\n## GET /b/{{token}}\n";
    let p = parsed(text);
    let offset = text.rfind("token}}").unwrap();
    let h = hover::hover(&p.document, text, &index(text), offset).expect("hover");
    match h.contents {
        lsp_types::HoverContents::Markup(m) => assert!(m.value.contains("captured")),
        _ => panic!("expected markup"),
    }
}

// --- convert (UTF-16) ------------------------------------------------------

#[test]
fn offset_to_position_counts_utf16_units() {
    // `é` is 2 UTF-8 bytes but 1 UTF-16 unit.
    let text = "café = {{x}}\n";
    let li = index(text);
    let offset = text.find("{{").unwrap();
    let pos = convert::offset_to_position(&li, offset);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 7); // c a f é space = space  →  7 UTF-16 units
    assert_eq!(convert::position_to_offset(&li, pos), Some(offset)); // round-trips
}

// --- path filter -----------------------------------------------------------

#[test]
fn path_filter_scopes_to_restmd_dirs() {
    let yes: lsp_types::Uri = "file:///proj/.restmd/auth.md".parse().unwrap();
    let no: lsp_types::Uri = "file:///proj/notes.md".parse().unwrap();
    assert!(crate::server::is_restmd(&yes));
    assert!(!crate::server::is_restmd(&no));
}

// --- end-to-end over a real lsp-server connection --------------------------

mod e2e {
    use std::time::Duration;

    use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
    use lsp_types::{
        CompletionParams, CompletionResponse, DidOpenTextDocumentParams, DocumentSymbolParams,
        DocumentSymbolResponse, InitializeParams, Position, PublishDiagnosticsParams,
        TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Uri,
    };

    fn recv_response(client: &Connection) -> Response {
        loop {
            match client
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
            {
                Message::Response(r) => return r,
                _ => continue,
            }
        }
    }

    fn recv_notification(client: &Connection, method: &str) -> Notification {
        loop {
            match client
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
            {
                Message::Notification(n) if n.method == method => return n,
                _ => continue,
            }
        }
    }

    #[test]
    fn initialize_open_completion_symbols_diagnostics() {
        let (server, client) = Connection::memory();
        let server_thread = std::thread::spawn(move || crate::server::run(server).unwrap());

        // initialize handshake
        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(1),
                method: "initialize".to_string(),
                params: serde_json::to_value(InitializeParams::default()).unwrap(),
            }))
            .unwrap();
        assert_eq!(recv_response(&client).id, RequestId::from(1));
        client
            .sender
            .send(Message::Notification(Notification {
                method: "initialized".to_string(),
                params: serde_json::json!({}),
            }))
            .unwrap();

        // didOpen a document with a capture, a use, and a typo (under `.restmd/`
        // so the server's path filter activates it).
        let uri: Uri = "file:///proj/.restmd/test.md".parse().unwrap();
        let text = "## POST /login\n> capture token = $.t\n\n## GET /u/{{}}\nX: {{nope}}\n";
        client
            .sender
            .send(Message::Notification(Notification {
                method: "textDocument/didOpen".to_string(),
                params: serde_json::to_value(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "markdown".to_string(),
                        version: 1,
                        text: text.to_string(),
                    },
                })
                .unwrap(),
            }))
            .unwrap();

        // diagnostics: the unknown `nope` warning is published
        let note = recv_notification(&client, "textDocument/publishDiagnostics");
        let diags: PublishDiagnosticsParams = serde_json::from_value(note.params).unwrap();
        assert!(diags.diagnostics.iter().any(|d| d.message.contains("nope")));

        // completion inside `{{` on the GET line offers the captured `token`
        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_string(),
                params: serde_json::to_value(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(3, 12),
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                    context: None,
                })
                .unwrap(),
            }))
            .unwrap();
        let resp = recv_response(&client);
        let labels =
            match serde_json::from_value::<CompletionResponse>(resp.result.unwrap()).unwrap() {
                CompletionResponse::Array(items) => {
                    items.into_iter().map(|i| i.label).collect::<Vec<_>>()
                }
                _ => Vec::new(),
            };
        assert!(labels.contains(&"token".to_string()), "labels: {labels:?}");

        // documentSymbol returns both requests
        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(3),
                method: "textDocument/documentSymbol".to_string(),
                params: serde_json::to_value(DocumentSymbolParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                })
                .unwrap(),
            }))
            .unwrap();
        let resp = recv_response(&client);
        match serde_json::from_value::<DocumentSymbolResponse>(resp.result.unwrap()).unwrap() {
            DocumentSymbolResponse::Nested(syms) => assert_eq!(syms.len(), 2),
            DocumentSymbolResponse::Flat(syms) => assert_eq!(syms.len(), 2),
        }

        drop(client); // closes the connection; the server loop ends
        server_thread.join().unwrap();
    }
}
