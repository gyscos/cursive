use view::View;
use vec::Vec2;
use view::ViewWrapper;

/// Wrapper around a view that remembers its size.
pub struct SizedView<T: View> {
    /// Wrapped view.
    pub view: T,
    /// Cached size from the last layout() call.
    pub size: Vec2,
}

impl<T: View> SizedView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        SizedView {
            view: view,
            size: Vec2::zero(),
        }
    }
}

impl<T: View> ViewWrapper for SizedView<T> {
    wrap_impl!(&self.view);

    fn wrap_layout(&mut self, size: Vec2) {
        self.size = size;
        self.view.layout(size);
    }
}
