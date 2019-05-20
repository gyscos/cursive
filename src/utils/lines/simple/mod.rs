//! Compute lines on simple text.
//!
//! The input is a single `&str`.
//!
//! Computed rows will include start/end byte offsets in the input string.

mod lines_iterator;
mod row;

pub use self::lines_iterator::LinesIterator;
pub use self::row::Row;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The length and width of a part of a string.
pub struct Span {
    /// The length (in bytes) of the string.
    pub length: usize,
    /// The unicode-width of the string.
    pub width: usize,
}

/// Compute lines for the given content and width.
///
/// Equivalent to constructing a new `LinesIterator` and collecting it.
pub fn make_lines(content: &str, width: usize) -> Vec<Row> {
    LinesIterator::new(content, width).collect()
}

/// Computes a prefix that fits in the given `width`.
///
/// Takes non-breakable elements from `iter`, while keeping the string width
/// under `width` (and adding `delimiter` between each element).
///
/// Given `total_text = iter.collect().join(delimiter)`, the result is the
/// length of the longest prefix of `width` or less cells, without breaking
/// inside an element.
///
/// Example:
///
/// ```
/// use unicode_segmentation::UnicodeSegmentation;
///
/// # use cursive::utils::lines::simple::prefix;
/// # fn main() {
/// let my_text = "blah...";
/// // This returns the number of bytes for a prefix of `my_text` that
/// // fits within 5 cells.
/// prefix(my_text.graphemes(true), 5, "");
/// # }
/// ```
pub fn prefix<'a, I>(iter: I, available_width: usize, delimiter: &str) -> Span
where
    I: Iterator<Item = &'a str>,
{
    let delimiter_width = delimiter.width();
    let delimiter_len = delimiter.len();

    // `current_width` is the width of everything
    // before the next token, including any space.
    let mut current_width = 0;
    let sum: usize = iter
        .take_while(|token| {
            let width = token.width();
            if current_width + width > available_width {
                false
            } else {
                // Include the delimiter after this token.
                current_width += width;
                current_width += delimiter_width;
                true
            }
        })
        .map(|token| token.len() + delimiter_len)
        .sum();

    // We counted delimiter once too many times,
    // but only if the iterator was non empty.
    let length = sum.saturating_sub(delimiter_len);

    // `current_width` includes a delimiter after the last token
    debug_assert!(current_width <= available_width + delimiter_width);

    Span {
        length,
        width: current_width,
    }
}

/// Computes the longest suffix that fits in the given `width`.
///
/// Doesn't break inside elements returned by `iter`.
///
/// Returns the number of bytes of the longest
/// suffix from `text` that fits in `width`.
///
/// This is a shortcut for `prefix_length(iter.rev(), width, delimiter)`
pub fn suffix<'a, I>(iter: I, width: usize, delimiter: &str) -> Span
where
    I: DoubleEndedIterator<Item = &'a str>,
{
    prefix(iter.rev(), width, delimiter)
}

/// Computes the longest suffix that fits in the given `width`.
///
/// Breaks between any two graphemes.
pub fn simple_suffix(text: &str, width: usize) -> Span {
    suffix(text.graphemes(true), width, "")
}

/// Computes the longest prefix that fits in the given width.
///
/// Breaks between any two graphemes.
pub fn simple_prefix(text: &str, width: usize) -> Span {
    prefix(text.graphemes(true), width, "")
}

#[cfg(test)]
mod tests;
