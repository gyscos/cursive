use std::cmp;

use vec::{ToVec2, Vec2};
use super::{View, ViewWrapper};

/// `BoxView` is a wrapper around an other view, with a given minimum size.
pub struct BoxView<T: View> {
    size: Vec2,
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
    /// let view = BoxView::new((20,4), TextView::new("Hello!"));
    /// ```
    pub fn new<S: ToVec2>(size: S, view: T) -> Self {
        BoxView {
            size: size.to_vec2(),
            view: view,
        }
    }
}

impl<T: View> ViewWrapper for BoxView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, mut req: Vec2) -> Vec2 {
        if self.size.x > 0 {
            req.x = cmp::min(self.size.x, req.x);
        }
        if self.size.y > 0 {
            req.y = cmp::min(self.size.y, req.y);
        }

        let mut size = self.view.get_min_size(req);

        // Did he think he got to decide?
        // Of course we have the last word here.
        if self.size.x > 0 {
            size.x = self.size.x;
        }
        if self.size.y > 0 {
            size.y = self.size.y;
        }

        size
    }
}
