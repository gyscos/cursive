use view::{IdView, View, ViewWrapper};
use printer::Printer;
use vec::Vec2;

pub struct TrackedView<T: View> {
    pub view: T,
    pub offset: Vec2,
}

impl<T: View> TrackedView<T> {
    pub fn new(view: T) -> Self {
        TrackedView {
            view: view,
            offset: Vec2::zero(),
        }
    }

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
