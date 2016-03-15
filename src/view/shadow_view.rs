use ncurses::chtype;
use view::{View, ViewWrapper, SizeRequest};
use printer::Printer;
use vec::Vec2;
use theme::ColorPair;

/// Wrapper view that adds a shadow.
///
/// It reserves a 1 pixel border on each side.
pub struct ShadowView<T: View> {
    view: T,
}

impl <T: View> ShadowView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        ShadowView { view: view }
    }
}

impl <T: View> ViewWrapper for ShadowView<T> {

    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.view.get_min_size(req.reduced((2, 2))) + (2, 2)
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.view.layout(size - (2, 2));
    }

    fn wrap_draw(&mut self, printer: &Printer) {

        printer.with_color(ColorPair::Primary, |printer| {
            // Draw the view background
            for y in 1..printer.size.y - 1 {
                printer.print_hline((1, y), printer.size.x - 2, ' ' as chtype);
            }
        });

        self.view.draw(&printer.sub_printer(Vec2::new(1, 1), printer.size - (2, 2), true));

        let h = printer.size.y - 1;
        let w = printer.size.x - 1;

        printer.with_color(ColorPair::Shadow, |printer| {
            printer.print_hline((2, h), w - 1, ' ' as chtype);
            printer.print_vline((w, 2), h - 1, ' ' as chtype);
        });
    }
}
