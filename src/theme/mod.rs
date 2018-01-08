//! Handle colors and themes in the UI.
//!
//! # Color palette
//!
//! To achieve a customizable, yet unified look, Cursive uses a configurable
//! palette of colors to be used through the entire application.
//!
//! These colors are:
//!
//! * **`background`**: used to color the application background
//!   (around views).
//!   Defaults to **blue**.
//! * **`shadow`**: used to color shadow around views.
//!   Defaults to **black**.
//! * **`view`**: used to color the background for views.
//!   Defaults to **white**.
//! * **`primary`**: used to print primary text.
//!   Defaults to **black**.
//! * **`secondary`**: used to print secondary text.
//!   Defaults to **blue**.
//! * **`tertiary`**: used to print tertiary text.
//!   Defaults to **white**.
//! * **`title_primary`**: used to print primary titles.
//!   Defaults to **red**.
//! * **`title_secondary`**: used to print secondary titles.
//!   Defaults to **yellow**.
//! * **`highlight`**: used to highlight selected items.
//!   Defaults to **red**.
//! * **`highlight_inactive`**: used to highlight selected but inactive items.
//!   Defaults to **blue**.
//!
//! # Color Styles
//!
//! Each cell of the terminal uses two colors: *foreground* and *background*.
//!
//! Color styles are defined to easily refer to a pair of colors from the
//! palette.
//!
//! * **`Background`**: style used to print the application background.
//!     * Its *background* color is `background`.
//!     * Its *foreground* color is unimportant as no characters are ever
//!       printed in the background.
//! * **`Shadow`**: style used to print shadows behind views.
//!     * Its *background* color is `shadow`.
//!     * Here again, the *foreground* color is unimportant.
//! * **`Primary`**: style used to print primary text.
//!     * Its *background* color is `view`.
//!     * Its *foreground* color is `primary`.
//! * **`Secondary`**: style used to print secondary text.
//!     * Its *background* color is `view`.
//!     * Its *foreground* color is `secondary`.
//! * **`Tertiary`**: style used to print tertiary text.
//!     * Its *background* color is `view`.
//!     * Its *foreground* color is `tertiary`.
//! * **`TitlePrimary`**: style used to print titles.
//!     * Its *background* color is `view`.
//!     * Its *foreground* color is `title_primary`.
//! * **`TitleSecondary`**: style used to print secondary titles.
//!     * Its *background* color is `view`.
//!     * Its *foreground* color is `title_secondary`.
//! * **`Highlight`**: style used to print selected items.
//!     * Its *background* color is `highlight`.
//!     * Its *foreground* color is `view`.
//! * **`HighlightInactive`**: style used to print selected,
//!   but inactive items.
//!     * Its *background* color is `highlight_inactive`.
//!     * Its *foreground* color is `view`.
//!
//! Using one of these pairs when styling your application helps give it a
//! coherent look.
//!
//! # Effects
//!
//! On top of a color style, some effects can be applied on cells: `Reverse`,
//! for instance, swaps the foreground and background colors of a cell.
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
mod style;
mod effect;
mod color;
mod color_pair;
mod color_style;
mod border_style;
mod palette;

pub use self::color::{Color, BaseColor};
pub use self::border_style::BorderStyle;
pub use self::color_pair::ColorPair;
pub use self::color_style::ColorStyle;
pub use self::effect::Effect;
pub use self::palette::Palette;
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
    pub colors: Palette,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            colors: Palette {
                background: Color::Dark(BaseColor::Blue),
                shadow: Color::Dark(BaseColor::Black),
                view: Color::Dark(BaseColor::White),
                primary: Color::Dark(BaseColor::Black),
                secondary: Color::Dark(BaseColor::Blue),
                tertiary: Color::Light(BaseColor::White),
                title_primary: Color::Dark(BaseColor::Red),
                title_secondary: Color::Dark(BaseColor::Yellow),
                highlight: Color::Dark(BaseColor::Red),
                highlight_inactive: Color::Dark(BaseColor::Blue),
            },
        }
    }
}

impl Theme {
    fn load(&mut self, table: &toml::value::Table) {
        if let Some(&toml::Value::Boolean(shadow)) = table.get("shadow") {
            self.shadow = shadow;
        }

        if let Some(&toml::Value::String(ref borders)) = table.get("borders") {
            self.borders = BorderStyle::from(borders);
        }

        if let Some(&toml::Value::Table(ref table)) = table.get("colors") {
            self.colors.load(table);
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
        let mut file = try!(File::open(filename));
        try!(file.read_to_string(&mut content));
        content
    };

    load_theme(&content)
}

/// Loads a theme string and sets it as active.
pub fn load_theme(content: &str) -> Result<Theme, Error> {
    let table = toml::de::from_str(content)?;

    let mut theme = Theme::default();
    theme.load(&table);

    Ok(theme)
}

/// Loads the default theme, and returns its representation.
pub fn load_default() -> Theme {
    Theme::default()
}

