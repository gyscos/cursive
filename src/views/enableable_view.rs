use crate::event::{Event, EventResult};
use crate::view::{View, ViewWrapper};
use crate::{Printer, With};

/// Wrapper around another view that can be enabled/disabled at will.
///
/// When disabled, all child views will be disabled and will stop receiving events.
pub struct EnableableView<V> {
    view: V,
    enabled: bool,
}

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

    fn wrap_draw(&self, printer: &Printer<'_, '_>) {
        self.view.draw(&printer.enabled(self.enabled));
    }
}
