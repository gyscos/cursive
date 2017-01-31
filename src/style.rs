//! Handle text style.

use ::theme::{ColorStyle, Effect};

/// Defines the style with which text shall be printed.
#[derive(Clone, Copy, Default, Debug)]
pub struct Style {
    /// The color style to apply when printing text.
    pub color: Option<ColorStyle>,
    /// The effect to apply when printing text.
    pub effect: Effect
}

/// Correlates some text with a `Style` to print it with.
pub type StyledStr<'a> = (&'a str, Style);
