use vec::{Vec2,ToVec2};
use super::{View,ViewWrapper,SizeRequest};

/// BoxView is a wrapper around an other view, with a given minimum size.
pub struct BoxView<T: View> {
    size: Vec2,
    view: T,
}

impl <T: View> BoxView<T> {
    /// Creates a new BoxView with the given minimum size and content
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

impl <T: View> ViewWrapper for BoxView<T> {

    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, _: SizeRequest) -> Vec2 {
        self.size
    }
}
