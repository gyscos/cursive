use super::{BaseColor, Color, ColorPair, Palette, PaletteColor};
use std::str::FromStr;

/// Possible color style for a cell.
///
/// Represents a color pair role to use when printing something.
///
/// The current theme will assign each role a foreground and background color.
///
/// The `Default` value is to inherit the parent's colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ColorStyle {
    /// Color used for the foreground (the text itself).
    pub front: ColorType,

    /// Color used for the background.
    pub back: ColorType,
}

impl ColorStyle {
    /// Creates a new color style, using the given values for front and back.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core as cursive;
    /// use cursive::style::ColorStyle;
    ///
    /// // `BaseColor` implements `Into<ColorStyle>`
    /// use cursive::style::BaseColor::*;
    /// let red_on_black = ColorStyle::new(Red, Black);
    ///
    /// // So does `Color`.
    /// let red_on_black = ColorStyle::new(Red.light(), Black.dark());
    ///
    /// // Or `PaletteColor`.
    /// use cursive::style::PaletteColor::*;
    /// let primary = ColorStyle::new(Primary, View);
    /// ```
    pub fn new<F, B>(front: F, back: B) -> Self
    where
        F: Into<ColorType>,
        B: Into<ColorType>,
    {
        let front = front.into();
        let back = back.into();
        Self { front, back }
    }

    /// Uses the given color as front, inherits the parent background color.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core as cursive;
    /// use cursive::style::{BaseColor::*, ColorStyle, ColorType};
    ///
    /// let color = ColorStyle::front(Red.dark());
    ///
    /// assert_eq!(color, ColorStyle::new(Red.dark(), ColorType::InheritParent));
    /// ```
    pub fn front<F>(front: F) -> Self
    where
        F: Into<ColorType>,
    {
        Self::new(front, ColorType::InheritParent)
    }

    /// Uses the given color as background, inherits the parent front color.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core as cursive;
    /// use cursive::style::{BaseColor::*, ColorStyle, ColorType};
    ///
    /// let color = ColorStyle::back(Black.dark());
    ///
    /// assert_eq!(
    ///     color,
    ///     ColorStyle::new(ColorType::InheritParent, Black.dark())
    /// );
    /// ```
    pub fn back<B>(back: B) -> Self
    where
        B: Into<ColorType>,
    {
        Self::new(ColorType::InheritParent, back)
    }

    /// Returns an inverted color style, with the front and back colors swapped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core as cursive;
    /// use cursive::style::BaseColor::*;
    /// use cursive::style::ColorStyle;
    ///
    /// let red_on_black = ColorStyle::new(Red.dark(), Black.dark());
    /// let black_on_red = red_on_black.invert();
    ///
    /// assert_eq!(black_on_red, ColorStyle::new(Black.dark(), Red.dark()));
    /// ```
    #[must_use]
    pub const fn invert(self) -> Self {
        ColorStyle {
            front: self.back,
            back: self.front,
        }
    }

    /// Uses `ColorType::InheritParent` for both front and background.
    pub const fn inherit_parent() -> Self {
        Self {
            front: ColorType::InheritParent,
            back: ColorType::InheritParent,
        }
    }

    /// Style set by terminal before entering a Cursive program.
    pub const fn terminal_default() -> Self {
        Self {
            front: ColorType::terminal_default(),
            back: ColorType::terminal_default(),
        }
    }

    /// Application background, where no view is present.
    pub const fn background() -> Self {
        Self {
            front: ColorType::background(),
            back: ColorType::background(),
        }
    }

    /// Color used by view shadows. Only background matters.
    pub const fn shadow() -> Self {
        Self {
            front: ColorType::shadow(),
            back: ColorType::shadow(),
        }
    }

    /// Color used by views.
    ///
    /// Primary foreground, View background.
    pub const fn view() -> Self {
        Self {
            front: ColorType::primary(),
            back: ColorType::view(),
        }
    }

    /// Main text with default background.
    pub const fn primary() -> Self {
        Self {
            front: ColorType::primary(),
            back: ColorType::InheritParent,
        }
    }

