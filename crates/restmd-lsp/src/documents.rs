//! The open-document store: text + line index + parse result, keyed by URI.

use std::collections::HashMap;

use line_index::LineIndex;
use lsp_types::Uri;
use restmd_core::{Parsed, parse};

/// One open document.
pub struct Doc {
    pub text: String,
    pub index: LineIndex,
    pub parsed: Parsed,
}

impl Doc {
    fn new(text: String) -> Self {
        let index = LineIndex::new(&text);
        let parsed = parse(&text);
        Self {
            text,
            index,
            parsed,
        }
    }
}

/// All open documents.
#[derive(Default)]
pub struct Store {
    docs: HashMap<Uri, Doc>,
}

impl Store {
    /// Insert or replace a document's text, re-parsing it.
    pub fn set(&mut self, uri: Uri, text: String) {
        self.docs.insert(uri, Doc::new(text));
    }

    pub fn remove(&mut self, uri: &Uri) {
        self.docs.remove(uri);
    }

    pub fn get(&self, uri: &Uri) -> Option<&Doc> {
        self.docs.get(uri)
    }
}
