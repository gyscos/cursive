//! Define backends using common libraries.
//!
//! Cursive doesn't print anything by itself: it delegates this job to a
//! backend library, which handles all actual input and output.
//!
//! This module defines the `Backend` trait, as well as a few implementations
//! using some common libraries. Each of those included backends needs a
//! corresonding feature to be enabled.
#[cfg(unix)]
mod resize;

pub mod dummy;

pub mod blt;
pub mod crossterm;
pub mod curses;
pub mod puppet;
pub mod termion;
