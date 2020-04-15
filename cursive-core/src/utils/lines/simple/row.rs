use crate::With;

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
