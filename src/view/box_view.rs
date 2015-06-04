use vec::{Vec2,ToVec2};
use super::{View,ViewWrapper,SizeRequest};

/// BoxView is a wrapper around an other view, with a given minimum size.
pub struct BoxView {
    size: Vec2,

    content: Box<View>,
}

impl BoxView {
    /// Creates a new BoxView with the given minimum size and content
    ///
    /// # Example
    ///
    /// ```
    /// # use cursive::view::{BoxView,TextView};
    /// // Creates a 20x4 BoxView with a TextView content.
    /// let view = BoxView::new((20,4), TextView::new("Hello!"));
    /// ```
    pub fn new<S: ToVec2, V: View + 'static>(size: S, view: V) -> Self {
        BoxView {
            size: size.to_vec2(),
            content: Box::new(view),
        }
    }
}

impl ViewWrapper for BoxView {

    wrap_impl!(self.content);

    fn wrap_get_min_size(&self, _: SizeRequest) -> Vec2 {
        self.size
    }
}
