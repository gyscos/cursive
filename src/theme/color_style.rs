use super::{Color, ColorPair, Theme};

/// Possible color style for a cell.
///
/// Represents a color pair role to use when printing something.
///
/// The current theme will assign each role a foreground and background color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorStyle {
    /// Style set by terminal before entering a Cursive program.
    TerminalDefault,
    /// Application background, where no view is present.
    Background,
    /// Color used by view shadows. Only background matters.
    Shadow,
    /// Main text with default background.
    Primary,
    /// Secondary text color, with default background.
    Secondary,
    /// Tertiary text color, with default background.
    Tertiary,
    /// Title text color with default background.
    TitlePrimary,
    /// Alternative color for a title.
    TitleSecondary,
    /// Alternate text with highlight background.
    Highlight,
    /// Highlight color for inactive views (not in focus).
    HighlightInactive,
    /// Directly specifies colors, independently of the theme.
    Custom {
        /// Foreground color
        front: Color,
        /// Background color
        back: Color,
    },
}

impl ColorStyle {
    /// Return the color pair that this style represents.
    ///
    /// Returns `(front, back)`.
    pub fn resolve(&self, theme: &Theme) -> ColorPair {
        let c = &theme.colors;
        let (front, back) = match *self {
            ColorStyle::TerminalDefault => {
                (Color::TerminalDefault, Color::TerminalDefault)
            }
            ColorStyle::Background => (c.view, c.background),
            ColorStyle::Shadow => (c.shadow, c.shadow),
            ColorStyle::Primary => (c.primary, c.view),
            ColorStyle::Secondary => (c.secondary, c.view),
            ColorStyle::Tertiary => (c.tertiary, c.view),
            ColorStyle::TitlePrimary => (c.title_primary, c.view),
            ColorStyle::TitleSecondary => (c.title_secondary, c.view),
            ColorStyle::Highlight => (c.view, c.highlight),
            ColorStyle::HighlightInactive => (c.view, c.highlight_inactive),
            ColorStyle::Custom { front, back } => (front, back),
        };
        ColorPair { front, back }
    }
}
