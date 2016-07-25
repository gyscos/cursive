use std::collections::HashMap;

use Cursive;
use event::{Callback, Event, EventResult};
use super::{View, ViewWrapper};

/// A simple wrapper view that catches some ignored event from its child.
///
/// If the event doesn't have a corresponding callback, it will stay ignored.
///
/// # Examples
///
/// ```
/// # use cursive::prelude::*;
/// let view = KeyEventView::new(TextView::new("This view has an event!"))
///                         .register('q', |s| s.quit())
///                         .register(Key::Esc, |s| s.quit());
/// ```
pub struct KeyEventView {
    content: Box<View>,
    callbacks: HashMap<Event, Callback>,
}

impl KeyEventView {
    /// Wraps the given view in a new KeyEventView.
    pub fn new<V: View + 'static>(view: V) -> Self {
        KeyEventView {
            content: Box::new(view),
            callbacks: HashMap::new(),
        }
    }

    /// Registers a callback when the given key is ignored by the child.
    pub fn register<F, E: Into<Event>>(mut self, event: E, cb: F) -> Self
        where F: Fn(&mut Cursive) + 'static
    {
        self.callbacks.insert(event.into(), Callback::from_fn(cb));

        self
    }
}

impl ViewWrapper for KeyEventView {
    wrap_impl!(self.content);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match self.content.on_event(event) {
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
