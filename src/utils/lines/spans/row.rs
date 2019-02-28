use super::Segment;
use crate::utils::span::{IndexedCow, Span, SpannedStr};

/// A list of segments representing a row of text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// List of segments
    pub segments: Vec<Segment>,
    /// Total width for this row
    pub width: usize,
}

impl Row {
    /// Resolve the row indices into string slices and attributes.
    pub fn resolve<'a, T, S>(&self, source: S) -> Vec<Span<'a, T>>
    where
        S: Into<SpannedStr<'a, T>>,
    {
        let source = source.into();

        self.segments
            .iter()
            .map(|seg| seg.resolve(&source))
            .filter(|span| !span.content.is_empty())
            .collect()
    }

    /// Returns indices in the source string, if possible.
    ///
    /// Returns overall `(start, end)`, or `None` if the segments are owned.
    pub fn overall_indices<S>(&self, spans: &[S]) -> Option<(usize, usize)>
    where
        S: AsRef<IndexedCow>,
    {
        let (start, _) = self.segments.first()?.source_indices(spans)?;
        let (_, end) = self.segments.last()?.source_indices(spans)?;

        Some((start, end))
    }
}
