use view::{View, ViewWrapper};
use vec::Vec2;

/// Simple wrapper view that asks for all the space it can get.
pub struct FullView<T: View> {
    view: T,
}

impl<T: View> FullView<T> {
    /// Wraps the given view into a new FullView.
    pub fn new(view: T) -> Self {
        FullView { view: view }
    }
}

impl<T: View> ViewWrapper for FullView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&mut self, req: Vec2) -> Vec2 {
        req
    }
}
