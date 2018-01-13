use utils::span::{Span, SpannedString};

/// Refers to a part of a span
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// ID of the span this segment refers to
    pub span_id: usize,

    /// Beginning of this segment within the span (included)
    pub start: usize,
    /// End of this segment within the span (excluded)
    pub end: usize,

    /// Width of this segment
    pub width: usize,
}

impl Segment {
    #[cfg(test)]
    pub fn with_text<'a>(self, text: &'a str) -> SegmentWithText<'a> {
        SegmentWithText { text, seg: self }
    }

    /// Resolve this segment to a string slice and an attribute.
    pub fn resolve<'a, T>(&self, source: &'a SpannedString<T>) -> Span<'a, T> {
        let span = &source.spans_raw()[self.span_id];

        let content = span.content.resolve(source.source());
        let content = &content[self.start..self.end];

        Span {
            content,
            attr: &span.attr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentWithText<'a> {
    pub seg: Segment,
    pub text: &'a str,
}
