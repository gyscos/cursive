use crate::printer::Printer;
use crate::view::{SizeConstraint, View, ViewWrapper};
use crate::Vec2;
use crate::XY;

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
/// use cursive_core::views::{ResizedView, TextView};
///
/// // Creates a 20x4 ResizedView with a TextView content.
/// let view = ResizedView::with_fixed_size((20, 4), TextView::new("Hello!"));
/// ```
///
/// See also [`Resizable`](crate::view::Resizable) for an easy way to wrap any view.
pub struct ResizedView<T> {
    /// Constraint on each axis
    size: XY<SizeConstraint>,

    /// Set to `true` whenever we change some settings. Means we should re-layout just in case.
    invalidated: bool,

    /// The actual view we're wrapping.
    view: T,
}

impl<T> ResizedView<T> {
    /// Creates a new `ResizedView` with the given width and height requirements.
    ///
    /// `None` values will use the wrapped view's preferences.
    pub fn new(width: SizeConstraint, height: SizeConstraint, view: T) -> Self {
        ResizedView {
            size: (width, height).into(),
            invalidated: true,
            view,
        }
    }

    /// Sets the size constraints for this view.
    pub fn set_constraints(&mut self, width: SizeConstraint, height: SizeConstraint) {
        self.set_width(width);
        self.set_height(height);
    }

    /// Sets the width constraint for this view.
    ///
    /// Leaves the height unchanged.
    pub fn set_width(&mut self, width: SizeConstraint) {
        self.size.x = width;
        self.invalidate();
    }

    /// Sets the height constraint for this view.
    ///
    /// Leaves the width unchanged.
    pub fn set_height(&mut self, height: SizeConstraint) {
        self.size.y = height;
        self.invalidate();
    }

    /// Wraps `view` in a new `ResizedView` with the given size.
    pub fn with_fixed_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        ResizedView::new(
            SizeConstraint::Fixed(size.x),
            SizeConstraint::Fixed(size.y),
            view,
        )
    }

    /// Wraps `view` in a new `ResizedView` with fixed width.
    pub fn with_fixed_width(width: usize, view: T) -> Self {
        ResizedView::new(SizeConstraint::Fixed(width), SizeConstraint::Free, view)
    }

    /// Wraps `view` in a new `ResizedView` with fixed height.
    pub fn with_fixed_height(height: usize, view: T) -> Self {
        ResizedView::new(SizeConstraint::Free, SizeConstraint::Fixed(height), view)
    }

    /// Wraps `view` in a `ResizedView` which will take all available space.
    pub fn with_full_screen(view: T) -> Self {
        ResizedView::new(SizeConstraint::Full, SizeConstraint::Full, view)
    }

    /// Wraps `view` in a `ResizedView` which will take all available width.
    pub fn with_full_width(view: T) -> Self {
        ResizedView::new(SizeConstraint::Full, SizeConstraint::Free, view)
    }

    /// Wraps `view` in a `ResizedView` which will take all available height.
    pub fn with_full_height(view: T) -> Self {
        ResizedView::new(SizeConstraint::Free, SizeConstraint::Full, view)
    }

    /// Wraps `view` in a `ResizedView` which will never be bigger than `size`.
    pub fn with_max_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        ResizedView::new(
            SizeConstraint::AtMost(size.x),
            SizeConstraint::AtMost(size.y),
            view,
        )
    }

    /// Wraps `view` in a `ResizedView` which will enforce a maximum width.
    ///
    /// The resulting width will never be more than `max_width`.
    pub fn with_max_width(max_width: usize, view: T) -> Self {
        ResizedView::new(
            SizeConstraint::AtMost(max_width),
            SizeConstraint::Free,
            view,
        )
    }

    /// Wraps `view` in a `ResizedView` which will enforce a maximum height.
    ///
    /// The resulting height will never be more than `max_height`.
    pub fn with_max_height(max_height: usize, view: T) -> Self {
        ResizedView::new(
            SizeConstraint::Free,
            SizeConstraint::AtMost(max_height),
            view,
        )
    }

    /// Wraps `view` in a `ResizedView` which will never be smaller than `size`.
    ///
    /// As long as the parent view is large enough.
    ///
    /// If the space is constrained (for example the window is too small),
    /// this view might still be given a smaller size than requested.
    pub fn with_min_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        ResizedView::new(
            SizeConstraint::AtLeast(size.x),
            SizeConstraint::AtLeast(size.y),
            view,
        )
    }

    /// Wraps `view` in a `ResizedView` which will enforce a minimum width.
    ///
    /// The resulting width will never be less than `min_width`.
    pub fn with_min_width(min_width: usize, view: T) -> Self {
        ResizedView::new(
            SizeConstraint::AtLeast(min_width),
            SizeConstraint::Free,
            view,
        )
    }

    /// Wraps `view` in a `ResizedView` which will enforce a minimum height.
    ///
    /// The resulting height will never be less than `min_height`.
    pub fn with_min_height(min_height: usize, view: T) -> Self {
        ResizedView::new(
            SizeConstraint::Free,
            SizeConstraint::AtLeast(min_height),
            view,
        )
    }

    /// Should be called anytime something changes.
    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for ResizedView<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer) {
        let available = self
            .size
            .zip_map(printer.size, |c, s| c.result((s, s)))
            .or_min(printer.size);

        self.view.draw(&printer.inner_size(available));
    }

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // This is what the child will see as request.
        let req = self.size.zip_map(req, SizeConstraint::available);

        // This is the size the child would like to have.
        // Given the constraints of our box.
        // TODO: Skip running this if not needed?
        let child_size = self.view.required_size(req);

        // Some of this request will be granted, but maybe not all.
        self.size
            .zip_map(child_size.zip(req), SizeConstraint::result)
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.invalidated = false;
        let available = self
            .size
            .zip_map(size, |c, s| c.result((s, s)))
            .or_min(size);
        self.view.layout(available);
    }

    fn wrap_needs_relayout(&self) -> bool {
        self.invalidated || self.view.needs_relayout()
    }
}

