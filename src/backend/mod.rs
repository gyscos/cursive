use event;
use theme;

#[cfg(feature = "termion")]
mod termion;
#[cfg(feature = "bear-lib-terminal")]
mod blt;
#[cfg(any(feature = "ncurses", feature = "pancurses"))]
mod curses;

#[cfg(feature = "bear-lib-terminal")]
pub use self::blt::*;
#[cfg(any(feature = "ncurses", feature = "pancurses"))]
pub use self::curses::*;
#[cfg(feature = "termion")]
pub use self::termion::*;

pub trait Backend {
    fn init() -> Self;
    // TODO: take `self` by value?
    // Or implement Drop?
    fn finish(&mut self);

    fn refresh(&mut self);

    fn has_colors(&self) -> bool;
    fn screen_size(&self) -> (usize, usize);

    /// Main input method
    fn poll_event(&self) -> event::Event;

    /// Main method used for printing
    fn print_at(&self, (usize, usize), &str);
    fn clear(&self, color: theme::Color);

    fn set_refresh_rate(&mut self, fps: u32);
    // TODO: unify those into a single method?
    fn with_color<F: FnOnce()>(&self, colors: theme::ColorPair, f: F);
    fn with_effect<F: FnOnce()>(&self, effect: theme::Effect, f: F);
}
