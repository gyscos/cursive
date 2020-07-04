use crate::view::{View, ViewWrapper};
use crate::Printer;
use crate::Vec2;
use std::cell::Cell;

/// Wrapper around a view that remembers its position.
pub struct TrackedView<T> {
    /// Wrapped view.
    pub view: T,
    /// Last position the view was located.
    offset: Cell<Vec2>,
}

new_default!(TrackedView<T: Default>);

impl<T> TrackedView<T> {
    /// Return the last offset at which the view was drawn.
    pub fn offset(&self) -> Vec2 {
        self.offset.get()
    }

    /// Creates a new `TrackedView` around `view`.
    pub fn new(view: T) -> Self {
        TrackedView {
            view,
            offset: Cell::new(Vec2::zero()),
        }
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for TrackedView<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer) {
        self.offset.set(printer.offset);
        self.view.draw(printer);
    }
}
