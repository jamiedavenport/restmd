//! `restmd-lsp` — a language server for `.restmd` request files.
//!
//! Provides completion, diagnostics, document symbols, and hover by parsing
//! files with `restmd-core` and analyzing the result. It does not execute
//! requests, so it needs no network and no async runtime.
//!
//! The analysis modules ([`completion`], [`diagnostics`], [`symbols`],
//! [`hover`], [`analysis`]) are pure functions over a parsed document, which
//! keeps them unit-testable independently of the protocol loop in [`server`].

pub mod analysis;
pub mod completion;
pub mod convert;
pub mod diagnostics;
pub mod documents;
pub mod hover;
pub mod server;
pub mod symbols;

#[cfg(test)]
mod tests;
