use super::Row;
use crate::utils::lines::spans;
use crate::utils::span::{IndexedSpan, SpannedText};

/// Generates rows of text in constrained width.
///
/// Given a long text and a width constraint, it iterates over
/// substrings of the text, each within the constraint.
pub struct LinesIterator<'a> {
    iter: spans::LinesIterator<DummySpannedText<'a>>,

    /// Available width. Don't output lines wider than that.
    width: usize,
}

struct DummySpannedText<'a> {
    content: &'a str,
    attrs: Vec<IndexedSpan<()>>,
}

impl<'a> DummySpannedText<'a> {
    fn new(content: &'a str) -> Self {
        let attrs = vec![IndexedSpan::simple_borrowed(content, ())];
        DummySpannedText { content, attrs }
    }
}

impl<'a> SpannedText for DummySpannedText<'a> {
    type S = IndexedSpan<()>;

    fn source(&self) -> &str {
        self.content
    }

    fn spans(&self) -> &[IndexedSpan<()>] {
        &self.attrs
    }
}

impl<'a> LinesIterator<'a> {
    /// Returns a new `LinesIterator` on `content`.
    ///
    /// Yields rows of `width` cells or less.
    pub fn new(content: &'a str, width: usize) -> Self {
        let iter = spans::LinesIterator::new(DummySpannedText::new(content), width);
        LinesIterator { iter, width }
    }

    /// Leave a blank cell at the end of lines.
    ///
    /// Unless a word had to be truncated, in which case
    /// it takes the entire width.
    #[must_use]
    pub fn show_spaces(self) -> Self {
        let iter = self.iter.show_spaces();
        let width = self.width;
        LinesIterator { iter, width }
    }
}

impl<'a> Iterator for LinesIterator<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        let row = self.iter.next()?;

        let start = row.segments.first()?.start;
        let end = row.segments.last()?.end;

        let spans::Row {
            width, is_wrapped, ..
        } = row;

        Some(Row {
            start,
            end,
            width,
            is_wrapped,
        })
    }
}
