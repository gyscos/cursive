use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

use utils::prefix_length;

/// Generates rows of text in constrained width.
///
/// Given a long text and a width constraint, it iterates over
/// substrings of the text, each within the constraint.
pub struct LinesIterator<'a> {
    content: &'a str,
    start: usize,
    width: usize,
}

impl<'a> LinesIterator<'a> {
    /// Returns a new `LinesIterator` on `content`.
    ///
    /// Yields rows of `width` cells or less.
    pub fn new(content: &'a str, width: usize) -> Self {
        LinesIterator {
            content: content,
            width: width,
            start: 0,
        }
    }
}

/// Represents a row of text within a `String`.
///
/// A row is made of an offset into a parent `String` and a length.
/// The corresponding substring should take `width` cells when printed.
pub struct Row {
    /// Beginning of the row in the parent `String`.
    pub start: usize,
    /// Length of the row, in bytes.
    pub end: usize,
    /// Width of the row, in cells.
    pub width: usize,
}

impl<'a> Iterator for LinesIterator<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.start >= self.content.len() {
            // This is the end.
            return None;
        }

        let start = self.start;
        let content = &self.content[self.start..];

        let next = content.find('\n').unwrap_or(content.len());
        let content = &content[..next];

        let line_width = content.width();
        if line_width <= self.width {
            // We found a newline before the allowed limit.
            // Break early.
            self.start += next + 1;
            return Some(Row {
                start: start,
                end: next + start,
                width: line_width,
            });
        }

        // Keep adding indivisible tokens
        let prefix_length =
            match prefix_length(content.split(' '), self.width, " ") {
                0 => prefix_length(content.graphemes(true), self.width, ""),
                other => {
                    self.start += 1;
                    other
                }
            };

        if prefix_length == 0 {
            // This mean we can't even get a single char?
            // Sucks. Let's bail.
            return None;
        }

        self.start += prefix_length;

        Some(Row {
            start: start,
            end: start + prefix_length,
            width: self.width,
        })
    }
}
