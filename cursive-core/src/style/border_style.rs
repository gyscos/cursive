use super::{ColorKey, PaletteColor};

use enum_map::Enum;

use std::str::FromStr;

/// Color used to draw borders.
///
/// Borders (around dialogs, panels, select popups...) are drawn with this color, in front of the
/// current background. If the theme does not define it, this is the same as
/// [`PaletteColor::Primary`].
///
/// It can be set in the palette with `Palette::set_custom_color(&BORDER, color)`,
/// `Palette::set_color("border", color)`, or with `border = "..."` in the `[colors]` section of
/// a TOML theme.
///
/// With [`BorderStyle::Outset`], the lit side of the borders keeps using
/// [`PaletteColor::Tertiary`] and ignores this color.
///
/// ```rust
/// use cursive_core::style::{BORDER, BaseColor, Color, Palette};
///
/// let mut palette = Palette::default();
/// palette.set_custom_color(&BORDER, Color::Dark(BaseColor::Blue));
/// ```
pub static BORDER: ColorKey = ColorKey::from_palette("border", PaletteColor::Primary);

/// Color used to draw the borders of what currently has the focus.
///
/// When the theme defines it, a border drawn around a focused view (a panel around the view the
/// user is in, the dialog at the front...) uses this color instead of [`BORDER`], so the user
/// can tell where the focus is. More precisely, it applies to everything on the way to the
/// focused view: a focused panel inside a dialog highlights both.
///
/// If the theme does not define it, focused borders look like the others.
///
/// It can be set in the palette with `Palette::set_custom_color(&BORDER_FOCUSED, color)`,
/// `Palette::set_color("border_focused", color)`, or with `border_focused = "..."` in the
/// `[colors]` section of a TOML theme.
///
/// A container gives the focus to one of its children even when none of them can take it, so
/// only use this color in layouts where the framed views are focusable, or accept that the first
/// one may look focused.
///
/// ```rust
/// use cursive_core::style::{BORDER, BORDER_FOCUSED, BaseColor, Color, Palette};
///
/// let mut palette = Palette::default();
/// palette.set_custom_color(&BORDER, Color::Dark(BaseColor::Blue));
/// palette.set_custom_color(&BORDER_FOCUSED, Color::Light(BaseColor::Blue));
/// ```
pub static BORDER_FOCUSED: ColorKey =
    ColorKey::from_palette("border_focused", PaletteColor::Primary);

/// Specifies how some borders should be drawn.
///
/// Borders are used around Dialogs, select popups, and panels.
/// Their color can be changed with the [`BORDER`] and [`BORDER_FOCUSED`] keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum BorderStyle {
    /// Simple borders, with square corners (`┌ ┐ └ ┘`).
    Simple,
    /// Like `Simple`, but with rounded corners (`╭ ╮ ╰ ╯`).
    Rounded,
    /// Outset borders with a simple 3d effect.
    Outset,
    /// No borders.
    None,
}

impl BorderStyle {
    /// The number of variants in this enum.
    pub const LENGTH: usize = crate::style::enum_length::<Self>();

    /// Returns the characters used for the top-left, top-right, bottom-left and bottom-right
    /// corners of a box, or `None` if no border should be drawn.
    pub(crate) fn corners(self) -> Option<[&'static str; 4]> {
        match self {
            BorderStyle::None => None,
            BorderStyle::Rounded => Some(["╭", "╮", "╰", "╯"]),
            BorderStyle::Simple | BorderStyle::Outset => Some(["┌", "┐", "└", "┘"]),
        }
    }

    /// Returns an iterator on all possible border styles.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl FromStr for BorderStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "simple" => Ok(BorderStyle::Simple),
            "rounded" => Ok(BorderStyle::Rounded),
            "outset" => Ok(BorderStyle::Outset),
            "none" => Ok(BorderStyle::None),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_border_styles() {
        assert_eq!("rounded".parse(), Ok(BorderStyle::Rounded));
        assert_eq!("simple".parse(), Ok(BorderStyle::Simple));
        assert_eq!("outset".parse(), Ok(BorderStyle::Outset));
        assert_eq!("none".parse(), Ok(BorderStyle::None));
        assert_eq!("round".parse::<BorderStyle>(), Err(()));
        assert!(BorderStyle::all().any(|b| b == BorderStyle::Rounded));
    }
}
