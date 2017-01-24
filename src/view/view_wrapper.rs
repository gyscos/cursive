use Printer;

use direction::Direction;
use event::{Event, EventResult};
use std::any::Any;
use vec::Vec2;
use view::{Selector, View};

/// Generic wrapper around a view.
///
/// Default implementation forwards all calls to the child view.
/// Overrides some methods as desired.
///
/// You can use the [`wrap_impl!`] macro to define `get_view` and
/// `get_view_mut` for you.
///
/// [`wrap_impl!`]: ../macro.wrap_impl.html
pub trait ViewWrapper {
    /// Type that this view wraps.
    type V: View;

    /// Get an immutable reference to the wrapped view.
    fn get_view(&self) -> &Self::V;

    /// Get a mutable reference to the wrapped view.
    fn get_view_mut(&mut self) -> &mut Self::V;

    /// Wraps the `draw` method.
    fn wrap_draw(&self, printer: &Printer) {
        self.get_view().draw(printer);
    }

    /// Wraps the `required_size` method.
    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        self.get_view_mut().required_size(req)
    }

    /// Wraps the `on_event` method.
    fn wrap_on_event(&mut self, ch: Event) -> EventResult {
        self.get_view_mut().on_event(ch)
    }

    /// Wraps the `layout` method.
    fn wrap_layout(&mut self, size: Vec2) {
        self.get_view_mut().layout(size);
    }

    /// Wraps the `take_focus` method.
    fn wrap_take_focus(&mut self, source: Direction) -> bool {
        self.get_view_mut().take_focus(source)
    }

    /// Wraps the `find` method.
    fn wrap_find_any(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.get_view_mut().find_any(selector)
    }

    /// Wraps the `needs_relayout` method.
    fn wrap_needs_relayout(&self) -> bool {
        self.get_view().needs_relayout()
    }
}

impl<T: ViewWrapper> View for T {
    fn draw(&self, printer: &Printer) {
        self.wrap_draw(printer);
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        self.wrap_required_size(req)
    }

    fn on_event(&mut self, ch: Event) -> EventResult {
        self.wrap_on_event(ch)
    }

    fn layout(&mut self, size: Vec2) {
        self.wrap_layout(size);
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.wrap_take_focus(source)
    }

    fn find_any(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.wrap_find_any(selector)
    }

    fn needs_relayout(&self) -> bool {
        self.wrap_needs_relayout()
    }
}

/// Convenient macro to implement the [`ViewWrapper`] trait.
///
/// It defines the `get_view` and `get_view_mut` implementations,
/// as well as the `type V` declaration.
///
/// [`ViewWrapper`]: view/trait.ViewWrapper.html
///
/// # Examples
///
/// ```no_run
/// # #[macro_use] extern crate cursive;
/// # use cursive::view::{View,ViewWrapper};
/// struct FooView<T: View> {
///     view: T,
/// }
///
/// impl <T: View> ViewWrapper for FooView<T> {
///     wrap_impl!(self.view: T);
/// }
/// # fn main() { }
/// ```
#[macro_export]
macro_rules! wrap_impl {
    (self.$v:ident: $t:ty) => {
        type V = $t;

        fn get_view(&self) -> &Self::V {
            &self.$v
        }

        fn get_view_mut(&mut self) -> &mut Self::V {
            &mut self.$v
        }
    };
}
