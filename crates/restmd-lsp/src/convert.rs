//! Converting between restmd byte [`Span`]s and LSP [`Range`]s.
//!
//! LSP positions are `(line, character)` where `character` counts **UTF-16 code
//! units**, not bytes or scalars. `line-index` does that conversion correctly,
//! so we route through it rather than hand-rolling.

use line_index::{LineIndex, WideEncoding, WideLineCol};
use lsp_types::{Position, Range};
use restmd_core::Span;
use text_size::TextSize;

/// Byte offset → LSP position (UTF-16).
pub fn offset_to_position(index: &LineIndex, offset: usize) -> Position {
    let line_col = index.line_col(TextSize::from(offset as u32));
    let wide = index
        .to_wide(WideEncoding::Utf16, line_col)
        .unwrap_or(WideLineCol {
            line: line_col.line,
            col: line_col.col,
        });
    Position::new(wide.line, wide.col)
}

/// Byte [`Span`] → LSP [`Range`].
pub fn span_to_range(index: &LineIndex, span: Span) -> Range {
    Range::new(
        offset_to_position(index, span.start),
        offset_to_position(index, span.end),
    )
}

/// LSP position (UTF-16) → byte offset, or `None` if out of bounds.
pub fn position_to_offset(index: &LineIndex, position: Position) -> Option<usize> {
    let wide = WideLineCol {
        line: position.line,
        col: position.character,
    };
    let line_col = index.to_utf8(WideEncoding::Utf16, wide)?;
    index.offset(line_col).map(usize::from)
}
