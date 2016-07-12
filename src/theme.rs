//! Module to handle colors and themes in the UI.

use std::io;
use std::io::Read;
use std::fs::File;
use std::path::Path;

use backend::Backend;

use toml;

use B;

/// Text effect
pub enum Effect {
    /// No effect
    Simple,
    /// Reverses foreground and background colors
    Reverse,
}

/// Possible color style for a cell.
///
/// Represents a color pair role to use when printing something.
///
/// The current theme will assign each role a foreground and background color.
#[derive(Clone,Copy)]
pub enum ColorStyle {
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
}

impl ColorStyle {
    /// Returns the ncurses pair ID associated with this color pair.
    pub fn id(self) -> i16 {
        match self {
            ColorStyle::Background => 1,
            ColorStyle::Shadow => 2,
            ColorStyle::Primary => 3,
            ColorStyle::Secondary => 4,
            ColorStyle::Tertiary => 5,
            ColorStyle::TitlePrimary => 6,
            ColorStyle::TitleSecondary => 7,
            ColorStyle::Highlight => 8,
            ColorStyle::HighlightInactive => 9,
        }
    }
}

/// Represents the style a Cursive application will use.
#[derive(Clone,Debug)]
pub struct Theme {
    /// Wheter views in a StackView should have shadows.
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
                background: Color::Blue,
                shadow: Color::Black,
                view: Color::White,
                primary: Color::Black,
                secondary: Color::Blue,
                tertiary: Color::White,
                title_primary: Color::Red,
                title_secondary: Color::Yellow,
                highlight: Color::Red,
                highlight_inactive: Color::Blue,
            },
        }
    }
}

impl Theme {
    fn load(&mut self, table: &toml::Table) {
        if let Some(&toml::Value::Boolean(shadow)) = table.get("shadow") {
            self.shadow = shadow;
        }

        if let Some(&toml::Value::String(ref borders)) = table.get("borders") {
            if let Some(borders) = BorderStyle::from(borders) {
                self.borders = borders;
            }
        }

        if let Some(&toml::Value::Table(ref table)) = table.get("colors") {
            self.colors.load(table);
        }
    }

    fn activate(&self) {
        // Initialize each color with the backend
        B::init_color_style(ColorStyle::Background,
                            &self.colors.view,
                            &self.colors.background);
        B::init_color_style(ColorStyle::Shadow,
                            &self.colors.shadow,
                            &self.colors.shadow);
        B::init_color_style(ColorStyle::Primary,
                            &self.colors.primary,
                            &self.colors.view);
        B::init_color_style(ColorStyle::Secondary,
                            &self.colors.secondary,
                            &self.colors.view);
        B::init_color_style(ColorStyle::Tertiary,
                            &self.colors.tertiary,
                            &self.colors.view);
        B::init_color_style(ColorStyle::TitlePrimary,
                            &self.colors.title_primary,
                            &self.colors.view);
        B::init_color_style(ColorStyle::TitleSecondary,
                            &self.colors.title_secondary,
                            &self.colors.view);
        B::init_color_style(ColorStyle::Highlight,
                            &self.colors.view,
                            &self.colors.highlight);
        B::init_color_style(ColorStyle::HighlightInactive,
                            &self.colors.view,
                            &self.colors.highlight_inactive);
    }
}

/// Specifies how some borders should be drawn.
///
/// Borders are used around Dialogs, select popups, and panels.
#[derive(Clone,Copy,Debug)]
pub enum BorderStyle {
    /// Don't draw any border.
    NoBorder,
    /// Simple borders.
    Simple,
    /// Outset borders with a simple 3d effect.
    Outset,
}

impl BorderStyle {
    fn from(s: &str) -> Option<Self> {
        if s == "simple" {
            Some(BorderStyle::Simple)
        } else if s == "none" {
            Some(BorderStyle::NoBorder)
        } else if s == "outset" {
            Some(BorderStyle::Outset)
        } else {
            None
        }
    }
}

/// Color configuration for the application.
///
/// Assign each color role an actual color.
#[derive(Clone,Debug)]
pub struct Palette {
    /// Color used for the application background.
    pub background: Color,
    /// Color used for View shadows.
    pub shadow: Color,
    /// Color used for View backgrounds.
    pub view: Color,
    /// Primary color used for the text.
    pub primary: Color,
    /// Secondary color used for the text.
    pub secondary: Color,
    /// Tertiary color used for the text.
    pub tertiary: Color,
    /// Primary color used for title text.
    pub title_primary: Color,
    /// Secondary color used for title text.
    pub title_secondary: Color,
    /// Color used for highlighting text.
    pub highlight: Color,
    /// Color used for highlighting inactive text.
    pub highlight_inactive: Color,
}

impl Palette {
    fn load(&mut self, table: &toml::Table) {

        load_color(&mut self.background, table.get("background"));
        load_color(&mut self.shadow, table.get("shadow"));
        load_color(&mut self.view, table.get("view"));
        load_color(&mut self.primary, table.get("primary"));
        load_color(&mut self.secondary, table.get("secondary"));
        load_color(&mut self.tertiary, table.get("tertiary"));
        load_color(&mut self.title_primary, table.get("title_primary"));
        load_color(&mut self.title_secondary, table.get("title_secondary"));
        load_color(&mut self.highlight, table.get("highlight"));
        load_color(&mut self.highlight_inactive,
                   table.get("highlight_inactive"));
    }
}

