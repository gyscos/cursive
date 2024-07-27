use crate::{
    event::{AnyCb, EventResult},
    view::{Selector, View, ViewNotFound},
    views::BoxedView,
};

/// Identifies a screen in the cursive root.
pub type ScreenId = usize;

/// A view that can switch between different screens.
pub struct ScreensView<V = BoxedView> {
    screens: Vec<V>,
    active_screen: ScreenId,
}

new_default!(ScreensView<V>);

impl<V> ScreensView<V> {
    /// Creates a new empty `ScreensView`.
    pub fn new() -> Self {
        ScreensView {
            screens: Vec::new(),
            active_screen: 0,
        }
    }

    /// Creates a new `ScreensView` with a single screen.
    pub fn single_screen(v: V) -> Self {
        ScreensView {
            screens: vec![v],
            active_screen: 0,
        }
    }

    /// Returns a reference to the currently active screen.
    ///
    /// Returns `None` if there is no active screen.
    pub fn screen(&self) -> Option<&V> {
        self.screens.get(self.active_screen)
    }

    /// Returns a mutable reference to the currently active screen.
    pub fn screen_mut(&mut self) -> Option<&mut V> {
        let id = self.active_screen;
        self.screens.get_mut(id)
    }

    /// Returns the id of the currently active screen.
    pub fn active_screen(&self) -> ScreenId {
        self.active_screen
    }

    /// Adds a new screen, and returns its ID.
    pub fn add_screen(&mut self, v: V) -> ScreenId {
        let res = self.screens.len();
        self.screens.push(v);
        res
    }

    /// Convenient method to create a new screen, and set it as active.
    pub fn add_active_screen(&mut self, v: V) -> ScreenId {
        let res = self.add_screen(v);
        self.set_active_screen(res);
        res
    }

    /// Sets the active screen. Panics if no such screen exist.
    pub fn set_active_screen(&mut self, screen_id: ScreenId) {
        if screen_id >= self.screens.len() {
            panic!(
                "Tried to set an invalid screen ID: {}, but only {} \
                 screens present.",
                screen_id,
                self.screens.len()
            );
        }
        self.active_screen = screen_id;
    }
}

impl ScreensView<crate::views::StackView> {
    /// Draws the background.
    ///
    /// This is mostly used internally by cursive. You probably just want
    /// `View::draw`.
    pub fn draw_bg(&self, printer: &crate::Printer) {
        if let Some(screen) = self.screen() {
            screen.draw_bg(printer);
        }
    }

    /// Draws the foreground.
    ///
    /// This is mostly used internally by cursive. You probably just want
    /// `View::draw`.
    pub fn draw_fg(&self, printer: &crate::Printer) {
        if let Some(screen) = self.screen() {
            screen.draw_fg(printer);
        }
    }
}

impl<V> crate::view::ViewWrapper for ScreensView<V>
where
    V: View,
{
    type V = V;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        self.screen().map(f)
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        self.screen_mut().map(f)
    }

    fn wrap_call_on_any(&mut self, selector: &Selector, callback: AnyCb) {
        for screen in &mut self.screens {
            screen.call_on_any(selector, callback);
        }
    }

    fn wrap_focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        for (i, child) in self.screens.iter_mut().enumerate() {
            if let Ok(res) = child.focus_view(selector) {
                self.active_screen = i;
                return Ok(res);
            }
        }

        Err(ViewNotFound)
    }
}

// TODO: blueprint?
