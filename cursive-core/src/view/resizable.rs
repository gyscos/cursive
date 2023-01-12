use crate::view::{SizeConstraint, View};
use crate::views::ResizedView;
use crate::Vec2;

/// Makes a view wrappable in a [`ResizedView`].
///
/// [`ResizedView`]: ../views/struct.ResizedView.html
pub trait Resizable: View + Sized {
    /// Wraps `self` in a `ResizedView` with the given size constraints.
    fn resized(self, width: SizeConstraint, height: SizeConstraint) -> ResizedView<Self> {
        ResizedView::new(width, height, self)
    }

    /// Wraps `self` into a fixed-size `ResizedView`.
    fn fixed_size<S: Into<Vec2>>(self, size: S) -> ResizedView<Self> {
        ResizedView::with_fixed_size(size, self)
    }

    /// Wraps `self` into a fixed-width `ResizedView`.
    fn fixed_width(self, width: usize) -> ResizedView<Self> {
        ResizedView::with_fixed_width(width, self)
    }

    /// Wraps `self` into a fixed-width `ResizedView`.
    fn fixed_height(self, height: usize) -> ResizedView<Self> {
        ResizedView::with_fixed_height(height, self)
    }

    /// Wraps `self` into a full-screen `ResizedView`.
    fn full_screen(self) -> ResizedView<Self> {
        ResizedView::with_full_screen(self)
    }

    /// Wraps `self` into a full-width `ResizedView`.
    fn full_width(self) -> ResizedView<Self> {
        ResizedView::with_full_width(self)
    }

    /// Wraps `self` into a full-height `ResizedView`.
    fn full_height(self) -> ResizedView<Self> {
        ResizedView::with_full_height(self)
    }

    /// Wraps `self` into a limited-size `ResizedView`.
    fn max_size<S: Into<Vec2>>(self, size: S) -> ResizedView<Self> {
        ResizedView::with_max_size(size, self)
    }

    /// Wraps `self` into a limited-width `ResizedView`.
    fn max_width(self, max_width: usize) -> ResizedView<Self> {
        ResizedView::with_max_width(max_width, self)
    }

    /// Wraps `self` into a limited-height `ResizedView`.
    fn max_height(self, max_height: usize) -> ResizedView<Self> {
        ResizedView::with_max_height(max_height, self)
    }

    /// Wraps `self` into a `ResizedView` at least sized `size`.
    fn min_size<S: Into<Vec2>>(self, size: S) -> ResizedView<Self> {
        ResizedView::with_min_size(size, self)
    }

    /// Wraps `self` in a `ResizedView` at least `min_width` wide.
    fn min_width(self, min_width: usize) -> ResizedView<Self> {
        ResizedView::with_min_width(min_width, self)
    }

    /// Wraps `self` in a `ResizedView` at least `min_height` tall.
    fn min_height(self, min_height: usize) -> ResizedView<Self> {
        ResizedView::with_min_height(min_height, self)
    }
}

impl<T: View> Resizable for T {}
