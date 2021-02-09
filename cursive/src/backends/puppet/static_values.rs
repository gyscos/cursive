/// Some default values to Puppet backend.
use lazy_static::lazy_static;

use crate::reexports::enumset::EnumSet;
use crate::theme::ColorPair;
use crate::theme::{Color, Effect};
use crate::Vec2;
use crate::XY;

use crate::backends::puppet::observed::*;

lazy_static! {
    /// Default size for the puppet terminal.
    pub static ref DEFAULT_SIZE: Vec2 = XY::<usize> { x: 120, y: 80 };

    /// Default style for the puppet terminal.
    pub static ref DEFAULT_OBSERVED_STYLE: ObservedStyle = ObservedStyle {
        colors: ColorPair {
            front: Color::TerminalDefault,
            back: Color::TerminalDefault,
        },
        effects: EnumSet::<Effect>::empty(),
    };
}
