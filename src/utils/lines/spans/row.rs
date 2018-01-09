use super::{Segment, Span};
use std::borrow::Cow;

/// A list of segments representing a row of text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// List of segments
    pub segments: Vec<Segment>,
    /// Total width for this row
    pub width: usize,
}

impl Row {
    /// Resolve the row indices into styled spans.
    pub fn resolve<'a: 'b, 'b>(&self, spans: &'b [Span<'a>]) -> Vec<Span<'b>> {
        self.segments
            .iter()
            .map(|seg| {
                let span: &'b Span<'a> = &spans[seg.span_id];
                let text: &'b str = &span.text;
                let text: &'b str = &text[seg.start..seg.end];

                Span {
                    text: Cow::Borrowed(text),
                    style: span.style,
                }
            })
            .collect()
    }
}
