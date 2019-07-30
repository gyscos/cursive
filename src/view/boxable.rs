use crate::vec::Vec2;
use crate::view::{SizeConstraint, View};
use crate::views::BoxView;

/// Makes a view wrappable in a [`BoxView`].
///
/// [`BoxView`]: ../views/struct.BoxView.html
pub trait Boxable: View + Sized {
    /// Wraps `self` in a `BoxView` with the given size constraints.
    fn boxed(
        self,
        width: SizeConstraint,
        height: SizeConstraint,
    ) -> BoxView<Self> {
        BoxView::new(width, height, self)
    }

    /// Wraps `self` into a fixed-size `BoxView`.
    fn fixed_size<S: Into<Vec2>>(self, size: S) -> BoxView<Self> {
        BoxView::with_fixed_size(size, self)
    }

    /// Wraps `self` into a fixed-width `BoxView`.
    fn fixed_width(self, width: usize) -> BoxView<Self> {
        BoxView::with_fixed_width(width, self)
    }

    /// Wraps `self` into a fixed-width `BoxView`.
    fn fixed_height(self, height: usize) -> BoxView<Self> {
        BoxView::with_fixed_height(height, self)
    }

    /// Wraps `self` into a full-screen `BoxView`.
    fn full_screen(self) -> BoxView<Self> {
        BoxView::with_full_screen(self)
    }

    /// Wraps `self` into a full-width `BoxView`.
    fn full_width(self) -> BoxView<Self> {
        BoxView::with_full_width(self)
    }

    /// Wraps `self` into a full-height `BoxView`.
    fn full_height(self) -> BoxView<Self> {
        BoxView::with_full_height(self)
    }

    /// Wraps `self` into a limited-size `BoxView`.
    fn max_size<S: Into<Vec2>>(self, size: S) -> BoxView<Self> {
        BoxView::with_max_size(size, self)
    }

    /// Wraps `self` into a limited-width `BoxView`.
    fn max_width(self, max_width: usize) -> BoxView<Self> {
        BoxView::with_max_width(max_width, self)
    }

    /// Wraps `self` into a limited-height `BoxView`.
    fn max_height(self, max_height: usize) -> BoxView<Self> {
        BoxView::with_max_height(max_height, self)
    }

    /// Wraps `self` into a `BoxView` at least sized `size`.
    fn min_size<S: Into<Vec2>>(self, size: S) -> BoxView<Self> {
        BoxView::with_min_size(size, self)
    }

    /// Wraps `self` in a `BoxView` at least `min_width` wide.
    fn min_width(self, min_width: usize) -> BoxView<Self> {
        BoxView::with_min_width(min_width, self)
    }

    /// Wraps `self` in a `BoxView` at least `min_height` tall.
    fn min_height(self, min_height: usize) -> BoxView<Self> {
        BoxView::with_min_height(min_height, self)
    }
}

impl<T: View> Boxable for T {}
