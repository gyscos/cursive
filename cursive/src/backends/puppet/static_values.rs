/// Some default values to Puppet backend.
use std::sync::LazyLock;

use crate::Vec2;
use crate::XY;
use crate::reexports::enumset::EnumSet;
use crate::theme::ColorPair;
use crate::theme::{Color, Effect};

use crate::backends::puppet::observed::*;

/// Default size for the puppet terminal.
pub static DEFAULT_SIZE: LazyLock<Vec2> = LazyLock::new(|| XY::<usize> { x: 120, y: 80 });

/// Default style for the puppet terminal.
pub static DEFAULT_OBSERVED_STYLE: LazyLock<ObservedStyle> = LazyLock::new(|| ObservedStyle {
    colors: ColorPair {
        front: Color::TerminalDefault,
        back: Color::TerminalDefault,
    },
    effects: EnumSet::<Effect>::empty(),
});
