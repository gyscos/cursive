use std::any::Any;

use direction::Direction;
use vec::Vec2;
use view::{Selector, View};
use Printer;
use event::{Event, EventResult};

/// Generic wrapper around a view.
///
/// Default implementation forwards all calls to the child view.
/// Overrides some methods as desired.
pub trait ViewWrapper {
    /// Get an immutable reference to the wrapped view.
    fn get_view(&self) -> &View;

    /// Get a mutable reference to the wrapped view.
    fn get_view_mut(&mut self) -> &mut View;

    /// Wraps the `draw` method.
    fn wrap_draw(&self, printer: &Printer) {
        self.get_view().draw(printer);
    }

    /// Wraps the `get_min_size` method.
    fn wrap_get_min_size(&mut self, req: Vec2) -> Vec2 {
        self.get_view_mut().get_min_size(req)
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
    fn wrap_find(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.get_view_mut().find(selector)
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

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        self.wrap_get_min_size(req)
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

    fn find(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.wrap_find(selector)
    }

    fn needs_relayout(&self) -> bool {
        self.wrap_needs_relayout()
    }
}

/// Convenient macro to implement the ViewWrapper trait.
///
/// # Examples
///
/// If the wrapped view is in a box, just name it in the macro:
///
/// ```no_run
/// # #[macro_use] extern crate cursive;
/// # use cursive::view::{View,ViewWrapper};
/// struct BoxFooView {
///     content: Box<View>,
/// }
///
/// impl ViewWrapper for BoxFooView {
///     wrap_impl!(self.content);
/// }
/// # fn main() { }
/// ```
///
/// If the content is directly a view, reference it:
///
/// ```no_run
/// # #[macro_use] extern crate cursive;
/// # use cursive::view::{View,ViewWrapper};
/// struct FooView<T: View> {
///     view: T,
/// }
///
/// impl <T: View> ViewWrapper for FooView<T> {
///     wrap_impl!(&self.view);
/// }
/// # fn main() { }
/// ```
#[macro_export]
macro_rules! wrap_impl {
    (&self.$v:ident) => {

        fn get_view(&self) -> &View {
            &self.$v
        }

        fn get_view_mut(&mut self) -> &mut View {
            &mut self.$v
        }
    };
    (self.$v:ident) => {

        fn get_view(&self) -> &View {
            &*self.$v
        }

        fn get_view_mut(&mut self) -> &mut View {
            &mut *self.$v
        }
    };
}
