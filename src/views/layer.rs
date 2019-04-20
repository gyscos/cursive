use crate::view::{View, ViewWrapper};
use crate::Printer;

/// Wrapper view that fills the background.
///
/// Used as layer in the [`StackView`].
///
/// [`StackView`]: struct.StackView.html
#[derive(Debug)]
pub struct Layer<T: View> {
    view: T,
}

impl<T: View> Layer<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        Layer { view }
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for Layer<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer<'_, '_>) {
        for y in 0..printer.size.y {
            printer.print_hline((0, y), printer.size.x, " ");
        }
        self.view.draw(printer);
    }
}
