use std::iter::FromIterator;

use super::{Color, ColorStyle, ColorType, Effect, PaletteColor};
use enumset::EnumSet;

/// Combine a color and an effect.
///
/// Represents any transformation that can be applied to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    /// Effect to apply.
    ///
    /// `None` to keep using previous effects.
    pub effects: EnumSet<Effect>,

    /// Color style to apply.
    ///
    /// `None` to keep using the previous colors.
    pub color: ColorStyle,
}

impl Default for Style {
    fn default() -> Self {
        Self::none()
    }
}

impl Style {
    /// Returns a new `Style` that doesn't apply anything.
    pub fn none() -> Self {
        Style {
            effects: EnumSet::new(),
            color: ColorStyle::inherit_parent(),
        }
    }

    /// Returns a new `Style` by merging all given styles.
    ///
    /// Will use the last non-`None` color, and will combine all effects.
    pub fn merge(styles: &[Style]) -> Self {
        styles.iter().collect()
    }

    /// Returns a combination of `self` and `other`.
    pub fn combine<S>(self, other: S) -> Self
    where
        S: Into<Style>,
    {
        Self::merge(&[self, other.into()])
    }
}

impl From<Effect> for Style {
    fn from(effect: Effect) -> Self {
        Style {
            effects: EnumSet::only(effect),
            color: ColorStyle::inherit_parent(),
        }
    }
}

impl From<ColorStyle> for Style {
    fn from(color: ColorStyle) -> Self {
        Style {
            effects: EnumSet::new(),
            color,
        }
    }
}

impl From<Color> for Style {
    fn from(color: Color) -> Self {
        ColorStyle::from(color).into()
    }
}

impl From<PaletteColor> for Style {
    fn from(color: PaletteColor) -> Self {
        ColorStyle::from(color).into()
    }
}

impl From<ColorType> for Style {
    fn from(color: ColorType) -> Self {
        ColorStyle::from(color).into()
    }
}

/// Creates a new `Style` by merging all given styles.
///
/// Will use the last non-`None` color, and will combine all effects.
impl<'a> FromIterator<&'a Style> for Style {
    fn from_iter<I: IntoIterator<Item = &'a Style>>(iter: I) -> Style {
        let mut color = ColorStyle::inherit_parent();
        let mut effects = EnumSet::new();

        for style in iter {
            color = ColorStyle::merge(color, style.color);
            effects.insert_all(style.effects);
        }

        Style { color, effects }
    }
}

/// Creates a new `Style` by merging all given styles.
///
/// Will use the last non-`None` color, and will combine all effects.
impl<T: Into<Style>> FromIterator<T> for Style {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Style {
        iter.into_iter().map(Into::into).collect()
    }
}
