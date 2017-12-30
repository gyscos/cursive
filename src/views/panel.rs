use Printer;
use event::{Event, EventResult};
use vec::Vec2;
use view::{View, ViewWrapper};

/// Draws a border around a wrapped view.
#[derive(Debug)]
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

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        self.view.on_event(event.relativized((1, 1)))
    }

    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        // TODO: make borders conditional?
        let req = req.saturating_sub((2, 2));

        self.view.required_size(req) + (2, 2)
    }

    fn wrap_draw(&self, printer: &Printer) {
        printer.print_box((0, 0), printer.size, true);
        self.view.draw(&printer.sub_printer(
            (1, 1),
            printer.size.saturating_sub((2, 2)),
            true,
        ));
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.view.layout(size.saturating_sub((2, 2)));
    }
}
