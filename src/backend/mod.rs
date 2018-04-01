use event;
use theme;

pub mod dummy;

/// Backend using the pure-rust termion library.
#[cfg(feature = "termion")]
pub mod termion;

/// Backend using BearLibTerminal
#[cfg(feature = "bear-lib-terminal")]
pub mod blt;

#[cfg(any(feature = "ncurses", feature = "pancurses"))]
pub mod curses;

pub trait Backend {
    // TODO: take `self` by value?
    // Or implement Drop?
    fn finish(&mut self);

    fn refresh(&mut self);

    fn has_colors(&self) -> bool;
    fn screen_size(&self) -> (usize, usize);

    /// Main input method
    fn poll_event(&mut self) -> event::Event;

    /// Main method used for printing
    fn print_at(&self, (usize, usize), &str);
    fn clear(&self, color: theme::Color);

    fn set_refresh_rate(&mut self, fps: u32);

    // This sets the Colours and returns the previous colours
    // to allow you to set them back when you're done.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair;

    fn set_effect(&self, effect: theme::Effect);
    fn unset_effect(&self, effect: theme::Effect);
}
