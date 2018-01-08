//! Compute lines on multiple spans of text.
//!
//! The input is a list of consecutive text spans.
//!
//! Computed rows will include a list of span segments.
//! Each segment include the source span ID, and start/end byte offsets.
mod lines_iterator;
mod chunk_iterator;
mod segment_merge_iterator;
mod row;
mod prefix;
mod chunk;
mod segment;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use theme::Style;

pub use self::lines_iterator::SpanLinesIterator;
pub use self::row::Row;
pub use self::segment::Segment;

/// Input to the algorithm
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a> {
    /// Text for this span.
    ///
    /// It can be either a reference to some input text,
    /// or an owned string.
    ///
    /// The owned string is mostly useful when parsing marked-up text that
    /// contains escape codes.
    pub text: Cow<'a, str>,

    /// Style to apply to this span of text.
    pub style: Style,
}

