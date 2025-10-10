//! Define backends using common libraries.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the [`Backend`] trait, as well as a few implementations
//! using some common libraries. Each of those included backends needs a
//! corresponding feature to be enabled.
//!
//! [`Backend`]: ../backend/trait.Backend.html
#[cfg(unix)]
mod resize;

pub mod blt;
pub mod crossterm;
pub mod curses;
pub mod puppet;
pub mod termion;

#[allow(dead_code)]
fn boxed(e: impl std::error::Error + 'static) -> Box<dyn std::error::Error> {
    Box::new(e)
}

/// Tries to initialize the default backend.
///
/// Will use the first backend enabled from the list:
/// * BearLibTerminal
/// * Termion
/// * Crossterm
/// * Pancurses
/// * Ncurses
/// * Dummy
pub fn try_default() -> Result<Box<dyn cursive_core::backend::Backend>, Box<dyn std::error::Error>>
{
    cfg_if::cfg_if! {
        if #[cfg(feature = "blt-backend")] {
            Ok(blt::Backend::init())
        } else if #[cfg(feature = "termion-backend")] {
            termion::Backend::init().map_err(boxed)
        } else if #[cfg(feature = "pancurses-backend")] {
            curses::pan::Backend::init().map_err(boxed)
        } else if #[cfg(feature = "ncurses-backend")] {
            curses::n::Backend::init().map_err(boxed)
        } else if #[cfg(feature = "crossterm-backend")] {
            crossterm::Backend::init().map_err(boxed)
        } else {
            log::warn!("No built-it backend, falling back to Dummy backend.");
            Ok(cursive_core::backend::Dummy::init())
        }
    }
}
