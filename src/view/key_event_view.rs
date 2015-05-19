use std::collections::HashMap;
use std::rc::Rc;

use ::Cursive;
use event::{EventResult,Callback};
use super::{View,ViewPath,ViewWrapper};

/// A simple wrapper view that catches some ignored event from its child.
///
/// Events ignored by its child without a callback will stay ignored.
pub struct KeyEventView {
    content: Box<View>,
    callbacks: HashMap<i32, Rc<Callback>>,
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
    pub fn register<F>(mut self, key: i32, cb: F) -> Self
        where F: Fn(&mut Cursive, &ViewPath) + 'static
    {
        self.callbacks.insert(key, Rc::new(Box::new(cb)));

        self
    }
}

impl ViewWrapper for KeyEventView {

    wrap_impl!(content);

    fn wrap_on_key_event(&mut self, ch: i32) -> EventResult {
        match self.content.on_key_event(ch) {
            EventResult::Ignored => match self.callbacks.get(&ch) {
                None => EventResult::Ignored,
                Some(cb) => EventResult::Consumed(Some(cb.clone()), ViewPath::new()),
            },
            res => res,
        }
    }

}
