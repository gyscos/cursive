//! # Cursive-core
//!
//! This library defines the core components for the Cursive TUI.
//!
//! The main purpose of `cursive-core` is to write third-party libraries to work with Cursive.
//!
//! If you are building an end-user application, then [`cursive`] is probably what you want.
//!
//! [`cursive`]: https://docs.rs/cursive
#![deny(missing_docs)]
#![cfg_attr(feature = "doc-cfg", feature(doc_cfg))]

macro_rules! new_default(
    ($c:ident<$t:ident>) => {
        impl<$t> Default for $c<$t> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ($c:ident) => {
        impl Default for $c {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ($c:ident<$t:ident: Default>) => {
        impl <$t> Default for $c<$t>
        where $t: Default {
            fn default() -> Self {
                Self::new($t::default())
            }
        }
    };
);

/// Re-export crates used in the public API
pub mod reexports {
    pub use ahash;
    pub use crossbeam_channel;
    pub use enumset;
    pub use log;
    pub use time;

    #[cfg(feature = "toml")]
    pub use toml;
}

#[macro_use]
pub mod utils;
#[macro_use]
pub mod view;
#[macro_use]
pub mod views;

pub mod align;
pub mod backend;
pub mod direction;
pub mod event;
pub mod logger;
pub mod menu;
pub mod theme;
pub mod traits;
pub mod vec;

mod cursive;
mod cursive_run;
mod dump;
mod printer;
mod rect;
mod with;
mod xy;

mod div;

pub use self::cursive::{CbSink, Cursive, ScreenId};
pub use self::cursive_run::CursiveRunner;
pub use self::dump::Dump;
pub use self::printer::Printer;
pub use self::rect::Rect;
pub use self::vec::Vec2;
pub use self::view::View;
pub use self::with::With;
pub use self::xy::XY;
