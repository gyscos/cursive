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
    #[must_use]
    pub fn merge(styles: &[Style]) -> Self {
        styles.iter().collect()
    }

    /// Returns a combination of `self` and `other`.
    #[must_use]
    pub fn combine<S>(self, other: S) -> Self
    where
        S: Into<Style>,
    {
        Self::merge(&[self, other.into()])
    }

    /// Uses `ColorType::InheritParent` for both front and background.
    pub fn inherit_parent() -> Self {
        ColorStyle::inherit_parent().into()
    }

    /// Style set by terminal before entering a Cursive program.
    pub fn terminal_default() -> Self {
        ColorStyle::terminal_default().into()
    }

    /// Application background, where no view is present.
    pub fn background() -> Self {
        ColorStyle::background().into()
    }

    /// Color used by view shadows. Only background matters.
    pub fn shadow() -> Self {
        ColorStyle::shadow().into()
    }

    /// Main text with default background.
    pub fn primary() -> Self {
        ColorStyle::primary().into()
    }

    /// Secondary text color, with default background.
    pub fn secondary() -> Self {
        ColorStyle::secondary().into()
    }

    /// Tertiary text color, with default background.
    pub fn tertiary() -> Self {
        ColorStyle::tertiary().into()
    }

    /// Title text color with default background.
    pub fn title_primary() -> Self {
        ColorStyle::title_primary().into()
    }

    /// Alternative color for a title.
    pub fn title_secondary() -> Self {
        ColorStyle::title_secondary().into()
    }

    /// Returns a highlight style.
    pub fn highlight() -> Self {
        Style {
            color: ColorStyle::highlight().invert(),
            effects: enumset::enum_set!(Effect::Reverse),
        }
    }

    /// Returns an inactive highlight style.
    pub fn highlight_inactive() -> Self {
        Style {
            color: ColorStyle::highlight_inactive().invert(),
            effects: enumset::enum_set!(Effect::Reverse),
        }
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

        Style { effects, color }
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
