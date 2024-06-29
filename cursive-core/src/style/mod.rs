//! Handle colors and styles in the UI.
//!
//! # Colors
//!
//! Characters can be printed in the terminal with a specific color.
//! The [`Color`] enum represents the ways this can be set.
//!
//! [`Color`]: enum.Color.html
//!
//! # Palette
//!
//! To achieve a customizable, yet unified look, Cursive uses a configurable
//! palette of colors to be used through the entire application.
//!
//! The [`PaletteColor`] enum defines possible entries in this palette.
//!
//! [`PaletteColor`]: enum.PaletteColor.html
//!
//! These entries are:
//!
//! * **`Background`**: used to color the application background
//!   (around views).
//!   Defaults to **blue**.
//! * **`Shadow`**: used to color shadow around views.
//!   Defaults to **black**.
//! * **`View`**: used to color the background for views.
//!   Defaults to **white**.
//! * **`Primary`**: used to print primary text.
//!   Defaults to **black**.
//! * **`Secondary`**: used to print secondary text.
//!   Defaults to **blue**.
//! * **`Tertiary`**: used to print tertiary text.
//!   Defaults to **white**.
//! * **`TitlePrimary`**: used to print primary titles.
//!   Defaults to **red**.
//! * **`TitleSecondary`**: used to print secondary titles.
//!   Defaults to **yellow**.
//! * **`Highlight`**: used to highlight selected items.
//!   Defaults to **red**.
//! * **`HighlightInactive`**: used to highlight selected but inactive items.
//!   Defaults to **blue**.
//! * **`HighlightText`**: used to print primary text when highlighted
//!   Defaults to **white**.
//!
//! A [`Palette`] then maps each of these to an actual [`Color`].
//!
//! [`Palette`]: struct.Palette.html
//!
//! # Color Types
//!
//! When drawing views, color can be picked in two way:
//!
//! * An exact [`Color`] can be given directly
//! * A [`PaletteColor`] entry can be given, which will fetch whatever color
//!   is currently defined for this.
//!
//! The [`ColorType`] enum abstract over these two choices.
//!
//! [`ColorType`]: enum.ColorType.html
//!
//! # Color Styles
//!
//! To actually print anything, two colors must be picked: one for the
//! foreground, and one for the background.
//!
//! As such, a [`ColorStyle`] is made of a pair of `ColorType`.
//!
//! [`ColorStyle`]: struct.ColorStyle.html
//!
//! Since some pairs are frequently used, `ColorStyle` defines some methods to
//! create these usual values:
//!
//! * **`ColorStyle::background()`**: style used to print the application
//!   background.
//!     * Its *background* color is `Background`.
//!     * Its *foreground* color is unimportant as no characters are ever
//!       printed in the background.
//! * **`ColorStyle::shadow()`**: style used to print shadows behind views.
//!     * Its *background* color is `Shadow`.
//!     * Here again, the *foreground* color is unimportant.
//! * **`ColorStyle::primary()`**: style used to print primary text.
//!     * Its *background* color is `View`.
//!     * Its *foreground* color is `Primary`.
//! * **`ColorStyle::secondary()`**: style used to print secondary text.
//!     * Its *background* color is `View`.
//!     * Its *foreground* color is `Secondary`.
//! * **`ColorStyle::tertiary()`**: style used to print tertiary text.
//!     * Its *background* color is `View`.
//!     * Its *foreground* color is `Tertiary`.
//! * **`ColorStyle::title_primary()`**: style used to print titles.
//!     * Its *background* color is `View`.
//!     * Its *foreground* color is `TitlePrimary`.
//! * **`ColorStyle::title_secondary()`**: style used to print secondary
//!   titles.
//!     * Its *background* color is `View`.
//!     * Its *foreground* color is `TitleSecondary`.
//! * **`ColorStyle::highlight()`**: style used to print selected items.
//!     * Its *background* color is `Highlight`.
//!     * Its *foreground* color is `HighlightText`.
//! * **`ColorStyle::highlight_inactive()`**: style used to print selected,
//!   but inactive items.
//!     * Its *background* color is `HighlightInactive`.
//!     * Its *foreground* color is `HighlightText`.
//!
//! Using one of these pairs when styling your application helps give it a
//! coherent look.
//!
//! # Effects
//!
//! On top of a color style, some effects can be applied on cells: `Reverse`,
//! for instance, swaps the foreground and background colors of a cell.
//!
//!
//! # Style
//!
//! Finally, a style combine a [`ColorType`] and a set of [`Effect`]s, to
//! represent any way text should be printed on screen.
//!
//! [`Effect`]: enum.Effect.html
//!
//!
//! [`Color`]: ./enum.Color.html
//! [`PaletteColor`]: ./enum.PaletteColor.html
//! [`Palette`]: ./struct.Palette.html
//! [`ColorType`]: ./enum.ColorType.html
//! [`ColorStyle`]: ./struct.ColorStyle.html
mod border_style;
mod color;
mod color_pair;
mod color_style;
mod effect;
pub mod gradient;
mod palette;
mod style_types;

pub use self::border_style::BorderStyle;
pub use self::color::{BaseColor, Color, Rgb};
pub use self::color_pair::ColorPair;
pub use self::color_style::{ColorStyle, ColorType};
pub use self::effect::{ConcreteEffects, Effect, EffectStatus, Effects};
pub use self::palette::{Palette, PaletteColor, PaletteNode, PaletteStyle};
pub use self::style_types::{ConcreteStyle, Style, StyleType};

/// Error parsing a color.
#[derive(Debug)]
pub struct NoSuchColor;

impl std::fmt::Display for NoSuchColor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Could not parse the given color")
    }
}

impl std::error::Error for NoSuchColor {}
