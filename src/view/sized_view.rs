use view::View;
use vec::Vec2;
use view::ViewWrapper;

/// Wrapper around a view that remembers its size.
pub struct SizedView <T: View> {
    pub view: T,
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

impl <T: View> ViewWrapper for SizedView<T> {
    fn get_view(&self) -> &View {
        &self.view
    }

    fn get_view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.view.layout(size);
        self.size = size;
    }
}
