use vec::Vec2;
use view::{View,SizeRequest};
use printer::Printer;
use event::EventResult;

/// Wrapper around a view. Can override some methods, forwards the others.
pub trait ViewWrapper {
    /// Get an immutable reference to the wrapped view, so that we can forward some calls to it.
    fn get_view(&self) -> &View;
    /// Get a mutable reference to the wrapped view, for the mutable methods.
    fn get_view_mut(&mut self) -> &mut View;

    /// Wraps the draw method.
    fn wrap_draw(&mut self, printer: &Printer, focused: bool) {
        self.get_view_mut().draw(printer, focused);
    }

    /// Wraps the get_min_size method.
    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.get_view().get_min_size(req)
    }

    /// Wraps the on_key_event method.
    fn wrap_on_key_event(&mut self, ch: i32) -> EventResult {
        self.get_view_mut().on_key_event(ch)
    }

    /// Wraps the layout method
    fn wrap_layout(&mut self, size: Vec2) {
        self.get_view_mut().layout(size);
    }

    /// Wraps the take_focus method
    fn wrap_take_focus(&mut self) -> bool {
        self.get_view_mut().take_focus()
    }
}

impl <T: ViewWrapper> View for T {
    fn draw(&mut self, printer: &Printer, focused: bool) {
        self.wrap_draw(printer, focused);
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.wrap_get_min_size(req)
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        self.wrap_on_key_event(ch)
    }

    fn layout(&mut self, size: Vec2) {
        self.wrap_layout(size);
    }

    fn take_focus(&mut self) -> bool {
        self.wrap_take_focus()
    }
}

/// Convenient macro to implement to two methods required for the ViewWrapper trait.
///
/// # Examples
///
/// If the wrapped view is in a box, just name it in the macro:
///
/// ```
/// struct BoxFooView {
///     content: Box<View>,
/// }
///
/// impl ViewWrapper for FooView {
///     wrap_impl!(self.content);
/// }
/// ```
///
/// If the content is directly a view, reference it:
///
/// ```
/// struct FooView<T: View> {
///     view: T,
/// }
///
/// impl <T> ViewWrapper for FooView<T> {
///     wrap_impl!(&self.view);
/// }
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
