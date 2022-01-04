use crate::event::{Event, EventResult};
use crate::view::{View, ViewWrapper};
use crate::Printer;

/// Wrapper around another view that can be enabled/disabled at will.
///
/// When disabled, all child views will be disabled and will stop receiving events.
///
/// # Examples
///
/// ```
/// use cursive_core::Cursive;
/// use cursive_core::views::{Button, EnableableView, Checkbox, LinearLayout};
/// use cursive_core::traits::Nameable;
///
/// let mut siv = Cursive::new();
///
/// siv.add_layer(LinearLayout::vertical()
///     .child(EnableableView::new(Checkbox::new()).with_name("my_view"))
///     .child(Button::new("Toggle", |s| {
///         s.call_on_name("my_view", |v: &mut EnableableView<Checkbox>| {
///             // This will disable (or re-enable) the checkbox, preventing the user from
///             // interacting with it.
///             v.set_enabled(!v.is_enabled());
///         });
///     }))
/// );
/// ```
pub struct EnableableView<V> {
    view: V,
    enabled: bool,
}

new_default!(EnableableView<V: Default>);

impl<V> EnableableView<V> {
    /// Creates a new `EnableableView` around `view`.
    ///
    /// It will be enabled by default.
    pub fn new(view: V) -> Self {
        EnableableView {
            view,
            enabled: true,
        }
    }

    impl_enabled!(self.enabled);
    inner_getters!(self.view: V);
}

impl<V: View> ViewWrapper for EnableableView<V> {
    wrap_impl!(self.view: V);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        if self.enabled {
            self.view.on_event(event)
        } else {
            EventResult::Ignored
        }
    }

    fn wrap_draw(&self, printer: &Printer) {
        self.view.draw(&printer.enabled(self.enabled));
    }
}
