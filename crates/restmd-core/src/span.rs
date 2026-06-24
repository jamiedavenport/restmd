//! Source spans.
//!
//! Every node the parser produces carries a [`Span`] — a byte range into the
//! original source. Spans are the foundation for diagnostics (the LSP needs to
//! point at exactly the offending bytes) and must therefore be tracked from the
//! very first parse, not bolted on later.

/// A half-open byte range `[start, end)` into the source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span start must not exceed end");
        Self { start, end }
    }

    /// An empty span at a single offset — used for constructs that are missing
    /// from the source (e.g. a request heading with no path).
    pub const fn empty(at: usize) -> Self {
        Self { start: at, end: at }
    }

    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The slice of `src` this span refers to.
    ///
    /// Panics in debug builds if the span is out of bounds or splits a UTF-8
    /// boundary — both indicate a parser bug, not user error.
    pub fn slice<'src>(&self, src: &'src str) -> &'src str {
        &src[self.start..self.end]
    }

    /// 1-based `(line, column)` of this span's start, computed against `src`.
    /// Column counts Unicode scalar values, not bytes.
    pub fn line_col(&self, src: &str) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (offset, ch) in src.char_indices() {
            if offset >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}
