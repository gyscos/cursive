use std::cmp;

use vec::Vec2;
use super::{View, ViewWrapper};

/// `BoxView` is a wrapper around an other view, with a given minimum size.
pub struct BoxView<T: View> {
    width: Option<usize>,
    height: Option<usize>,
    view: T,
}

impl<T: View> BoxView<T> {
    /// Creates a new `BoxView` with the given minimum size and content
    ///
    /// # Example
    ///
    /// ```
    /// # use cursive::view::{BoxView,TextView};
    /// // Creates a 20x4 BoxView with a TextView content.
    /// let view = BoxView::fixed_size((20,4), TextView::new("Hello!"));
    /// ```
    pub fn fixed_size<S: Into<Vec2>>(size: S, view: T) -> Self {
        let size = size.into();

        BoxView::new(Some(size.x), Some(size.y), view)
    }

    pub fn new(width: Option<usize>, height: Option<usize>, view: T) -> Self {
        BoxView {
            width: width,
            height: height,
            view: view,
        }
    }

    pub fn fixed_width(width: usize, view: T) -> Self {
        BoxView::new(Some(width), None, view)
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

        if let (Some(w), Some(h)) = (self.width, self.height) {
            Vec2::new(w, h)
        } else {
            let req = Vec2::new(min(req.x, self.width),
                                min(req.y, self.height));
            let child_size = self.view.get_min_size(req);

            Vec2::new(self.width.unwrap_or(child_size.x),
                      self.height.unwrap_or(child_size.y))
        }
    }
}
