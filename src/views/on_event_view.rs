

use Cursive;
use event::{Callback, Event, EventResult};
use std::collections::HashMap;
use view::{View, ViewWrapper};
use With;

/// A simple wrapper view that catches some ignored event from its child.
///
/// If the event doesn't have a corresponding callback, it will stay ignored.
///
/// # Examples
///
/// ```
/// # use cursive::event;;
/// # use cursive::views::{OnEventView, TextView};
/// let view = OnEventView::new(TextView::new("This view has an event!"))
///                         .on_event('q', |s| s.quit())
///                         .on_event(event::Key::Esc, |s| s.quit());
/// ```
pub struct OnEventView<T: View> {
    content: T,
    callbacks: HashMap<Event, Callback>,
}

impl<T: View> OnEventView<T> {
    /// Wraps the given view in a new OnEventView.
    pub fn new(view: T) -> Self {
        OnEventView {
            content: view,
            callbacks: HashMap::new(),
        }
    }

    /// Registers a callback when the given event is ignored by the child.
    ///
    /// Chainable variant.
    pub fn on_event<F, E: Into<Event>>(self, event: E, cb: F) -> Self
        where F: Fn(&mut Cursive) + 'static
    {
        self.with(|s| s.set_on_event(event, cb))
    }

    /// Registers a callback when the given event is ignored by the child.
    pub fn set_on_event<F, E: Into<Event>>(&mut self, event: E, cb: F)
        where F: Fn(&mut Cursive) + 'static
    {
        self.callbacks.insert(event.into(), Callback::from_fn(cb));
    }
}

impl<T: View> ViewWrapper for OnEventView<T> {
    wrap_impl!(self.content: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match self.content.on_event(event.clone()) {
            EventResult::Ignored => {
                match self.callbacks.get(&event) {
                    None => EventResult::Ignored,
                    Some(cb) => EventResult::Consumed(Some(cb.clone())),
                }
            }
            res => res,
        }
    }
}
