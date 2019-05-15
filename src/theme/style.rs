use super::{Color, ColorStyle, ColorType, Effect, PaletteColor};
use enumset::{enum_set, EnumSet};

/// Combine a color and an effect.
///
/// Represents any transformation that can be applied to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// Effect to apply.
    ///
    /// `None` to keep using previous effects.
    pub effects: EnumSet<Effect>,

    /// Color style to apply.
    ///
    /// `None` to keep using the previous colors.
    pub color: Option<ColorStyle>,
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
            color: None,
        }
    }

    /// Returns a new `Style` by merging all given styles.
    ///
    /// Will use the last non-`None` color, and will combine all effects.
    pub fn merge(styles: &[Style]) -> Self {
        let mut color = None;
        let mut effects = EnumSet::new();

        for style in styles {
            if style.color.is_some() {
                color = style.color;
            }

            effects.insert_all(style.effects);
        }

        Style { color, effects }
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
            effects: enum_set!(effect),
            color: None,
        }
    }
}

impl From<ColorStyle> for Style {
    fn from(color: ColorStyle) -> Self {
        Style {
            effects: EnumSet::new(),
            color: Some(color),
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
