use Printer;
use vec::Vec2;
use view::{View, ViewWrapper};

/// Draws a border around a wrapped view.
pub struct Panel<V: View> {
    view: V,
}

impl<V: View> Panel<V> {
    /// Creates a new panel around the given view.
    pub fn new(view: V) -> Self {
        Panel { view: view }
    }
}


impl<V: View> ViewWrapper for Panel<V> {
    wrap_impl!(self.view: V);

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // TODO: make borders conditional?
        self.view.required_size(req - (2, 2)) + (2, 2)
    }

    fn wrap_draw(&self, printer: &Printer) {
        printer.print_box((0, 0), printer.size, true);
        self.view
            .draw(&printer.sub_printer((1, 1), printer.size - (2, 2), true));
    }
}
