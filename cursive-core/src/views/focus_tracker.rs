use crate::{
    direction::Direction,
    event::{Event, EventResult},
    view::{CannotFocus, View, ViewWrapper},
    With,
};

/// Detects focus events for a view.
pub struct FocusTracker<T> {
    view: T,
    on_focus_lost: Box<dyn FnMut(&mut T) -> EventResult>,
    on_focus: Box<dyn FnMut(&mut T) -> EventResult>,
}

impl<T> FocusTracker<T> {
    /// Wraps a view in a new `FocusTracker`.
    pub fn new(view: T) -> Self {
        FocusTracker {
            view,
            on_focus_lost: Box::new(|_| EventResult::Ignored),
            on_focus: Box::new(|_| EventResult::Ignored),
        }
    }

    /// Sets a callback to be run when the focus is gained.
    #[must_use]
    pub fn on_focus<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T) -> EventResult,
    {
        self.with(|s| s.on_focus = Box::new(f))
    }

    /// Sets a callback to be run when the focus is lost.
    #[must_use]
    pub fn on_focus_lost<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T) -> EventResult,
    {
        self.with(|s| s.on_focus_lost = Box::new(f))
    }
}

impl<T: View> ViewWrapper for FocusTracker<T> {
    wrap_impl!(self.view: T);

    fn wrap_take_focus(
        &mut self,
        source: Direction,
    ) -> Result<EventResult, CannotFocus> {
        match self.view.take_focus(source) {
            Ok(res) => Ok(res.and((self.on_focus)(&mut self.view))),
            Err(CannotFocus) => Err(CannotFocus),
        }
    }

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        let res = if let Event::FocusLost = event {
            (self.on_focus_lost)(&mut self.view)
        } else {
            EventResult::Ignored
        };
        res.and(self.view.on_event(event))
    }
}
