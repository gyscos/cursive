use std::iter::FromIterator;
use std::str::FromStr;

use super::{
    Color, ColorPair, ColorStyle, ColorType, ConcreteEffects, Effect, Effects, Palette,
    PaletteColor, PaletteStyle,
};
use enumset::EnumSet;

/// Combine a color and effects.
///
/// Represents any transformation that can be applied to text.
///
/// This is a "abstract" style, which can depend on the current theme, or on the previously active
/// style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    /// Effects to apply.
    pub effects: Effects,

    /// Color style to apply.
    pub color: ColorStyle,
}

/// Combine a concrete color and effects.
///
/// This is a rendered version of `Style` or `StyleType`, which does not depend on the current
/// theme or the previously active style.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct ConcreteStyle {
    /// Effect to apply.
    pub effects: ConcreteEffects,

    /// Color style to apply.
    pub color: ColorPair,
}

impl ConcreteStyle {
    /// Return a new concrete style that uses the terminal default colors.
    pub const fn terminal_default() -> Self {
        ConcreteStyle {
            effects: EnumSet::empty(),
            color: ColorPair::terminal_default(),
        }
    }
}

impl Style {
    /// Returns a new `Style` that doesn't apply anything.
    ///
    /// Same as [`Style::inherit_parent()`].
    pub const fn none() -> Self {
        Self::inherit_parent()
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

    /// Create a new `Style` from a single `ColorStyle` and no effect.
    pub const fn from_color_style(color: ColorStyle) -> Self {
        Style {
            effects: Effects::empty(),
            color,
        }
    }

    /// Uses `ColorType::InheritParent` for both front and background.
    pub const fn inherit_parent() -> Self {
        Self::from_color_style(ColorStyle::inherit_parent())
    }

    /// Style set by terminal before entering a Cursive program.
    pub const fn terminal_default() -> Self {
        Self::from_color_style(ColorStyle::terminal_default())
    }

    /// Application background, where no view is present.
    pub const fn background() -> Self {
        Self::from_color_style(ColorStyle::background())
    }

    /// Color used by view shadows. Only background matters.
    pub const fn shadow() -> Self {
        Self::from_color_style(ColorStyle::shadow())
    }

    /// Style used for views.
    pub const fn view() -> Self {
        Self::from_color_style(ColorStyle::view())
    }

    /// Main text with default background.
    pub const fn primary() -> Self {
        Self::from_color_style(ColorStyle::primary())
    }

    /// Secondary text color, with default background.
    pub const fn secondary() -> Self {
        Self::from_color_style(ColorStyle::secondary())
    }

    /// Tertiary text color, with default background.
    pub const fn tertiary() -> Self {
        Self::from_color_style(ColorStyle::tertiary())
    }

    /// Title text color with default background.
    pub const fn title_primary() -> Self {
        Self::from_color_style(ColorStyle::title_primary())
    }

    /// Alternative color for a title.
    pub const fn title_secondary() -> Self {
        Self::from_color_style(ColorStyle::title_secondary())
    }

    /// Returns a highlight style.
    pub const fn highlight() -> Self {
        Style {
            color: ColorStyle::highlight().invert(),
            effects: Effects::only(Effect::Reverse),
        }
    }

    /// Returns an inactive highlight style.
    pub const fn highlight_inactive() -> Self {
        Style {
            color: ColorStyle::highlight_inactive().invert(),
            effects: Effects::only(Effect::Reverse),
        }
    }

    /// Parse a toml entry
    #[cfg(feature = "toml")]
    pub(crate) fn parse(table: &toml::Value) -> Option<Self> {
        let table = table.as_table()?;
        let mut effects = Effects::empty();

        for effect in table.get("effects")?.as_array()? {
            let effect = effect.as_str()?.parse().ok()?;
            effects.insert(effect);
        }

        let color = ColorStyle::parse(table)?;

        Some(Style { effects, color })
    }

    /// Resolve a style to a concrete style.
    pub fn resolve(&self, palette: &Palette, previous: ConcreteStyle) -> ConcreteStyle {
        ConcreteStyle {
            effects: self.effects.resolve(previous.effects),
            color: self.color.resolve(palette, previous.color),
        }
    }
}

fn parse_single_style(s: &str) -> Result<Style, super::NoSuchColor> {
    if let Some(s) = s.strip_prefix("back.") {
        if let Ok(back) = s.parse::<ColorType>() {
            return Ok(ColorStyle::back(back).into());
        }
    }

    if let Ok(front) = s.parse::<ColorType>() {
        return Ok(front.into());
    }

    if let Ok(effect) = s.parse::<Effect>() {
        return Ok(effect.into());
    }

    Err(super::NoSuchColor)
}

impl FromStr for Style {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split('+').map(parse_single_style).collect()
    }
}

impl From<Effects> for Style {
    fn from(effects: Effects) -> Self {
        Style {
            effects,
            color: ColorStyle::inherit_parent(),
        }
    }
}

impl From<Effect> for Style {
    fn from(effect: Effect) -> Self {
        Style {
            effects: Effects::only(effect),
            color: ColorStyle::inherit_parent(),
        }
    }
}

