//! Define backends using common libraries.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the `Backend` trait, as well as a few implementations
//! using some common libraries. Each of those included backends needs a
//! corresonding feature to be enabled.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use signal_hook::iterator::Signals;

use event::Event;
use theme;
use vec::Vec2;

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

/// This starts a new thread to listen for SIGWINCH signals
///
/// As long as `resize_running` is true, it will listen for SIGWINCH, and,
/// when detected, it wil set `needs_resize` to true and send an event to
/// `resize_sender`. It will also consume an event from `resize_requests`
/// afterward, to keep the balance in the force.
#[cfg(unix)]
fn start_resize_thread(
    signals: Signals, resize_sender: Sender<Option<Event>>,
    resize_requests: Receiver<InputRequest>, resize_running: Arc<AtomicBool>,
    needs_resize: Option<Arc<AtomicBool>>,
) {
    thread::spawn(move || {
        // This thread will listen to SIGWINCH events and report them.
        while resize_running.load(Ordering::Relaxed) {
            // We know it will only contain SIGWINCH signals, so no need to check.
            if signals.wait().count() > 0 {
                // Tell ncurses about the new terminal size.
                // Well, do the actual resizing later on, in the main thread.
                // Ncurses isn't really thread-safe so calling resize_term() can crash
                // other calls like clear() or refresh().
                if let Some(ref needs_resize) = needs_resize {
                    needs_resize.store(true, Ordering::Relaxed);
                }

                resize_sender.send(Some(Event::WindowResize));
                // We've sent the message.
                // This means Cursive was listening, and will now soon be sending a new request.
                // This means the input thread accepted a request, but hasn't sent a message yet.
                // So we KNOW the input thread is not waiting for a new request.

                // We sent an event for free, so pay for it now by consuming a request
                while let Some(InputRequest::Peek) = resize_requests.recv() {
                    // At this point Cursive will now listen for input.
                    // There is a chance the input thread will send his event before us.
                    // But without some extra atomic flag, it'd be hard to know.
                    // So instead, keep sending `None`

                    // Repeat until we receive a blocking call
                    resize_sender.send(None);
                }
            }
        }
    });
}
