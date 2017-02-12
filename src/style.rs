//! Handle text style.

use ::theme::{ColorStyle, Effect};
use ::std::borrow::Cow;

/// Defines the style with which text shall be printed.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Style {
    /// The color style to apply when printing text.
    pub color: Option<ColorStyle>,
    /// The effect to apply when printing text.
    pub effect: Effect
}

/// Correlates some text with a `Style` to print it with.
pub type StyledString<'a> = (Cow<'a, str>, Style);

/// Styled text, in whatever representation.
pub trait StyledText {
    /// Returns the unstyled text.
    fn to_plain(&self) -> String;

    /// Returns the length of the text.
    ///
    /// Equal to `self.to_plain().len()` but should not allocate.
    ///
    /// *Not named `len()` to avoid shadowing `Vec::len()`.*
    fn len_plain(&self) -> usize;
}

impl<'a, T: AsRef<str>> StyledText for &'a [(T, Style)] {
    fn to_plain(&self) -> String {
        let mut plain = String::with_capacity(self.len_plain());
        for part in *self {
            plain.push_str(part.0.as_ref());
        }
        plain
    }

    fn len_plain(&self) -> usize {
        self.iter().fold(0, |len, it| len + it.0.as_ref().len())
    }
}
