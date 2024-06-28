//! Theming support for a consistent UI.
//!
//! A [`Theme`] object defines the color palette an application will use, as
//! well as various options to style views.
//!
//! There are several ways to set a theme for the application:
//!
//! * Construct a [`Theme`] object by setting every field individually.
//! * Get the current theme with [`Cursive::current_theme`] method and
//!   changing the required fields (for example see [theme_manual example]).
//! * Using a toml file as a theme configuration (for example see
//!   [theme example]).
//!
//! ## Configuring theme with toml
//!
//! This requires the `toml` feature to be enabled.
//!
//! ```toml
//! [dependencies]
//! cursive = { version = "*", features = ["toml"] }
//! ```
//!
//! To use the theme in your application, load it with [`Cursive::load_toml`]
//! method (or use [`theme::load_theme_file`] to acquire the theme object).
//!
//! ```rust,ignore
//! let mut siv = Cursive::new();
//! // Embed the theme with the binary.
//! siv.load_toml(include_str!("<path_to_theme_file>.toml")).unwrap();
//! ```
//!
//! Here are the possible entries (all fields are optional):
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
//!     background = "black"
//!     # If the value is an array, the first valid color will be used.
//!     # If the terminal doesn't support custom color,
//!     # non-base colors will be skipped.
//!     shadow     = ["#000000", "black"]
//!     view       = "#d3d7cf"
//!
//!     # Array and simple values have the same effect.
//!     primary   = ["#111111"]
//!     secondary = "#EEEEEE"
//!     tertiary  = "#444444"
//!
//!     # Hex values can use lower or uppercase.
//!     # (base color MUST be lowercase)
//!     title_primary   = "#ff5555"
//!     title_secondary = "#ffff55"
//!
//!     # Lower precision values can use only 3 digits.
//!     highlight          = "#F00"
//!     highlight_inactive = "#5555FF"
//! ```
//!
//! [`Theme`]: ./struct.Theme.html
//! [`Cursive::current_theme`]: ../struct.Cursive.html#method.current_theme
//! [theme_manual example]: https://github.com/gyscos/cursive/blob/main/cursive/examples/theme_manual.rs
//! [theme example]: https://github.com/gyscos/cursive/blob/main/cursive/examples/theme.rs
//! [`Cursive::load_toml`]: ../struct.Cursive.html#method.load_toml
//! [`theme::load_theme_file`]: ./fn.load_theme_file.html
//!
//! # Re-exports from `style`
//!
//! For backward-compatibility, this module re-exports most of the [`crate::style`] module.
//! These re-exports are deprecated and will be removed in a future version.

// Deprecated re-export
#[deprecated]
pub use crate::style::{
    BaseColor, BorderStyle, Color, ColorPair, ColorStyle, ColorType, ConcreteEffects,
    ConcreteStyle, Effect, EffectStatus, Effects, NoSuchColor, Palette, PaletteColor, PaletteNode,
    PaletteStyle, Style, StyleType,
};

#[cfg(feature = "toml")]
use std::fs::File;
use std::io;
#[cfg(feature = "toml")]
use std::io::Read;
#[cfg(feature = "toml")]
use std::path::Path;

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

/// Currently returns the retro theme.
impl Default for Theme {
    fn default() -> Self {
        Theme::retro()
    }
}

impl Theme {
    /// Returns a bi-color theme using the terminal default text and background colors.
    pub fn terminal_default() -> Self {
        Theme {
            shadow: false,
            borders: BorderStyle::Simple,
            palette: Palette::terminal_default(),
        }
    }

    /// Returns a retro theme, similar to GNU dialog applications.
    pub fn retro() -> Self {
        Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            palette: Palette::retro(),
        }
    }

    #[cfg(feature = "toml")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
    /// Load values from an already parsed toml [`Table`], overwriting previous values.
    ///
    /// [`Table`]: https://docs.rs/toml/latest/toml/type.Table.html
    pub fn load_toml(&mut self, table: &toml::value::Table) {
        if let Some(&toml::Value::Boolean(shadow)) = table.get("shadow") {
            self.shadow = shadow;
        }

        if let Some(toml::Value::String(borders)) = table.get("borders") {
            self.borders = BorderStyle::from(borders);
        }

        if let Some(toml::Value::Table(table)) = table.get("colors") {
            self.palette.load_toml(table);
        }

        if let Some(toml::Value::Table(table)) = table.get("styles") {
            self.palette.load_toml_styles(table);
        }
    }
}

/// Possible error returned when loading a theme.
#[derive(Debug)]
pub enum Error {
    /// An error occurred when reading the file.
    Io(io::Error),

    #[cfg(feature = "toml")]
    #[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
    /// An error occurred when parsing the toml content.
    Parse(toml::de::Error),
}

#[cfg(feature = "toml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

#[cfg(feature = "toml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Parse(err)
    }
}

/// Loads a theme from file.
///
/// Must have the `toml` feature enabled.
#[cfg(feature = "toml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
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
///
/// Must have the `toml` feature enabled.
#[cfg(feature = "toml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
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
