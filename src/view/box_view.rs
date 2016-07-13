use std::cmp;

use XY;
use vec::Vec2;
use super::{View, ViewWrapper};

/// `BoxView` is a wrapper around an other view, with a given minimum size.
///
/// # Example
///
/// ```
/// # use cursive::view::{BoxView,TextView};
/// // Creates a 20x4 BoxView with a TextView content.
/// let view = BoxView::fixed_size((20,4), TextView::new("Hello!"));
/// ```
pub struct BoxView<T: View> {
    size: XY<Option<usize>>,
    view: T,
}

impl<T: View> BoxView<T> {
    /// Wraps `view` in a neww `BoxView` with the given size.
    pub fn fixed_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(Some(size.x), Some(size.y), view)
    }

    /// Creates a new `BoxView` with the given width and height requirements.
    ///
    /// `None` values will use the wrapped view's preferences.
    pub fn new(width: Option<usize>, height: Option<usize>, view: T) -> Self {
        BoxView {
            size: (width, height).into(),
            view: view,
        }
    }

    /// Wraps `view` in a new `BoxView` with fixed width.
    pub fn fixed_width(width: usize, view: T) -> Self {
        BoxView::new(Some(width), None, view)
    }

    /// Wraps `view` in a new `BoxView` with fixed height.
    pub fn fixed_height(height: usize, view: T) -> Self {
        BoxView::new(None, Some(height), view)
    }
}

fn min<T: Ord>(a: T, b: Option<T>) -> T {
    match b {
        Some(b) => cmp::min(a, b),
        None => a,
    }
}

impl<T: View> ViewWrapper for BoxView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&mut self, req: Vec2) -> Vec2 {

        if let (Some(w), Some(h)) = self.size.pair() {
            Vec2::new(w, h)
        } else {
            let req = Vec2::new(min(req.x, self.size.x),
                                min(req.y, self.size.y));
            let child_size = self.view.get_min_size(req);

            Vec2::new(self.size.x.unwrap_or(child_size.x),
                      self.size.y.unwrap_or(child_size.y))
        }.or_min(req)
    }
}