impl From<ColorStyle> for Style {
    fn from(color: ColorStyle) -> Self {
        Style {
            effects: Effects::default(),
            color,
        }
    }
}

impl From<ColorPair> for Style {
    fn from(color: ColorPair) -> Self {
        ColorStyle::from(color).into()
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

/// Returns an direct Style that just inherits the parent color.
impl Default for StyleType {
    fn default() -> Self {
        Style::default().into()
    }
}

/// Type of style to apply to some text.
///
/// Can be either an entry in the style palette, or a direct explicit style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleType {
    /// References a style from the palette.
    Palette(PaletteStyle),

    /// A direct style.
    Style(Style),
}

impl StyleType {
    /// Given a palette, resolve `self` to a concrete style.
    pub fn resolve(self, palette: &Palette) -> Style {
        match self {
            StyleType::Style(style) => style,
            StyleType::Palette(style) => style.resolve(palette),
        }
    }

    /// Uses `ColorType::InheritParent` for both front and background.
    pub const fn inherit_parent() -> Self {
        Self::Style(Style::inherit_parent())
    }

    /// Style set by terminal before entering a Cursive program.
    pub const fn terminal_default() -> Self {
        Self::Style(Style::terminal_default())
    }

    /// Application background, where no view is present.
    pub const fn background() -> Self {
        Self::Palette(PaletteStyle::Background)
    }

    /// Color used by view shadows. Only background matters.
    pub const fn shadow() -> Self {
        Self::Palette(PaletteStyle::Shadow)
    }

    /// Style used for views.
    pub const fn view() -> Self {
        Self::Palette(PaletteStyle::View)
    }

    /// Main text with default background.
    pub const fn primary() -> Self {
        Self::Palette(PaletteStyle::Primary)
    }

    /// Secondary text color, with default background.
    pub const fn secondary() -> Self {
        Self::Palette(PaletteStyle::Secondary)
    }

    /// Tertiary text color, with default background.
    pub const fn tertiary() -> Self {
        Self::Palette(PaletteStyle::Tertiary)
    }

    /// Title text color with default background.
    pub const fn title_primary() -> Self {
        Self::Palette(PaletteStyle::TitlePrimary)
    }

    /// Alternative color for a title.
    pub const fn title_secondary() -> Self {
        Self::Palette(PaletteStyle::TitleSecondary)
    }

    /// Returns a highlight style.
    pub const fn highlight() -> Self {
        Self::Palette(PaletteStyle::Highlight)
    }

    /// Returns an inactive highlight style.
    pub const fn highlight_inactive() -> Self {
        Self::Palette(PaletteStyle::HighlightInactive)
    }
}

impl From<ColorPair> for ConcreteStyle {
    fn from(color: ColorPair) -> Self {
        ConcreteStyle {
            effects: Default::default(),
            color,
        }
    }
}

impl From<Effect> for StyleType {
    fn from(effect: Effect) -> Self {
        StyleType::Style(effect.into())
    }
}

impl From<ColorStyle> for StyleType {
    fn from(color: ColorStyle) -> Self {
        StyleType::Style(color.into())
    }
}

impl From<ColorPair> for StyleType {
    fn from(color: ColorPair) -> Self {
        StyleType::Style(color.into())
    }
}

impl From<Color> for StyleType {
    fn from(color: Color) -> Self {
        StyleType::Style(color.into())
    }
}

impl From<PaletteColor> for StyleType {
    fn from(color: PaletteColor) -> Self {
        StyleType::Style(color.into())
    }
}

impl From<ColorType> for StyleType {
    fn from(color: ColorType) -> Self {
        StyleType::Style(color.into())
    }
}

impl From<Style> for StyleType {
    fn from(style: Style) -> Self {
        StyleType::Style(style)
    }
}

impl From<PaletteStyle> for StyleType {
    fn from(style: PaletteStyle) -> Self {
        StyleType::Palette(style)
    }
}

/// Creates a new `Style` by merging all given styles.
///
/// Will use the last non-`None` color, and will combine all effects.
impl<'a> FromIterator<&'a Style> for Style {
    fn from_iter<I: IntoIterator<Item = &'a Style>>(iter: I) -> Style {
        combine_styles(iter)
    }
}

impl AsRef<Style> for Style {
    fn as_ref(&self) -> &Style {
        self
    }
}

fn combine_styles<S: AsRef<Style>>(styles: impl IntoIterator<Item = S>) -> Style {
    let mut color = ColorStyle::inherit_parent();
    let mut effects = Effects::empty();

    for style in styles {
        let style = style.as_ref();
        color = ColorStyle::merge(color, style.color);
        effects = Effects::merge(effects, style.effects);
    }

    Style { effects, color }
}

/// Creates a new `Style` by merging all given styles.
///
/// Will use the last non-`None` color, and will combine all effects.
impl<T: Into<Style>> FromIterator<T> for Style {
    // TODO: Find some common implementation for both?
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Style {
        combine_styles(iter.into_iter().map(Into::into))
    }
}