#[cfg(test)]
mod tests {

    use crate::view::{Resizable, View};
    use crate::views::DummyView;
    use crate::Vec2;

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

    #[test]
    fn test_get_inner() {
        use crate::views::TextView;

        let parent = TextView::new("abc").full_screen();
        let child = parent.get_inner();
        assert_eq!(child.get_content().source(), "abc");
    }

    #[test]
    fn test_get_inner_mut() {
        use crate::views::TextView;

        let mut parent = TextView::new("").full_screen();
        let new_value = "new";
        let child = parent.get_inner_mut();

        child.set_content(new_value);

        assert_eq!(child.get_content().source(), new_value);
    }

    #[test]
    fn test_restricted() {
        // Make sure that we don't offer the inner view more space than we have.
        use crate::views::{LastSizeView, ResizedView};

        fn test_view(
            mut view: ResizedView<LastSizeView<DummyView>>,
            expected_big: Vec2,
            expected_small: Vec2,
        ) {
            view.layout(Vec2::new(10, 10));
            assert_eq!(view.view.size, expected_big);

            view.layout(Vec2::new(3, 10));
            assert_eq!(view.view.size, expected_small);
        }

        test_view(
            LastSizeView::new(DummyView).fixed_size((5usize, 5)),
            Vec2::new(5, 5),
            Vec2::new(3, 5),
        );
        test_view(
            LastSizeView::new(DummyView).max_size((5usize, 5)),
            Vec2::new(5, 5),
            Vec2::new(3, 5),
        );
        test_view(
            LastSizeView::new(DummyView).min_size((5usize, 5)),
            Vec2::new(10, 10),
            Vec2::new(3, 10),
        );
    }
}

#[crate::blueprint(ResizedView::new(SizeConstraint::Free, SizeConstraint::Free, view))]
struct Blueprint {
    view: crate::views::BoxedView,
    width: Option<SizeConstraint>,
    height: Option<SizeConstraint>,
}

crate::manual_blueprint!(with full_screen, |_config, _context| {
    Ok(crate::views::ResizedView::with_full_screen)
});

crate::manual_blueprint!(with full_width, |_config, _context| {
    Ok(crate::views::ResizedView::with_full_width)
});

crate::manual_blueprint!(with full_height, |_config, _context| {
    Ok(crate::views::ResizedView::with_full_height)
});

crate::manual_blueprint!(with fixed_size, |config, context| {
    let size: Vec2 = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_fixed_size(size, view))
});

crate::manual_blueprint!(with max_size, |config, context| {
    let size: Vec2 = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_max_size(size, view))
});

crate::manual_blueprint!(with min_size, |config, context| {
    let size: Vec2 = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_min_size(size, view))
});

crate::manual_blueprint!(with fixed_width, |config, context| {
    let width = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_fixed_width(width, view))
});

crate::manual_blueprint!(with fixed_height, |config, context| {
    let height = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_fixed_height(height, view))
});

crate::manual_blueprint!(with max_width, |config, context| {
    let width = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_max_width(width, view))
});

crate::manual_blueprint!(with max_height, |config, context| {
    let height = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_max_height(height, view))
});

crate::manual_blueprint!(with min_width, |config, context| {
    let width = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_min_width(width, view))
});

crate::manual_blueprint!(with min_height, |config, context| {
    let height = context.resolve(config)?;
    Ok(move |view| crate::views::ResizedView::with_min_height(height, view))
});
