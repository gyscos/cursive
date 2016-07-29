//! Toolbox to make text layout easier.

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

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

/// Computes a sub-string length that fits in the given `width`.
///
/// Takes non-breakable elements from `iter`, while keeping the
/// string width under `width` (and adding the length of `delimiter`
/// between each element).
///
/// Given `total_text = iter.collect().join(delimiter)`, the result
/// is the length of the longest prefix of `width` or less cells,
/// without breaking inside an element.
///
/// Example:
///
/// ```
/// # extern crate cursive;
/// extern crate unicode_segmentation;
/// use unicode_segmentation::UnicodeSegmentation;
///
/// # use cursive::utils::prefix_length;
/// # fn main() {
/// let my_text = "blah...";
/// // This returns the number of bytes for a prefix of `my_text` that
/// // fits within 5 cells.
/// prefix_length(my_text.graphemes(true), 5, "");
/// # }
/// ```
pub fn prefix_length<'a, I: Iterator<Item = &'a str>>(iter: I, width: usize,
                                               delimiter: &str)
                                               -> usize {
    let delimiter_width = delimiter.width();
    let delimiter_len = delimiter.len();

    let sum = iter.scan(0, |w, token| {
            *w += token.width();
            if *w > width {
                None
            } else {
                // Add a space
                *w += delimiter_width;
                Some(token)
            }
        })
        .map(|token| token.len() + delimiter_len)
        .fold(0, |a, b| a + b);

    // We counted delimiter once too many times,
    // but only if the iterator was non empty.
    if sum == 0 {
        sum
    } else {
        sum - delimiter_len
    }
}
