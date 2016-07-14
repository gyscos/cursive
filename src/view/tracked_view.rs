use view::{IdView, View, ViewWrapper};
use Printer;
use vec::Vec2;

/// Wrapper around a view that remembers its position.
pub struct TrackedView<T: View> {
    /// Wrapped view.
    pub view: T,
    /// Last position the view was located.
    pub offset: Vec2,
}

impl<T: View> TrackedView<T> {
    /// Creates a new `TrackedView` around `view`.
    pub fn new(view: T) -> Self {
        TrackedView {
            view: view,
            offset: Vec2::zero(),
        }
    }

    /// Wraps itself in a `IdView` for easy retrieval.
    pub fn with_id(self, id: &str) -> IdView<Self> {
        IdView::new(id, self)
    }
}

impl<T: View> ViewWrapper for TrackedView<T> {
    wrap_impl!(&self.view);

    fn wrap_draw(&mut self, printer: &Printer) {
        self.offset = printer.offset;
        self.view.draw(printer);
    }
}