fn load_color(target: &mut Color, value: Option<&toml::Value>) -> bool {
    if let Some(value) = value {
        match *value {
            toml::Value::String(ref value) => {
                if let Some(color) = Color::parse(value) {
                    *target = color;
                    true
                } else {
                    false
                }
            }
            toml::Value::Array(ref array) => {
                array.iter().any(|item| load_color(target, Some(item)))
            }
            _ => false,
        }
    } else {
        false
    }
}

/// Represents a color used by the theme.
#[derive(Clone,Debug)]
pub enum Color {
    /// Black color
    ///
    /// Color #0
    Black,
    /// Red color
    ///
    /// Color #1
    Red,
    /// Green color
    ///
    /// Color #2
    Green,
    /// Yellow color (Red + Green)
    ///
    /// Color #3
    Yellow,
    /// Blue color
    ///
    /// Color #4
    Blue,
    /// Magenta color (Red + Blue)
    ///
    /// Color #5
    Magenta,
    /// Cyan color (Green + Blue)
    ///
    /// Color #6
    Cyan,
    /// White color (Red + Green + Blue)
    ///
    /// Color #7
    White,
    /// True-color, 24-bit.
    Rgb(u8, u8, u8),
    /// Low-resolution
    ///
    /// Each value should be `<= 5` (you'll get panics otherwise).
    ///
    /// These 216 possible colors are part of the default color palette.
    RgbLowRes(u8, u8, u8),
}

impl Color {}

/// Possible error returned when loading a theme.
#[derive(Debug)]
pub enum Error {
    /// An error occured when reading the file.
    Io(io::Error),
    /// An error occured when parsing the toml content.
    Parse,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl Color {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            value => return Color::parse_special(value),
        })
    }

    fn parse_special(value: &str) -> Option<Color> {
        if value.starts_with('#') {

            let value = &value[1..];
            // Compute per-color length, and amplitude
            let (l, multiplier) = match value.len() {
                6 => (2, 1),
                3 => (1, 17),
                _ => panic!("Cannot parse color: {}", value),
            };
            let r = load_hex(&value[0 * l..1 * l]) * multiplier;
            let g = load_hex(&value[1 * l..2 * l]) * multiplier;
            let b = load_hex(&value[2 * l..3 * l]) * multiplier;
            Some(Color::Rgb(r as u8, g as u8, b as u8))
        } else if value.len() == 3 {
            // RGB values between 0 and 5 maybe?
            let rgb: Vec<_> =
                value.chars().map(|c| c as i16 - '0' as i16).collect();
            if rgb.iter().all(|&i| i >= 0 && i < 6) {
                Some(Color::RgbLowRes(rgb[0] as u8,
                                      rgb[1] as u8,
                                      rgb[2] as u8))
            } else {
                None
            }
        } else {
            None
        }
    }
}


/// Loads a theme file, and returns its representation.
///
/// The file should be a toml file. All fields are optional.
///
/// Here are the possible entries:
///
/// ```text
/// # Every field in a theme file is optional.
///
/// shadow = false
/// borders = "simple" # Alternatives are "none" and "outset"
///
/// # Base colors are red, green, blue,
/// # cyan, magenta, yellow, white and black.
/// [colors]
/// 	background = "black"
/// 	# If the value is an array, the first valid color will be used.
/// 	# If the terminal doesn't support custom color,
/// 	# non-base colors will be skipped.
/// 	shadow     = ["#000000", "black"]
/// 	view       = "#d3d7cf"
///
/// 	# Array and simple values have the same effect.
/// 	primary   = ["#111111"]
/// 	secondary = "#EEEEEE"
/// 	tertiary  = "#444444"
///
/// 	# Hex values can use lower or uppercase.
/// 	# (base color MUST be lowercase)
/// 	title_primary   = "#ff5555"
/// 	title_secondary = "#ffff55"
///
/// 	# Lower precision values can use only 3 digits.
/// 	highlight          = "#F00"
/// 	highlight_inactive = "#5555FF"
/// ```

/// Loads a theme and sets it as active.
pub fn load_theme<P: AsRef<Path>>(filename: P) -> Result<Theme, Error> {
    let content = {
        let mut content = String::new();
        let mut file = try!(File::open(filename));
        try!(file.read_to_string(&mut content));
        content
    };

    let mut parser = toml::Parser::new(&content);
    let table = match parser.parse() {
        Some(value) => value,
        None => return Err(Error::Parse),
    };

    let mut theme = Theme::default();
    theme.load(&table);
    theme.activate();

    Ok(theme)
}

/// Loads the default theme, and returns its representation.
pub fn load_default() -> Theme {
    let theme = Theme::default();
    theme.activate();
    theme
}

/// Loads a hexadecimal code
fn load_hex(s: &str) -> u16 {
    let mut sum = 0;
    for c in s.chars() {
        sum *= 16;
        sum += match c {
            n @ '0'...'9' => n as i16 - '0' as i16,
            n @ 'a'...'f' => n as i16 - 'a' as i16 + 10,
            n @ 'A'...'F' => n as i16 - 'A' as i16 + 10,
            _ => 0,
        };
    }

    sum as u16
}
