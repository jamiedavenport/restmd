//! The LSP protocol loop: initialize, then dispatch requests and notifications.
//!
//! Synchronous, built on `lsp-server`. Each handler is thin — it looks up the
//! document and calls into the pure analysis modules.

use anyhow::Result;
use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, DocumentSymbolRequest, HoverRequest};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DocumentSymbolParams,
    DocumentSymbolResponse, Hover, HoverParams, HoverProviderCapability, OneOf,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    Uri,
};
use serde::de::DeserializeOwned;

use crate::convert::position_to_offset;
use crate::documents::Store;
use crate::{completion, diagnostics, hover, symbols};

/// The capabilities we advertise.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            // `{` for variables, space for `## `/`> `, backtick for body fences.
            trigger_characters: Some(vec!["{".to_string(), " ".to_string(), "`".to_string()]),
            ..Default::default()
        }),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    }
}

/// Run the initialize handshake and then the main loop until shutdown.
pub fn run(connection: Connection) -> Result<()> {
    let capabilities = serde_json::to_value(server_capabilities())?;
    connection.initialize(capabilities)?;
    main_loop(&connection)?;
    Ok(())
}

fn main_loop(connection: &Connection) -> Result<()> {
    let mut store = Store::default();
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                handle_request(connection, &store, request)?;
            }
            Message::Notification(note) => handle_notification(connection, &mut store, note)?,
            Message::Response(_) => {}
        }
    }
    Ok(())
}

// --- requests --------------------------------------------------------------

fn handle_request(connection: &Connection, store: &Store, request: Request) -> Result<()> {
    let request = match cast_req::<Completion>(request) {
        Ok((id, params)) => return respond(connection, id, completion(store, params)),
        Err(ExtractError::MethodMismatch(req)) => req,
        Err(ExtractError::JsonError { .. }) => return Ok(()),
    };
    let request = match cast_req::<DocumentSymbolRequest>(request) {
        Ok((id, params)) => return respond(connection, id, document_symbol(store, params)),
        Err(ExtractError::MethodMismatch(req)) => req,
        Err(ExtractError::JsonError { .. }) => return Ok(()),
    };
    match cast_req::<HoverRequest>(request) {
        Ok((id, params)) => respond(connection, id, hover_at(store, params)),
        Err(_) => Ok(()),
    }
}

fn completion(store: &Store, params: CompletionParams) -> Option<CompletionResponse> {
    let pos = params.text_document_position;
    if !is_restmd(&pos.text_document.uri) {
        return None;
    }
    let doc = store.get(&pos.text_document.uri)?;
    let offset = position_to_offset(&doc.index, pos.position)?;
    Some(CompletionResponse::Array(completion::completion(
        &doc.text,
        offset,
        &doc.parsed,
    )))
}

fn document_symbol(store: &Store, params: DocumentSymbolParams) -> Option<DocumentSymbolResponse> {
    if !is_restmd(&params.text_document.uri) {
        return None;
    }
    let doc = store.get(&params.text_document.uri)?;
    let symbols = symbols::document_symbols(&doc.parsed.document, &doc.text, &doc.index);
    Some(DocumentSymbolResponse::Nested(symbols))
}

fn hover_at(store: &Store, params: HoverParams) -> Option<Hover> {
    let pos = params.text_document_position_params;
    if !is_restmd(&pos.text_document.uri) {
        return None;
    }
    let doc = store.get(&pos.text_document.uri)?;
    let offset = position_to_offset(&doc.index, pos.position)?;
    hover::hover(&doc.parsed.document, &doc.text, &doc.index, offset)
}

// --- notifications ---------------------------------------------------------

fn handle_notification(
    connection: &Connection,
    store: &mut Store,
    note: Notification,
) -> Result<()> {
    let note = match cast_note::<DidOpenTextDocument>(note) {
        Ok(params) => {
            let uri = params.text_document.uri;
            store.set(uri.clone(), params.text_document.text);
            return publish(connection, store, &uri);
        }
        Err(ExtractError::MethodMismatch(n)) => n,
        Err(ExtractError::JsonError { .. }) => return Ok(()),
    };
    let note = match cast_note::<DidChangeTextDocument>(note) {
        Ok(params) => {
            let uri = params.text_document.uri;
            // Full sync: the last change carries the whole document.
            if let Some(change) = params.content_changes.into_iter().next_back() {
                store.set(uri.clone(), change.text);
            }
            return publish(connection, store, &uri);
        }
        Err(ExtractError::MethodMismatch(n)) => n,
        Err(ExtractError::JsonError { .. }) => return Ok(()),
    };
    match cast_note::<DidCloseTextDocument>(note) {
        Ok(params) => {
            store.remove(&params.text_document.uri);
            // Clear diagnostics for the closed file.
            publish_diagnostics(connection, params.text_document.uri, Vec::new())
        }
        Err(_) => Ok(()),
    }
}

fn publish(connection: &Connection, store: &Store, uri: &Uri) -> Result<()> {
    let Some(doc) = store.get(uri) else {
        return Ok(());
    };
    // Only restmd files get diagnostics; others get an empty set (clears any).
    let diags = if is_restmd(uri) {
        diagnostics::diagnostics(&doc.parsed, &doc.index)
    } else {
        Vec::new()
    };
    publish_diagnostics(connection, uri.clone(), diags)
}

/// Whether a document is a restmd request file — i.e. lives under a `.restmd/`
/// directory. Editors that attach servers per *language* (Zed, Helix) run this
/// on all markdown; self-scoping keeps it inert outside `.restmd/`.
pub(crate) fn is_restmd(uri: &Uri) -> bool {
    uri.as_str().contains("/.restmd/")
}

fn publish_diagnostics(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<()> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    let note = Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: serde_json::to_value(params)?,
    };
    connection.sender.send(Message::Notification(note))?;
    Ok(())
}

// --- helpers ---------------------------------------------------------------

fn respond<T: serde::Serialize>(connection: &Connection, id: RequestId, result: T) -> Result<()> {
    let response = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    connection.sender.send(Message::Response(response))?;
    Ok(())
}

fn cast_req<R>(request: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: DeserializeOwned,
{
    request.extract(R::METHOD)
}

fn cast_note<N>(note: Notification) -> Result<N::Params, ExtractError<Notification>>
where
    N: lsp_types::notification::Notification,
    N::Params: DeserializeOwned,
{
    note.extract(N::METHOD)
}
