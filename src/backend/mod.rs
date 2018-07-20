//! Define backends using common libraries.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the `Backend` trait, as well as a few implementations
//! using some common libraries. Each of those included backends needs a
//! corresonding feature to be enabled.

use crossbeam_channel::{Receiver, Sender};

use event::Event;
use theme;
use vec::Vec2;

#[cfg(unix)]
mod resize;

pub mod dummy;

pub mod blt;
pub mod curses;
pub mod termion;

/// A request for input, sent to the backend.
pub enum InputRequest {
    /// The backend should respond immediately with an answer, possibly empty.
    Peek,
    /// The backend should block until input is available.
    Block,
}

/// Trait defining the required methods to be a backend.
pub trait Backend {
    // TODO: take `self` by value?
    // Or implement Drop?
    /// Prepares to close the backend.
    ///
    /// This should clear any state in the terminal.
    fn finish(&mut self);

    /// Starts a thread to collect input and send it to the given channel.
    ///
    /// `event_trigger` will receive a value before any event is needed.
    fn start_input_thread(
        &mut self, event_sink: Sender<Option<Event>>,
        input_request: Receiver<InputRequest>,
    ) {
        // Dummy implementation for some backends.
        let _ = event_sink;
        let _ = input_request;
    }

    /// Prepares the backend to collect input.
    ///
    /// This is only required for non-thread-safe backends like BearLibTerminal
    /// where we cannot collect input in a separate thread.
    fn prepare_input(&mut self, input_request: InputRequest) {
        // Dummy implementation for most backends.
        // Little trick to avoid unused variables.
        let _ = input_request;
    }

    /// Refresh the screen.
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
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair;

    /// Enables the given effect.
    fn set_effect(&self, effect: theme::Effect);

    /// Disables the given effect.
    fn unset_effect(&self, effect: theme::Effect);
}