    /// Secondary text color, with default background.
    pub const fn secondary() -> Self {
        Self {
            front: ColorType::secondary(),
            back: ColorType::InheritParent,
        }
    }

    /// Tertiary text color, with default background.
    pub const fn tertiary() -> Self {
        Self {
            front: ColorType::tertiary(),
            back: ColorType::InheritParent,
        }
    }

    /// Title text color with default background.
    pub const fn title_primary() -> Self {
        Self {
            front: ColorType::title_primary(),
            back: ColorType::InheritParent,
        }
    }

    /// Alternative color for a title.
    pub const fn title_secondary() -> Self {
        Self {
            front: ColorType::title_secondary(),
            back: ColorType::InheritParent,
        }
    }

    /// Alternate text with highlight background.
    pub const fn highlight() -> Self {
        Self {
            front: ColorType::highlight_text(),
            back: ColorType::highlight(),
        }
    }

    /// Highlight color for inactive views (not in focus).
    pub const fn highlight_inactive() -> Self {
        Self {
            front: ColorType::highlight_text(),
            back: ColorType::highlight_inactive(),
        }
    }

    /// Merge the color type `new` over the color type `old`.
    ///
    /// This merges the front and back color types of `a` and `b`.
    pub const fn merge(old: Self, new: Self) -> Self {
        Self {
            front: ColorType::merge(old.front, new.front),
            back: ColorType::merge(old.back, new.back),
        }
    }

    /// Return the color pair that this style represents.
    pub fn resolve(&self, palette: &Palette, previous: ColorPair) -> ColorPair {
        ColorPair {
            front: self.front.resolve(palette, previous.front),
            back: self.back.resolve(palette, previous.back),
        }
    }

    /// Apply a function to both the front and back colors.
    pub fn map<F: FnMut(ColorType) -> ColorType>(self, mut f: F) -> Self {
        ColorStyle {
            front: f(self.front),
            back: f(self.back),
        }
    }

    /// Apply a function to each pair of front/back color.
    pub fn zip_map<F: FnMut(ColorType, ColorType) -> ColorType>(
        self,
        other: Self,
        mut f: F,
    ) -> Self {
        ColorStyle {
            front: f(self.front, other.front),
            back: f(self.back, other.back),
        }
    }

    #[cfg(feature = "toml")]
    pub(crate) fn parse(table: &toml::value::Table) -> Option<Self> {
        let front = table.get("front")?.as_str()?.parse().ok()?;
        let back = table.get("back")?.as_str()?.parse().ok()?;

        Some(ColorStyle { front, back })
    }
}

impl From<ColorPair> for ColorStyle {
    fn from(color: ColorPair) -> Self {
        Self::new(color.front, color.back)
    }
}

impl From<Color> for ColorStyle {
    fn from(color: Color) -> Self {
        Self::front(color)
    }
}

impl From<BaseColor> for ColorStyle {
    fn from(color: BaseColor) -> Self {
        Self::front(Color::Dark(color))
    }
}

impl From<PaletteColor> for ColorStyle {
    fn from(color: PaletteColor) -> Self {
        Self::front(color)
    }
}

impl From<ColorType> for ColorStyle {
    fn from(color: ColorType) -> Self {
        Self::front(color)
    }
}

impl<F, B> From<(F, B)> for ColorStyle
where
    F: Into<ColorType>,
    B: Into<ColorType>,
{
    fn from((front, back): (F, B)) -> Self {
        Self::new(front, back)
    }
}

/// Either a color from the palette, or a direct color.
///
/// The `Default` implementation returns `InheritParent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// Uses a color from the application palette.
    ///
    /// This is the best way to support themes and achieve a unified look
    /// across different views.
    Palette(PaletteColor),

    /// Uses a direct color, independent of the current palette.
    Color(Color),

    /// Re-uses the color from the parent.
    InheritParent,
}

impl Default for ColorType {
    fn default() -> Self {
        ColorType::InheritParent
    }
}

