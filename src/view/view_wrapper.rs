use std::any::Any;

use vec::Vec2;
use view::{View,SizeRequest,Selector};
use printer::Printer;
use event::{Event,EventResult};

/// Generic wrapper around a view.
///
/// Default implementation forwards all calls to the child view.
/// Overrides some methods as desired.
pub trait ViewWrapper {
    /// Get an immutable reference to the wrapped view, so that we can forward some calls to it.
    fn get_view(&self) -> &View;
    /// Get a mutable reference to the wrapped view, for the mutable methods.
    fn get_view_mut(&mut self) -> &mut View;

    /// Wraps the draw method.
    fn wrap_draw(&mut self, printer: &Printer) {
        self.get_view_mut().draw(printer);
    }

    /// Wraps the get_min_size method.
    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.get_view().get_min_size(req)
    }

    /// Wraps the on_event method.
    fn wrap_on_event(&mut self, ch: Event) -> EventResult {
        self.get_view_mut().on_event(ch)
    }

    /// Wraps the layout method
    fn wrap_layout(&mut self, size: Vec2) {
        self.get_view_mut().layout(size);
    }

    /// Wraps the take_focus method
    fn wrap_take_focus(&mut self) -> bool {
        self.get_view_mut().take_focus()
    }

    fn wrap_find(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.get_view_mut().find(selector)
    }
}

impl <T: ViewWrapper> View for T {
    fn draw(&mut self, printer: &Printer) {
        self.wrap_draw(printer);
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.wrap_get_min_size(req)
    }

    fn on_event(&mut self, ch: Event) -> EventResult {
        self.wrap_on_event(ch)
    }

    fn layout(&mut self, size: Vec2) {
        self.wrap_layout(size);
    }

    fn take_focus(&mut self) -> bool {
        self.wrap_take_focus()
    }

    fn find(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.wrap_find(selector)
    }
}

/// Convenient macro to implement to two methods required for the ViewWrapper trait.
///
/// # Examples
///
/// If the wrapped view is in a box, just name it in the macro:
///
/// ```rust
/// # #[macro_use] extern crate cursive;
/// # use cursive::view::{View,ViewWrapper};
/// struct BoxFooView {
///     content: Box<View>,
/// }
///
/// impl ViewWrapper for BoxFooView {
///     wrap_impl!(self.content);
/// }
///
/// # fn main() { }
/// ```
///
/// If the content is directly a view, reference it:
///
/// ```
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

