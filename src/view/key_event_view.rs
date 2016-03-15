use std::collections::HashMap;
use std::rc::Rc;

use Cursive;
use event::{Event, EventResult, ToEvent, Callback};
use super::{View, ViewWrapper};

/// A simple wrapper view that catches some ignored event from its child.
///
/// Events ignored by its child without a callback will stay ignored.
pub struct KeyEventView {
    content: Box<View>,
    callbacks: HashMap<Event, Rc<Callback>>,
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
    pub fn register<F, E: ToEvent>(mut self, event: E, cb: F) -> Self
        where F: Fn(&mut Cursive) + 'static
    {
        self.callbacks.insert(event.to_event(), Rc::new(Box::new(cb)));

        self
    }
}

impl ViewWrapper for KeyEventView {

    wrap_impl!(self.content);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match self.content.on_event(event) {
            EventResult::Ignored => match self.callbacks.get(&event) {
                None => EventResult::Ignored,
                Some(cb) => EventResult::Consumed(Some(cb.clone())),
            },
            res => res,
        }
    }

}
