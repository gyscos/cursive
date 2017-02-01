//! Toolbox to make text layout easier.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod lines_iterator;
mod reader;

pub use self::lines_iterator::{LinesIterator, Row};
pub use self::reader::ProgressReader;

/// Computes the length of a prefix that fits in the given `width`.
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
pub fn prefix_length<'a, I>(iter: I, available_width: usize, delimiter: &str) -> usize
    where I: Iterator<Item = &'a str>
{
    let delimiter_width = delimiter.width();
    let delimiter_len = delimiter.len();

    let mut current_width = 0;
    let sum = iter.take_while(|token| {
            let width = token.width();
            if current_width + width > available_width {
                false
            } else {
                if current_width != 0 {
                    current_width += delimiter_width;
                }
                current_width += width;
                true
            }
        })
        .map(|token| token.len() + delimiter_len)
        .fold(0, |a, b| a + b);

    // We counted delimiter once too many times,
    // but only if the iterator was non empty.
    if sum == 0 { sum } else { sum - delimiter_len }
}

/// Computes the length of a suffix that fits in the given `width`.
///
/// Doesn't break inside elements returned by `iter`.
///
/// Returns the number of bytes of the longest
/// suffix from `text` that fits in `width`.
///
/// This is a shortcut for `prefix_length(iter.rev(), width, delimiter)`
pub fn suffix_length<'a, I>(iter: I, width: usize, delimiter: &str) -> usize
    where I: DoubleEndedIterator<Item = &'a str>
{
    prefix_length(iter.rev(), width, delimiter)
}

/// Computes the length of a suffix that fits in the given `width`.
///
/// Breaks between any two graphemes.
pub fn simple_suffix_length(text: &str, width: usize) -> usize {
    suffix_length(text.graphemes(true), width, "")
}
