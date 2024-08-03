use crate::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    rect::Rect,
    view::{CannotFocus, Selector, View, ViewNotFound},
    Printer, Vec2,
};

/// Generic wrapper around a view.
///
/// This trait is a shortcut to implement `View` on a type by forwarding
/// calls to a wrapped view.
///
/// You only need to define `with_view` and `with_view_mut`
/// (the [`wrap_impl!`] macro can help you with that), and you will get
/// the `View` implementation for free.
///
/// You can also override any of the `wrap_*` methods for more specific
/// behaviors (the default implementations simply forwards the calls to the
/// child view).
///
/// [`wrap_impl!`]: crate::wrap_impl!
pub trait ViewWrapper: Send + Sync + 'static {
    /// Type that this view wraps.
    type V: View + ?Sized;

    /// Runs a function on the inner view, returning the result.
    ///
    /// Returns `None` if the inner view is unavailable.  This should only
    /// happen with some views if they are already borrowed by another call.
    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R;

    /// Runs a function on the inner view, returning the result.
    ///
    /// Returns `None` if the inner view is unavailable.  This should only
    /// happen with some views if they are already borrowed by another call.
    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R;

    /// Attempts to retrieve the inner view.
    fn into_inner(self) -> Result<Self::V, Self>
    where
        Self: Sized,
        Self::V: Sized,
    {
        Err(self)
    }

    /// Wraps the `draw` method.
    fn wrap_draw(&self, printer: &Printer) {
        self.with_view(|v| v.draw(printer));
    }

    /// Wraps the `required_size` method.
    fn wrap_required_size(&mut self, req: Vec2) -> Vec2 {
        self.with_view_mut(|v| v.required_size(req))
            .unwrap_or_else(Vec2::zero)
    }

    /// Wraps the `on_event` method.
    fn wrap_on_event(&mut self, ch: Event) -> EventResult {
        self.with_view_mut(|v| v.on_event(ch))
            .unwrap_or(EventResult::Ignored)
    }

    /// Wraps the `layout` method.
    fn wrap_layout(&mut self, size: Vec2) {
        self.with_view_mut(|v| v.layout(size));
    }

    /// Wraps the `take_focus` method.
    fn wrap_take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.with_view_mut(|v| v.take_focus(source))
            .unwrap_or(Err(CannotFocus))
    }

    /// Wraps the `find` method.
    fn wrap_call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        self.with_view_mut(|v| v.call_on_any(selector, callback));
    }

    /// Wraps the `focus_view` method.
    fn wrap_focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.with_view_mut(|v| v.focus_view(selector))
            .unwrap_or(Err(ViewNotFound))
    }

    /// Wraps the `needs_relayout` method.
    fn wrap_needs_relayout(&self) -> bool {
        self.with_view(View::needs_relayout).unwrap_or(true)
    }

    /// Wraps the `important_area` method.
    fn wrap_important_area(&self, size: Vec2) -> Rect {
        self.with_view(|v| v.important_area(size))
            .unwrap_or_else(|| Rect::from_size(Vec2::zero(), size))
    }
}

// The main point of implementing ViewWrapper is to have View for free.
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

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.wrap_take_focus(source)
    }

    fn call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        self.wrap_call_on_any(selector, callback)
    }

    fn needs_relayout(&self) -> bool {
        self.wrap_needs_relayout()
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.wrap_focus_view(selector)
    }

    fn important_area(&self, size: Vec2) -> Rect {
        self.wrap_important_area(size)
    }
}

/// Convenient macro to implement the [`ViewWrapper`] trait.
///
/// It defines the `with_view` and `with_view_mut` implementations,
/// as well as the `type V` declaration.
///
/// [`ViewWrapper`]: crate::view::ViewWrapper
///
/// # Examples
///
/// ```rust
/// # use cursive_core::view::{View,ViewWrapper};
/// struct FooView<T: View> {
///     view: T,
/// }
///
/// impl<T: View> ViewWrapper for FooView<T> {
///     cursive_core::wrap_impl!(self.view: T);
/// }
/// # fn main() { }
/// ```
#[macro_export]
macro_rules! wrap_impl {
    (self.$v:ident: $t:ty) => {
        type V = $t;

        fn with_view<F, R>(&self, f: F) -> ::std::option::Option<R>
        where
            F: ::std::ops::FnOnce(&Self::V) -> R,
        {
            ::std::option::Option::Some(f(&self.$v))
        }

        fn with_view_mut<F, R>(&mut self, f: F) -> ::std::option::Option<R>
        where
            F: ::std::ops::FnOnce(&mut Self::V) -> R,
        {
            ::std::option::Option::Some(f(&mut self.$v))
        }

        fn into_inner(self) -> ::std::result::Result<Self::V, Self>
        where
            Self::V: ::std::marker::Sized,
        {
            ::std::result::Result::Ok(self.$v)
        }
    };
}

/// Convenient macro to implement the getters for inner [`View`] in
/// [`ViewWrapper`].
///
/// It defines the `get_inner` and `get_inner_mut` implementations.
///
/// [`ViewWrapper`]: crate::view::ViewWrapper
/// [`View`]: crate::View
///
/// # Examples
///
/// ```rust
/// # use cursive_core::view::{View, ViewWrapper};
/// struct FooView<T: View> {
///     view: T,
/// }
///
/// impl<T: View> FooView<T> {
///     cursive_core::inner_getters!(self.view: T);
/// }
///
/// impl<T: View> ViewWrapper for FooView<T> {
///     cursive_core::wrap_impl!(self.view: T);
/// }
/// # fn main() { }
/// ```
#[macro_export]
macro_rules! inner_getters {
    (self.$v:ident: $t:ty) => {
        /// Gets access to the inner view.
        pub fn get_inner(&self) -> &$t {
            &self.$v
        }
        /// Gets mutable access to the inner view.
        pub fn get_inner_mut(&mut self) -> &mut $t {
            &mut self.$v
        }
    };
}
