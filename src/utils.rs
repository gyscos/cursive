use unicode_width::UnicodeWidthStr;

// Computes a sub-string length that fits in the given `width`.
//
// Takes non-breakable elements from `iter`, while keeping the
// string width under `width` (and adding the length of `delimiter`
// between each element).
//
// Example:
//
// ```
// let my_text = "blah...";
// // This returns the number of bytes for a prefix of `my_text` that
// // fits within 5 cells.
// head_bytes(my_text.graphemes(true), 5, "");
// ```
pub fn head_bytes<'a, I: Iterator<Item = &'a str>>(iter: I, width: usize,
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
