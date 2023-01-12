use std::cmp::min;

/// Single-dimensional constraint on a view size.
///
/// This describes a possible behaviour for a [`ResizedView`].
///
/// [`ResizedView`]: crate::views::ResizedView
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeConstraint {
    /// No constraint imposed, the child view's response is used.
    Free,
    /// Tries to take all available space, no matter what the child needs.
    Full,
    /// Always return the included size, no matter what the child needs.
    Fixed(usize),
    /// Returns the minimum of the included value and the child view's size.
    AtMost(usize),
    /// Returns the maximum of the included value and the child view's size.
    AtLeast(usize),
}

impl SizeConstraint {
    /// Returns the size to be given to the child.
    ///
    /// When `available` is offered to the `ResizedView`.
    pub fn available(self, available: usize) -> usize {
        match self {
            SizeConstraint::Free | SizeConstraint::Full | SizeConstraint::AtLeast(_) => available,
            // If the available space is too small, always give in.
            SizeConstraint::Fixed(value) | SizeConstraint::AtMost(value) => min(value, available),
        }
    }

    /// Returns the size the child view should actually use.
    ///
    /// When it said it wanted `result`.
    pub fn result(self, (result, available): (usize, usize)) -> usize {
        match self {
            SizeConstraint::AtLeast(value) if result < value => value, /* max(result, value) */
            SizeConstraint::AtMost(value) if result > value => value,  /* min(result, value) */
            SizeConstraint::Fixed(value) => value,
            // Explanation required: why return result if result > available?
            SizeConstraint::Full if available > result => available,
            _ => result,
        }
    }
}
