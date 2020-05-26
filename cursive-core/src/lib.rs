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

macro_rules! new_default(
    ($c:ident<$t:ident>) => {
        impl<$t> Default for $c<$t> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ($c:ty) => {
        impl Default for $c {
            fn default() -> Self {
                Self::new()
            }
        }
    }
);

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
mod printer;
mod rect;
mod with;
mod xy;

mod div;

pub use self::cursive::{CbSink, Cursive, ScreenId};
pub use self::printer::Printer;
pub use self::rect::Rect;
pub use self::vec::Vec2;
pub use self::view::View;
pub use self::with::With;
pub use self::xy::XY;
