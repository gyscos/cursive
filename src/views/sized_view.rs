use crate::vec::Vec2;
use crate::view::View;
use crate::view::ViewWrapper;

/// Wrapper around a view that remembers its size.
pub struct SizedView<T> {
    /// Wrapped view.
    pub view: T,
    /// Cached size from the last layout() call.
    pub size: Vec2,
}

impl<T> SizedView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        SizedView {
            view,
            size: Vec2::zero(),
        }
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for SizedView<T> {
    wrap_impl!(self.view: T);

    fn wrap_layout(&mut self, size: Vec2) {
        self.size = size;
        self.view.layout(size);
    }
}
