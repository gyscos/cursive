use std::borrow::Cow;

use view::{View, ViewWrapper};
use printer::Printer;
use vec::Vec2;
use theme::ColorStyle;

/// Wrapper view that adds a shadow.
///
/// It reserves a 1 pixel border on each side.
pub struct ShadowView<T: View> {
    view: T,
    topleft_padding: bool,
}

impl<T: View> ShadowView<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        ShadowView {
            view: view,
            topleft_padding: true,
        }
    }

    fn padding(&self) -> (usize, usize) {
        if self.topleft_padding {
            (2, 2)
        } else {
            (1, 1)
        }
    }

    pub fn no_topleft_padding(mut self) -> Self {
        self.topleft_padding = false;
        self
    }

}

impl<T: View> ViewWrapper for ShadowView<T> {
    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, req: Vec2) -> Vec2 {
        let offset = self.padding();
        self.view.get_min_size(req - offset) + offset
    }

    fn wrap_layout(&mut self, size: Vec2) {
        let offset = self.padding();
        self.view.layout(size - offset);
    }

    fn wrap_draw(&mut self, printer: &Printer) {

        // Skip the first row/column
        let printer = if self.topleft_padding {
            Cow::Owned(printer.sub_printer(Vec2::new(1, 1), printer.size, true))
        } else {
            Cow::Borrowed(printer)
        };

        // Draw the view background
        for y in 0..printer.size.y - 1 {
            printer.print_hline((0, y), printer.size.x - 1, " ");
        }

        self.view.draw(&printer.sub_printer(Vec2::zero(),
                                            printer.size - (1, 1),
                                            true));

        let h = printer.size.y;
        let w = printer.size.x;

        printer.with_color(ColorStyle::Shadow, |printer| {
            printer.print_hline((1, h-1), w - 1, " ");
            printer.print_vline((w-1, 1), h - 1, " ");
        });
    }
}
