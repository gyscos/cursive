use XY;
use vec::Vec2;
use view::{SizeConstraint, View, ViewWrapper};

/// Wrapper around another view, with a controlled size.
///
/// Each axis can independently be set to:
///
/// * Keep a **fixed** size
/// * Use **all** available size
/// * Use **at most** a given size
/// * Use **at least** a given size
/// * Let the wrapped view decide.
///
/// # Examples
///
/// ```
/// # use cursive::views::{BoxView,TextView};
/// // Creates a 20x4 BoxView with a TextView content.
/// let view = BoxView::with_fixed_size((20,4), TextView::new("Hello!"));
/// ```
pub struct BoxView<T: View> {
    /// Constraint on each axis
    size: XY<SizeConstraint>,

    /// `true` if the view can be squished.
    ///
    /// This means if the required size is less than the computed size,
    /// consider returning a smaller size.
    /// For instance, try to return the child's desires size.
    squishable: bool,

    /// The actual view we're wrapping.
    view: T,
}

impl<T: View> BoxView<T> {
    /// Creates a new `BoxView` with the given width and height requirements.
    ///
    /// `None` values will use the wrapped view's preferences.
    pub fn new(width: SizeConstraint, height: SizeConstraint, view: T)
               -> Self {
        BoxView {
            size: (width, height).into(),
            squishable: false,
            view: view,
        }
    }

    /// Sets `self` to be squishable.
    ///
    /// A squishable `BoxView` will take a smaller size than it should when
    /// the available space is too small. In that case, it will allow the
    /// child view to contract, if it can.
    ///
    /// More specifically, if the available space is less than the size we
    /// would normally ask for, return the child size.
    pub fn squishable(mut self) -> Self {
        self.squishable = true;
        self
    }

    /// Wraps `view` in a new `BoxView` with the given size.
    pub fn with_fixed_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(SizeConstraint::Fixed(size.x),
                     SizeConstraint::Fixed(size.y),
                     view)
    }

    /// Wraps `view` in a new `BoxView` with fixed width.
    pub fn with_fixed_width(width: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::Fixed(width), SizeConstraint::Free, view)
    }

    /// Wraps `view` in a new `BoxView` with fixed height.
    pub fn with_fixed_height(height: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::Free, SizeConstraint::Fixed(height), view)
    }

    /// Wraps `view` in a `BoxView` which will take all available space.
    pub fn with_full_screen(view: T) -> Self {
        BoxView::new(SizeConstraint::Full, SizeConstraint::Full, view)
    }

    /// Wraps `view` in a `BoxView` which will take all available width.
    pub fn with_full_width(view: T) -> Self {
        BoxView::new(SizeConstraint::Full, SizeConstraint::Free, view)
    }

    /// Wraps `view` in a `BoxView` which will take all available height.
    pub fn with_full_height(view: T) -> Self {
        BoxView::new(SizeConstraint::Free, SizeConstraint::Full, view)
    }

    /// Wraps `view` in a `BoxView` which will never be bigger than `size`.
    pub fn with_max_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(SizeConstraint::AtMost(size.x),
                     SizeConstraint::AtMost(size.y),
                     view)
    }

    /// Wraps `view` in a `BoxView` which will enforce a maximum width.
    ///
    /// The resulting width will never be more than `max_width`.
    pub fn with_max_width(max_width: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::AtMost(max_width),
                     SizeConstraint::Free,
                     view)
    }

    /// Wraps `view` in a `BoxView` which will enforce a maximum height.
    ///
    /// The resulting height will never be more than `max_height`.
    pub fn with_max_height(max_height: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::Free,
                     SizeConstraint::AtMost(max_height),
                     view)
    }

    /// Wraps `view` in a `BoxView` which will never be smaller than `size`.
    pub fn with_min_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(SizeConstraint::AtLeast(size.x),
                     SizeConstraint::AtLeast(size.y),
                     view)
    }

    /// Wraps `view` in a `BoxView` which will enforce a minimum width.
    ///
    /// The resulting width will never be less than `min_width`.
    pub fn with_min_width(min_width: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::AtLeast(min_width),
                     SizeConstraint::Free,
                     view)
    }

    /// Wraps `view` in a `BoxView` which will enforce a minimum height.
    ///
    /// The resulting height will never be less than `min_height`.
    pub fn with_min_height(min_height: usize, view: T) -> Self {
        BoxView::new(SizeConstraint::Free,
                     SizeConstraint::AtLeast(min_height),
                     view)
    }
}

impl<T: View> ViewWrapper for BoxView<T> {
    wrap_impl!(self.view: T);

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {

        let req = self.size.zip_map(req, SizeConstraint::available);
        let child_size = self.view.required_size(req);

        let result = self.size
            .zip_map(child_size.zip(req), SizeConstraint::result);

        debug!("{:?}", result);

        if self.squishable {
            // We respect the request if we're less or equal.
            let respect_req = result.zip_map(req, |res, req| res <= req);
            result.zip_map(respect_req.zip(child_size),
                           |res, (respect, child)| {
                if respect {
                    // If we respect the request, keep the result
                    res
                } else {
                    // Otherwise, take the child as squish attempt.
                    child
                }
            })
        } else {
            result
        }
    }
}

#[cfg(test)]
mod tests {

    use vec::Vec2;
    use view::{Boxable, View};
    use views::DummyView;

    // No need to test `draw()` method as it's directly forwarded.

    #[test]
    fn min_size() {
        let mut min_w = DummyView.full_screen().min_width(5);

        assert_eq!(Vec2::new(5, 1), min_w.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(5, 10), min_w.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(10, 1), min_w.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(10, 10), min_w.required_size(Vec2::new(10, 10)));

        let mut min_h = DummyView.full_screen().min_height(5);

        assert_eq!(Vec2::new(1, 5), min_h.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(1, 10), min_h.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(10, 5), min_h.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(10, 10), min_h.required_size(Vec2::new(10, 10)));

        let mut min_s = DummyView.full_screen().min_size((5, 5));

        assert_eq!(Vec2::new(5, 5), min_s.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(5, 10), min_s.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(10, 5), min_s.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(10, 10), min_s.required_size(Vec2::new(10, 10)));
    }

    #[test]
    fn max_size() {
        let mut max_w = DummyView.full_screen().max_width(5);

        assert_eq!(Vec2::new(1, 1), max_w.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(1, 10), max_w.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(5, 1), max_w.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(5, 10), max_w.required_size(Vec2::new(10, 10)));

        let mut max_h = DummyView.full_screen().max_height(5);

        assert_eq!(Vec2::new(1, 1), max_h.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(1, 5), max_h.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(10, 1), max_h.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(10, 5), max_h.required_size(Vec2::new(10, 10)));

        let mut max_s = DummyView.full_screen().max_size((5, 5));

        assert_eq!(Vec2::new(1, 1), max_s.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(1, 5), max_s.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(5, 1), max_s.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(5, 5), max_s.required_size(Vec2::new(10, 10)));
    }

    #[test]
    fn full_screen() {
        let mut full = DummyView.full_screen();

        assert_eq!(Vec2::new(1, 1), full.required_size(Vec2::new(1, 1)));
        assert_eq!(Vec2::new(1, 10), full.required_size(Vec2::new(1, 10)));
        assert_eq!(Vec2::new(10, 1), full.required_size(Vec2::new(10, 1)));
        assert_eq!(Vec2::new(10, 10), full.required_size(Vec2::new(10, 10)));
    }
}
