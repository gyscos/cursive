use Printer;
use view::{View, ViewWrapper};

/// Wrapper view that fills the background.
///
/// Used as layer in the [`StackView`].
///
/// [`StackView`]: struct.StackView.html
pub struct Layer<T: View> {
    view: T,
}

impl<T: View> Layer<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        Layer { view: view }
    }
}

impl<T: View> ViewWrapper for Layer<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer) {
        for y in 0..printer.size.y {
            printer.print_hline((0, y), printer.size.x, " ");
        }
        self.view.draw(printer);
    }
}
