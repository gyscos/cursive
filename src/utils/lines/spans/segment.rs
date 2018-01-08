
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
    fn with_text<'a>(self, text: &'a str) -> SegmentWithText<'a> {
        SegmentWithText { text, seg: self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentWithText<'a> {
    pub seg: Segment,
    pub text: &'a str,
}
