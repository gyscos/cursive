use crate::{
    direction::Direction,
    event::{Event, EventResult, Key},
    view::{View, ViewWrapper},
    With,
};

/// Adds circular focus to a wrapped view.
///
/// Wrap a view in `CircularFocus` to enable wrap-around focus
/// (when the focus exits this view, it will come back the other side).
///
/// It can be configured to wrap Tab (and Shift+Tab) keys, and/or Arrow keys.
pub struct CircularFocus<T: View> {
    view: T,
    wrap_tab: bool,
    wrap_up_down: bool,
    wrap_left_right: bool,
}

impl<T: View> CircularFocus<T> {
    /// Creates a new `CircularFocus` around the given view.
    ///
    /// Does not wrap anything by default,
    /// so you'll want to call one of the setters.
    pub fn new(view: T) -> Self {
        CircularFocus {
            view,
            wrap_tab: false,
            wrap_left_right: false,
            wrap_up_down: false,
        }
    }

    /// Returns `true` if Tab key cause focus to wrap around.
    pub fn wraps_tab(&self) -> bool {
        self.wrap_tab
    }

    /// Returns `true` if Arrow keys cause focus to wrap around.
    pub fn wraps_arrows(&self) -> bool {
        self.wrap_left_right && self.wrap_up_down
    }

    /// Return `true` if left/right keys cause focus to wrap around.
    pub fn wraps_left_right(&self) -> bool {
        self.wrap_left_right
    }

    /// Return `true` if up/down keys cause focus to wrap around.
    pub fn wraps_up_down(&self) -> bool {
        self.wrap_up_down
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    #[must_use]
    pub fn wrap_arrows(self) -> Self {
        self.with_wrap_arrows(true)
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    #[must_use]
    pub fn wrap_up_down(self) -> Self {
        self.with_wrap_up_down(true)
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    #[must_use]
    pub fn wrap_left_right(self) -> Self {
        self.with_wrap_left_right(true)
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_wrap_tab(self, wrap_tab: bool) -> Self {
        self.with(|s| s.set_wrap_tab(wrap_tab))
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn wrap_tab(self) -> Self {
        self.with_wrap_tab(true)
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    #[must_use]
    pub fn with_wrap_left_right(self, wrap_left_right: bool) -> Self {
        self.with(|s| s.set_wrap_left_right(wrap_left_right))
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    #[must_use]
    pub fn with_wrap_up_down(self, wrap_up_down: bool) -> Self {
        self.with(|s| s.set_wrap_up_down(wrap_up_down))
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    #[must_use]
    pub fn with_wrap_arrows(self, wrap_arrows: bool) -> Self {
        self.with(|s| s.set_wrap_arrows(wrap_arrows))
    }

    /// Make this view now wrap focus around when the Tab key is pressed.
    pub fn set_wrap_tab(&mut self, wrap_tab: bool) {
        self.wrap_tab = wrap_tab;
    }

    /// Make this view now wrap focus around when arrow keys are pressed.
    pub fn set_wrap_arrows(&mut self, wrap_arrows: bool) {
        self.wrap_left_right = wrap_arrows;
        self.wrap_up_down = wrap_arrows;
    }

    /// Make this view now wrap focus around when the up/down keys are pressed.
    pub fn set_wrap_up_down(&mut self, wrap_up_down: bool) {
        self.wrap_up_down = wrap_up_down;
    }

    /// Make this view now wrap focus around when the left/right keys are pressed.
    pub fn set_wrap_left_right(&mut self, wrap_left_right: bool) {
        self.wrap_left_right = wrap_left_right;
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for CircularFocus<T> {
    wrap_impl!(self.view: T);

    fn wrap_on_event(&mut self, event: Event) -> EventResult {
        match (self.view.on_event(event.clone()), event) {
            (EventResult::Ignored, Event::Key(Key::Tab)) if self.wrap_tab => {
                // Focus comes back!
                self.view
                    .take_focus(Direction::front())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Shift(Key::Tab))
                if self.wrap_tab =>
            {
                // Focus comes back!
                self.view
                    .take_focus(Direction::back())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Right))
                if self.wrap_left_right =>
            {
                // Focus comes back!
                self.view
                    .take_focus(Direction::left())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Left))
                if self.wrap_left_right =>
            {
                // Focus comes back!
                self.view
                    .take_focus(Direction::right())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Up))
                if self.wrap_up_down =>
            {
                // Focus comes back!
                self.view
                    .take_focus(Direction::down())
                    .unwrap_or(EventResult::Ignored)
            }
            (EventResult::Ignored, Event::Key(Key::Down))
                if self.wrap_up_down =>
            {
                // Focus comes back!
                self.view
                    .take_focus(Direction::up())
                    .unwrap_or(EventResult::Ignored)
            }
            (other, _) => other,
        }
    }
}
