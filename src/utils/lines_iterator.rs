

use With;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use utils::prefix_length;

/// Generates rows of text in constrained width.
///
/// Given a long text and a width constraint, it iterates over
/// substrings of the text, each within the constraint.
pub struct LinesIterator<'a> {
    /// Content to iterate on.
    content: &'a str,
    /// Current offset in the content.
    offset: usize,
    /// Available width. Don't output lines wider than that.
    width: usize,

    /// If `true`, keep a blank cell at the end of lines
    /// when a whitespace or newline should be.
    show_spaces: bool,
}

impl<'a> LinesIterator<'a> {
    /// Returns a new `LinesIterator` on `content`.
    ///
    /// Yields rows of `width` cells or less.
    pub fn new(content: &'a str, width: usize) -> Self {
        LinesIterator {
            content: content,
            width: width,
            offset: 0,
            show_spaces: false,
        }
    }

    /// Leave a blank cell at the end of lines.
    ///
    /// Unless a word had to be truncated, in which case
    /// it takes the entire width.
    pub fn show_spaces(mut self) -> Self {
        self.show_spaces = true;
        self
    }
}

/// Represents a row of text within a `String`.
///
/// A row is made of offsets into a parent `String`.
/// The corresponding substring should take `width` cells when printed.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// Beginning of the row in the parent `String`.
    pub start: usize,
    /// End of the row (excluded)
    pub end: usize,
    /// Width of the row, in cells.
    pub width: usize,
}

impl Row {
    /// Shift a row start and end by `offset`.
    pub fn shift(&mut self, offset: usize) {
        self.start += offset;
        self.end += offset;
    }

    /// Shift a row start and end by `offset`.
    ///
    /// Chainable variant;
    pub fn shifted(self, offset: usize) -> Self {
        self.with(|s| s.shift(offset))
    }

    /// Shift back a row start and end by `offset`.
    pub fn rev_shift(&mut self, offset: usize) {
        self.start -= offset;
        self.end -= offset;
    }
}

impl<'a> Iterator for LinesIterator<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.offset >= self.content.len() {
            // This is the end.
            return None;
        }

        // We start at the current offset.
        let start = self.offset;
        let content = &self.content[start..];

        // Find the ideal line, in an infinitely wide world.
        // We'll make a line larger than that.
        let next = content.find('\n').unwrap_or(content.len());
        let content = &content[..next];

        let allowed_width = if self.show_spaces {
            self.width - 1
        } else {
            self.width
        };

        let line_width = content.width();
        if line_width <= allowed_width {
            // We found a newline before the allowed limit.
            // Break early.
            // Advance the cursor to after the newline.
            self.offset += next + 1;
            return Some(Row {
                start: start,
                end: start + next,
                width: line_width,
            });
        }

        // First attempt: only break on spaces.
        let prefix_length =
            match prefix_length(content.split(' '), allowed_width, " ") {
                // If this fail, fallback: only break on graphemes.
                // There's no whitespace to skip there.
                // And don't reserve the white space anymore.
                0 => prefix_length(content.graphemes(true), self.width, ""),
                other => {
                    // If it works, advance the cursor by 1
                    // to jump the whitespace.
                    // We don't want to add 1 to `prefix_length` though, it
                    // would include the whitespace in the row.
                    self.offset += 1;
                    other
                }
            };

        if prefix_length == 0 {
            // This mean we can't even get a single char?
            // Sucks. Let's bail.
            return None;
        }

        // Advance the offset to the end of the line.
        self.offset += prefix_length;

        Some(Row {
            start: start,
            end: start + prefix_length,
            width: self.width,
        })
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_layout() {}
}
