use crate::direction::Direction;
use crate::event::{Event, EventResult, Key};
use crate::view::{View, ViewWrapper};

/// Adds circular focus to a wrapped view.
///
/// Wrap a view in `CircularFocus` to enable wrap-around focus
/// (when the focus exits this view, it will come back the other side).
///
/// It can be configured to wrap Tab (and Shift+Tab) keys, and/or Arrow keys.
pub struct CircularFocus<T: View> {
    view: T,
    wrap_tab: bool,
    wrap_arrows: bool,
}

impl<T: View> CircularFocus<T> {
    /// Creates a new `CircularFocus` around the given view.
    ///
    /// If `wrap_tab` is true, Tab keys will cause focus to wrap around.
    /// If `wrap_arrows` is true, Arrow keys will cause focus to wrap around.
    pub fn new(view: T, wrap_tab: bool, wrap_arrows: bool) -> Self {
        CircularFocus {
            view,
            wrap_tab,
            wrap_arrows,
        }
    }

    /// Creates a new `CircularFocus` view which will wrap around Tab-based
    /// focus changes.
    ///
    /// Whenever `Tab` would leave focus from this view, the focus will be
    /// brought back to the beginning of the view.
    pub fn wrap_tab(view: T) -> Self {
        CircularFocus::new(view, true, false)
    }

    /// Creates a new `CircularFocus` view which will wrap around Tab-based
    /// focus changes.
    ///
    /// Whenever an arrow key
    pub fn wrap_arrows(view: T) -> Self {
        CircularFocus::new(view, false, true)
    }

    /// Returns `true` if Tab key cause focus to wrap around.
    pub fn wraps_tab(&self) -> bool {
        self.wrap_tab
    }

    /// Returns `true` if Arrow keys cause focus to wrap around.
    pub fn wraps_arrows(&self) -> bool {
        self.wrap_arrows
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for CircularFocus<T> {
    wrap_impl!(self.view: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match (self.view.on_event(event.clone()), event) {
            (EventResult::Ignored, Event::Key(Key::Tab)) if self.wrap_tab => {
                // Focus comes back!
                if self.view.take_focus(Direction::front()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (EventResult::Ignored, Event::Shift(Key::Tab))
                if self.wrap_tab =>
            {
                // Focus comes back!
                if self.view.take_focus(Direction::back()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (EventResult::Ignored, Event::Key(Key::Right))
                if self.wrap_arrows =>
            {
                // Focus comes back!
                if self.view.take_focus(Direction::left()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (EventResult::Ignored, Event::Key(Key::Left))
                if self.wrap_arrows =>
            {
                // Focus comes back!
                if self.view.take_focus(Direction::right()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (EventResult::Ignored, Event::Key(Key::Up))
                if self.wrap_arrows =>
            {
                // Focus comes back!
                if self.view.take_focus(Direction::down()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (EventResult::Ignored, Event::Key(Key::Down))
                if self.wrap_arrows =>
            {
                // Focus comes back!
                if self.view.take_focus(Direction::up()) {
                    EventResult::Consumed(None)
                } else {
                    EventResult::Ignored
                }
            }
            (other, _) => other,
        }
    }
}
