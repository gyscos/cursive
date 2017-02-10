//! Handle text style.

use ::theme::{ColorStyle, Effect};
use ::std::borrow::Cow;

/// Defines the style with which text shall be printed.
#[derive(Clone, Copy, Default, Debug)]
pub struct Style {
    /// The color style to apply when printing text.
    pub color: Option<ColorStyle>,
    /// The effect to apply when printing text.
    pub effect: Effect
}

/// Correlates some text with a `Style` to print it with.
pub type StyledString<'a> = (Cow<'a, str>, Style);
