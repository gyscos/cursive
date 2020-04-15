use crate::utils::span::{IndexedCow, Span, SpannedStr, SpannedText};

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
    /// Resolve this segment to a string slice and an attribute.
    pub fn resolve<'a, T>(&self, source: &SpannedStr<'a, T>) -> Span<'a, T> {
        let span = &source.spans_raw()[self.span_id];

        let content = span.content.resolve(source.source());
        let content = &content[self.start..self.end];

        Span {
            content,
            attr: &span.attr,
            width: self.width,
        }
    }

    /// Resolves this segment to plain text.
    pub fn resolve_plain<'a, S>(&self, source: &'a S) -> &'a str
    where
        S: SpannedText,
    {
        let span = &source.spans()[self.span_id];

        let content = span.as_ref().resolve(source.source());

        &content[self.start..self.end]
    }

    /// Returns indices in the source string, if possible.
    ///
    /// Returns `(start, end)`, or `None` if the target span is an owned string.
    pub fn source_indices<S>(&self, spans: &[S]) -> Option<(usize, usize)>
    where
        S: AsRef<IndexedCow>,
    {
        let span = spans[self.span_id].as_ref();

        if let IndexedCow::Borrowed { start, .. } = *span {
            Some((self.start + start, self.end + start))
        } else {
            None
        }
    }
}
