use super::Segment;
use utils::span::{AsSpannedStr, IndexedCow, Span};

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
        S: 'a + AsSpannedStr<'a, T>,
    {
        let source = source.as_spanned_str();

        self.segments
            .iter()
            .map(|seg| seg.resolve(source.clone()))
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