impl ColorType {
    /// Given a palette, resolve `self` to a concrete color.
    pub fn resolve(self, palette: &Palette, previous: Color) -> Color {
        match self {
            ColorType::Color(color) => color,
            ColorType::Palette(color) => color.resolve(palette),
            ColorType::InheritParent => previous,
        }
    }

    /// Merge the color type `new` over the color type `old`.
    ///
    /// This returns `new`, unless `new = ColorType::InheritParent`,
    /// in which case it returns `old`.
    pub const fn merge(old: ColorType, new: ColorType) -> ColorType {
        match new {
            ColorType::InheritParent => old,
            new => new,
        }
    }

    /// Return the `Color::TerminalDefault` color type.
    pub const fn terminal_default() -> Self {
        Self::Color(Color::TerminalDefault)
    }

    /// Return the `Color::Dark(base)` color type.
    pub const fn dark(base: BaseColor) -> Self {
        Self::Color(Color::Dark(base))
    }

    /// Return the `Color::Light(base)` color type.
    pub const fn light(base: BaseColor) -> Self {
        Self::Color(Color::Light(base))
    }

    /// Return the `Color::Rgb(r, g, b)` (true colors) color type.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Color(Color::Rgb(r, g, b))
    }

    /// Return the `Color::RgbLowRes(r, g, b)` color type.
    ///
    /// Returns `None`
    pub const fn rgb_low_res(r: u8, g: u8, b: u8) -> Option<Self> {
        // TODO: Use ? or `.map` once these support const.
        match Color::low_res(r, g, b) {
            None => None,
            Some(color) => Some(Self::Color(color)),
        }
    }

    /// Return the `PaletteColor::Background` color type.
    pub const fn background() -> Self {
        Self::Palette(PaletteColor::Background)
    }

    /// Return the `PaletteColor::Shadow` color type.
    pub const fn shadow() -> Self {
        Self::Palette(PaletteColor::Shadow)
    }

    /// Return the `PaletteColor::TitlePrimary` color type.
    pub const fn title_primary() -> Self {
        Self::Palette(PaletteColor::TitlePrimary)
    }

    /// Return the `PaletteColor::TitleSecondary` color type.
    pub const fn title_secondary() -> Self {
        Self::Palette(PaletteColor::TitleSecondary)
    }

    /// Return the `PaletteColor::Primary` color type.
    pub const fn primary() -> Self {
        Self::Palette(PaletteColor::Primary)
    }

    /// Return the `PaletteColor::Secondary` color type.
    pub const fn secondary() -> Self {
        Self::Palette(PaletteColor::Secondary)
    }

    /// Return the `PaletteColor::Tertiary` color type.
    pub const fn tertiary() -> Self {
        Self::Palette(PaletteColor::Tertiary)
    }

    /// Return the `PaletteColor::View` color type.
    pub const fn view() -> Self {
        Self::Palette(PaletteColor::View)
    }

    /// Return the `PaletteColor::HighlightText` color type.
    pub const fn highlight_text() -> Self {
        Self::Palette(PaletteColor::HighlightText)
    }

    /// Return the `PaletteColor::HighlightInactive` color type.
    pub const fn highlight_inactive() -> Self {
        Self::Palette(PaletteColor::HighlightInactive)
    }

    /// Return the `PaletteColor::Highlight` color type.
    pub const fn highlight() -> Self {
        Self::Palette(PaletteColor::Highlight)
    }
}

impl FromStr for ColorType {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "inherit_parent" || s == "InheritParent" || s == "inherit parent" || s == "parent" {
            return Ok(ColorType::InheritParent);
        }

        if let Ok(color) = s.parse() {
            return Ok(ColorType::Palette(color));
        }

        if let Ok(color) = s.parse() {
            return Ok(ColorType::Color(color));
        }

        Err(super::NoSuchColor)
    }
}

impl From<BaseColor> for ColorType {
    fn from(color: BaseColor) -> Self {
        ColorType::Color(color.dark())
    }
}

impl From<Color> for ColorType {
    fn from(color: Color) -> Self {
        ColorType::Color(color)
    }
}

impl From<PaletteColor> for ColorType {
    fn from(color: PaletteColor) -> Self {
        ColorType::Palette(color)
    }
}
