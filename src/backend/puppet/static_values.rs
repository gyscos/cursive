/// Some default values to Puppet backend.
#[allow(missing_docs)]

use lazy_static::lazy_static;

use crate::theme::{Color, Effect};
use crate::theme::ColorPair;
use crate::vec::Vec2;
use crate::XY;
use enumset::EnumSet;

use crate::backend::puppet::observed::*;

lazy_static! {
    pub static ref DEFAULT_SIZE: Vec2 = XY::<usize> { x: 120, y: 80 };
    pub static ref DEFAULT_OBSERVED_STYLE: ObservedStyle = ObservedStyle {
        colors: ColorPair {
            front: Color::TerminalDefault,
            back: Color::TerminalDefault,
        },
        effects: EnumSet::<Effect>::empty(),
    };
}
