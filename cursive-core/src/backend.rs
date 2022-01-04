//! Define the backend trait for actual terminal interaction.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the [`Backend`] trait, to be implemented by actual
//! types, usually using third-party libraries.
//!
//! [`Backend`]: trait.Backend.html

use crate::event::Event;
use crate::theme;
use crate::Vec2;
use unicode_width::UnicodeWidthStr;

/// Trait defining the required methods to be a backend.
///
/// A backend is the interface between the abstract view tree and the actual
/// input/output, like a terminal.
///
/// It usually delegates the work to a terminal-handling library like ncurses
/// or termion, or it can entirely simulate a terminal and show it as a
/// graphical window (`BearLibTerminal`).
///
/// When creating a new cursive tree with [`Cursive::new()`][1], you will need to
/// provide a backend initializer - usually their `init()` function.
///
/// Backends are responsible for handling input and converting it to [`Event`].
/// Input must be non-blocking, it will be polled regularly.
///
/// [1]: ../struct.Cursive.html#method.new
/// [`Event`]: ../event/enum.Event.html
pub trait Backend {
    /// Polls the backend for any input.
    ///
    /// Should return immediately:
    /// * `None` if no event is currently available.
    /// * `Some(event)` for each event to process.
    fn poll_event(&mut self) -> Option<Event>;

    /// Sets the title for the backend.
    ///
    /// This usually sets the terminal window title.
    fn set_title(&mut self, title: String);

    /// Refresh the screen.
    ///
    /// This will be called each frame after drawing has been done.
    ///
    /// A backend could, for example, buffer any print command, and apply
    /// everything when refresh() is called.
    fn refresh(&mut self);

    /// Should return `true` if this backend supports colors.
    fn has_colors(&self) -> bool;

    /// Returns the screen size.
    fn screen_size(&self) -> Vec2;

    /// Main method used for printing
    fn print_at(&self, pos: Vec2, text: &str);

    /// Efficient method to print repetitions of the same text.
    ///
    /// Usually used to draw horizontal lines/borders.
    ///
    /// It is a small optimization to avoid moving the cursor after each step.
    fn print_at_rep(&self, pos: Vec2, repetitions: usize, text: &str) {
        if repetitions > 0 {
            self.print_at(pos, text);

            let width = text.width();
            let mut pos = pos;
            let mut dupes_left = repetitions - 1;

            while dupes_left > 0 {
                pos = pos.saturating_add((width, 0));
                self.print_at(pos, text);
                dupes_left -= 1;
            }
        }
    }

    /// Clears the screen with the given color.
    fn clear(&self, color: theme::Color);

    /// Starts using a new color.
    ///
    /// This should return the previously active color.
    ///
    /// Any call to `print_at` from now on should use the given color.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair;

    /// Enables the given effect.
    ///
    /// Any call to `print_at` from now on should use the given effect.
    fn set_effect(&self, effect: theme::Effect);

    /// Disables the given effect.
    fn unset_effect(&self, effect: theme::Effect);

    /// Returns a name to identify the backend.
    ///
    /// Mostly used for debugging.
    fn name(&self) -> &str {
        "unknown"
    }
}

/// Dummy backend that does nothing and immediately exits.
///
/// Mostly used for testing.
pub struct Dummy;

impl Dummy {
    /// Creates a new dummy backend.
    pub fn init() -> Box<dyn Backend>
    where
        Self: Sized,
    {
        Box::new(Dummy)
    }
}

impl Backend for Dummy {
    fn name(&self) -> &str {
        "dummy"
    }

    fn set_title(&mut self, _title: String) {}

    fn refresh(&mut self) {}

    fn has_colors(&self) -> bool {
        false
    }

    fn screen_size(&self) -> Vec2 {
        (1, 1).into()
    }
    fn poll_event(&mut self) -> Option<Event> {
        Some(Event::Exit)
    }

    fn print_at(&self, _: Vec2, _: &str) {}

    fn print_at_rep(&self, _pos: Vec2, _repetitions: usize, _text: &str) {}

    fn clear(&self, _: theme::Color) {}

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair {
        // TODO: actually save a stack of colors?
        colors
    }

    fn set_effect(&self, _: theme::Effect) {}
    fn unset_effect(&self, _: theme::Effect) {}
}
