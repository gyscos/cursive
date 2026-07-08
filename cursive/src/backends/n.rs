//! Ncurses-specific backend.
//!
//! This module re-exports the ncurses backend from the `cursive-ncurses` crate.
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ncurses-backend")))]

pub use cursive_ncurses::ncurses;
pub use cursive_ncurses::Backend;
