//! User-input events and their effects.

use std::rc::Rc;

use ::Cursive;
use view::ViewPath;

/// Callback is a function that can be triggered by an event.
/// It has a mutable access to the cursive root.
pub type Callback = Box<Fn(&mut Cursive, &ViewPath)>;

/// Answer to an event notification.
/// The event can be consumed or ignored.
pub enum EventResult {
    /// The event was ignored. The parent can keep handling it.
    Ignored,
    /// The event was consumed. An optionnal callback to run is attached.
    Consumed(Option<Rc<Callback>>, ViewPath),
}

impl EventResult {
    pub fn callback(cb: Rc<Callback>) -> Self {
        EventResult::Consumed(Some(cb), ViewPath::new())
    }

    pub fn consume() -> Self {
        EventResult::Consumed(None, ViewPath::new())
    }
}
