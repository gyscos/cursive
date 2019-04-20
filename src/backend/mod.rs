//! Define backends using common libraries.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the `Backend` trait, as well as a few implementations
//! using some common libraries. Each of those included backends needs a
//! corresonding feature to be enabled.

use crate::event::Event;
use crate::theme;
use crate::vec::Vec2;

#[cfg(unix)]
mod resize;

pub mod dummy;

pub mod blt;
pub mod curses;
pub mod termion;
pub mod puppet;

/// Trait defining the required methods to be a backend.
///
/// A backend is the interface between the abstract view tree and the actual
/// input/output, like a terminal.
///
/// It usually delegates the work to a terminal-handling library like ncurses
/// or termion, or it can entirely simulate a terminal and show it as a
/// graphical window (`BearLibTerminal`).
///
/// When creating a new cursive tree with `Cursive::new()`, you will need to
/// provide a backend initializer - usually their `init()` function.
///
/// Backends are responsible for handling input and converting it to `Event`. Input must be
/// non-blocking, it will be polled regularly.
pub trait Backend {
    /// Polls the backend for any input.
    ///
    /// Should return immediately.
    fn poll_event(&mut self) -> Option<Event>;

    // TODO: take `self` by value?
    // Or implement Drop?
    /// Prepares to close the backend.
    ///
    /// This should clear any state in the terminal.
    fn finish(&mut self);

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
}
