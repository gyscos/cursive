use super::{BaseColor, Color, ColorPair, Palette, PaletteColor};

/// Possible color style for a cell.
///
/// Represents a color pair role to use when printing something.
///
/// The current theme will assign each role a foreground and background color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColorStyle {
    /// Color used for the foreground (the text itself).
    pub front: ColorType,

    /// Color used for the background.
    pub back: ColorType,
}

impl ColorStyle {
    /// Creates
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
    pub fn front<F>(front: F) -> Self
    where
        F: Into<ColorType>,
    {
        Self::new(front, ColorType::InheritParent)
    }

    /// Uses the given color as background, inherits the parent front color.
    pub fn back<B>(back: B) -> Self
    where
        B: Into<ColorType>,
    {
        Self::new(ColorType::InheritParent, back)
    }

    /// Style set by terminal before entering a Cursive program.
    pub fn terminal_default() -> Self {
        Self::new(Color::TerminalDefault, Color::TerminalDefault)
    }

    /// Application background, where no view is present.
    pub fn background() -> Self {
        Self::new(PaletteColor::Background, PaletteColor::Background)
    }

    /// Color used by view shadows. Only background matters.
    pub fn shadow() -> Self {
        Self::new(PaletteColor::Shadow, PaletteColor::Shadow)
    }

    /// Main text with default background.
    pub fn primary() -> Self {
        Self::new(PaletteColor::Primary, PaletteColor::View)
    }

    /// Secondary text color, with default background.
    pub fn secondary() -> Self {
        Self::new(PaletteColor::Secondary, PaletteColor::View)
    }

    /// Tertiary text color, with default background.
    pub fn tertiary() -> Self {
        Self::new(PaletteColor::Tertiary, PaletteColor::View)
    }

    /// Title text color with default background.
    pub fn title_primary() -> Self {
        Self::new(PaletteColor::TitlePrimary, PaletteColor::View)
    }

    /// Alternative color for a title.
    pub fn title_secondary() -> Self {
        Self::new(PaletteColor::TitleSecondary, PaletteColor::View)
    }

    /// Alternate text with highlight background.
    pub fn highlight() -> Self {
        Self::new(PaletteColor::HighlightText, PaletteColor::Highlight)
    }

    /// Highlight color for inactive views (not in focus).
    pub fn highlight_inactive() -> Self {
        Self::new(PaletteColor::HighlightText, PaletteColor::HighlightInactive)
    }

    /// Return the color pair that this style represents.
    pub fn resolve(
        &self,
        palette: &Palette,
        previous: ColorPair,
    ) -> ColorPair {
        ColorPair {
            front: self.front.resolve(palette, previous.front),
            back: self.back.resolve(palette, previous.back),
        }
    }
}

impl From<Color> for ColorStyle {
    fn from(color: Color) -> Self {
        Self::new(color, PaletteColor::View)
    }
}

impl From<BaseColor> for ColorStyle {
    fn from(color: BaseColor) -> Self {
        Self::new(Color::Dark(color), PaletteColor::View)
    }
}

impl From<PaletteColor> for ColorStyle {
    fn from(color: PaletteColor) -> Self {
        Self::new(color, PaletteColor::View)
    }
}

impl From<ColorType> for ColorStyle {
    fn from(color: ColorType) -> Self {
        Self::new(color, PaletteColor::View)
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// Uses a color from the application palette.
    Palette(PaletteColor),

    /// Uses a direct color, independent of the current palette.
    Color(Color),

    /// Re-use the color from the parent.
    InheritParent,
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
