use super::Segment;
use utils::span::{Span, SpannedStr};

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
    pub fn resolve<'a, T>(
        &self, source: SpannedStr<'a, T>
    ) -> Vec<Span<'a, T>> {
        self.segments
            .iter()
            .map(|seg| seg.resolve(source.clone()))
            .collect()
    }
}
