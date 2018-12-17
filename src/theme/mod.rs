//! Handle colors and themes in the UI.
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
//!
//! A [`Palette`] then maps each of these to an actual [`Color`].
//!
//! [`Palette`]: type.Palette.html
//! [`Color`]: enum.Color.html
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
//!     * Its *foreground* color is `View`.
//! * **`ColorStyle::highlight_inactive()`**: style used to print selected,
//!   but inactive items.
//!     * Its *background* color is `HighlightInactive`.
//!     * Its *foreground* color is `View`.
//!
//! Using one of these pairs when styling your application helps give it a
//! coherent look.
//!
//! [`ColorStyle`]: struct.ColorStyle.html
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
//! # Themes
//!
//! A theme defines the color palette an application will use, as well as
//! various options to style views.
//!
//! Themes are described in toml configuration files. All fields are optional.
//!
//! Here are the possible entries:
//!
//! ```toml
//! # Every field in a theme file is optional.
//!
//! # First come some various options
//! shadow = false  # Don't draw shadows around stacked views
//! borders = "simple"  # Alternatives are "none" and "outset"
//!
//! # Here we define the color palette.
//! [colors]
//! 	background = "black"
//! 	# If the value is an array, the first valid color will be used.
//! 	# If the terminal doesn't support custom color,
//! 	# non-base colors will be skipped.
//! 	shadow     = ["#000000", "black"]
//! 	view       = "#d3d7cf"
//!
//! 	# Array and simple values have the same effect.
//! 	primary   = ["#111111"]
//! 	secondary = "#EEEEEE"
//! 	tertiary  = "#444444"
//!
//! 	# Hex values can use lower or uppercase.
//! 	# (base color MUST be lowercase)
//! 	title_primary   = "#ff5555"
//! 	title_secondary = "#ffff55"
//!
//! 	# Lower precision values can use only 3 digits.
//! 	highlight          = "#F00"
//! 	highlight_inactive = "#5555FF"
//! ```
mod border_style;
mod color;
mod color_pair;
mod color_style;
mod effect;
mod palette;
mod style;

pub use self::border_style::BorderStyle;
pub use self::color::{BaseColor, Color};
pub use self::color_pair::ColorPair;
pub use self::color_style::{ColorStyle, ColorType};
pub use self::effect::Effect;
pub use self::palette::{Palette, PaletteColor};
pub use self::style::Style;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use toml;

/// Represents the style a Cursive application will use.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Whether views in a StackView should have shadows.
    pub shadow: bool,
    /// How view borders should be drawn.
    pub borders: BorderStyle,
    /// What colors should be used through the application?
    pub palette: Palette,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            palette: Palette::default(),
        }
    }
}

impl Theme {
    fn load_toml(&mut self, table: &toml::value::Table) {
        if let Some(&toml::Value::Boolean(shadow)) = table.get("shadow") {
            self.shadow = shadow;
        }

        if let Some(&toml::Value::String(ref borders)) = table.get("borders") {
            self.borders = BorderStyle::from(borders);
        }

        if let Some(&toml::Value::Table(ref table)) = table.get("colors") {
            palette::load_toml(&mut self.palette, table);
        }
    }
}

/// Possible error returned when loading a theme.
#[derive(Debug)]
pub enum Error {
    /// An error occured when reading the file.
    Io(io::Error),
    /// An error occured when parsing the toml content.
    Parse(toml::de::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Parse(err)
    }
}

/// Loads a theme from file and sets it as active.
pub fn load_theme_file<P: AsRef<Path>>(filename: P) -> Result<Theme, Error> {
    let content = {
        let mut content = String::new();
        let mut file = File::open(filename)?;
        file.read_to_string(&mut content)?;
        content
    };

    load_toml(&content)
}

/// Loads a theme string and sets it as active.
pub fn load_toml(content: &str) -> Result<Theme, Error> {
    let table = toml::de::from_str(content)?;

    let mut theme = Theme::default();
    theme.load_toml(&table);

    Ok(theme)
}

/// Loads the default theme, and returns its representation.
pub fn load_default() -> Theme {
    Theme::default()
}
