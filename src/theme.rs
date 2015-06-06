//! Module to handle colors and themes in the UI.

use std::io;
use std::io::Read;
use std::fs::File;
use std::path::Path;

use ncurses;
use toml;

/// Represents the color of a character and its background.
#[derive(Clone,Copy)]
pub enum ColorPair {
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

impl ColorPair {
    /// Returns the ncurses pair ID associated with this color pair.
    pub fn ncurses_id(self) -> i16 {
        match self {
            ColorPair::Background => 1,
            ColorPair::Shadow => 2,
            ColorPair::Primary => 3,
            ColorPair::Secondary => 4,
            ColorPair::Tertiary => 5,
            ColorPair::TitlePrimary => 6,
            ColorPair::TitleSecondary => 7,
            ColorPair::Highlight => 8,
            ColorPair::HighlightInactive => 9,
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
    pub colors: ColorStyle,
}

impl Theme {
    fn default() -> Theme {
        Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            colors: ColorStyle {
                background: Color::blue(),
                shadow: Color::black(),
                view: Color::white(),
                primary: Color::black(),
                secondary: Color::blue(),
                tertiary: Color::white(),
                title_primary: Color::red(),
                title_secondary: Color::yellow(),
                highlight: Color::red(),
                highlight_inactive: Color::blue(),
            },
        }
    }

    fn load(&mut self, table: &toml::Table) {
        match table.get("shadow") {
            Some(&toml::Value::Boolean(shadow)) => self.shadow = shadow,
            _ => (),
        }

        match table.get("borders") {
            Some(&toml::Value::String(ref borders)) => match BorderStyle::from(borders) {
                Some(borders) => self.borders = borders,
                None => (),
            },
            _ => (),
        }

        match table.get("colors") {
            Some(&toml::Value::Table(ref table)) => self.colors.load(table),
            _ => (),
        }
    }

    fn apply(&self) {
        Theme::apply_color(ColorPair::Background, &self.colors.background, &self.colors.background);
        Theme::apply_color(ColorPair::Shadow, &self.colors.shadow, &self.colors.shadow);
        Theme::apply_color(ColorPair::Primary, &self.colors.primary, &self.colors.view);
        Theme::apply_color(ColorPair::Secondary, &self.colors.secondary, &self.colors.view);
        Theme::apply_color(ColorPair::Tertiary, &self.colors.tertiary, &self.colors.view);
        Theme::apply_color(ColorPair::TitlePrimary, &self.colors.title_primary, &self.colors.view);
        Theme::apply_color(ColorPair::TitleSecondary, &self.colors.title_secondary, &self.colors.view);
        Theme::apply_color(ColorPair::Highlight, &self.colors.view, &self.colors.highlight);
        Theme::apply_color(ColorPair::HighlightInactive, &self.colors.view, &self.colors.highlight_inactive);
    }

    fn apply_color(pair: ColorPair, front: &Color, back: &Color) {
        ncurses::init_pair(pair.ncurses_id(), front.id, back.id);
    }
}

/// Specifies how View borders should be drawn.
#[derive(Clone,Copy,Debug)]
pub enum BorderStyle {
    /// Don't draw any border.
    NoBorder,
    /// Simple borders.
    Simple,
    /// Outset borders with a 3d effect.
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

/// Represents the colors the application will use in various situations.
#[derive(Clone,Debug)]
pub struct ColorStyle {
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

impl ColorStyle {
    fn load(&mut self, table: &toml::Table) {
        let mut new_id = 8;

        self.background.load(table, "background", &mut new_id);
        self.shadow.load(table, "shadow", &mut new_id);
        self.view.load(table, "view", &mut new_id);
        self.primary.load(table, "primary", &mut new_id);
        self.secondary.load(table, "secondary", &mut new_id);
        self.tertiary.load(table, "tertiary", &mut new_id);
        self.title_primary.load(table, "title_primary", &mut new_id);
        self.title_secondary.load(table, "title_secondary", &mut new_id);
        self.highlight.load(table, "highlight", &mut new_id);
        self.highlight_inactive.load(table, "highlight_inactive", &mut new_id);
    }
}

/// Represents a color used by the theme.
#[derive(Clone,Debug)]
pub struct Color {
    /// Color ID used by ncurses.
    pub id: i16,
}

impl Color {
    /// Return the rgb values used by the color.
    pub fn rgb(&self) -> (i16,i16,i16) {
        let (mut r, mut g, mut b) = (0,0,0);

        ncurses::color_content(self.id, &mut r, &mut g, &mut b);

        (r,g,b)
    }
}

/// Possible error returned when loading a theme.
#[derive(Debug)]
pub enum Error {
    /// An error occured when reading the file.
    IoError(io::Error),
    /// An error occured when parsing the toml content.
    ParseError,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl Color {
    fn parse(value: &str, new_id: &mut i16) -> Option<Self> {
        let color = if value == "black" {
            Color::black()
        } else if value == "red" {
            Color::red()
        } else if value == "green" {
            Color::green()
        } else if value == "yellow" {
            Color::yellow()
        } else if value == "blue" {
            Color::blue()
        } else if value == "magenta" {
            Color::magenta()
        } else if value == "cyan" {
            Color::cyan()
        } else if value == "white" {
            Color::white()
        } else {
            // Let's make a new color
            return Color::make_new(value, new_id);
        };

        Some(color)
    }

    fn load(&mut self, table: &toml::Table, key: &str, new_id: &mut i16) {
        match table.get(key) {
            Some(&toml::Value::String(ref value)) => { self.load_value(value, new_id); },
            Some(&toml::Value::Array(ref array)) => for color in array.iter() {
                match color {
                    &toml::Value::String(ref color) => if self.load_value(color, new_id) { return; },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    fn load_value(&mut self, value: &str, new_id: &mut i16) -> bool {
        match Color::parse(value, new_id) {
            Some(color) => self.id = color.id,
            None => return false,
        }
        true
    }

    fn make_new(value: &str, new_id: &mut i16) -> Option<Self> {
        // if !ncurses::can_change_color() {
        if !ncurses::has_colors() {
            return None;
        }

        if !value.starts_with("#") {
            return None;
        }

        let s = &value[1..];
        let (l,max) = match s.len() {
            6 => (2,255),
            3 => (1,15),
            _ => panic!("Cannot parse color: {}", s),
        };

        let r = (load_hex(&s[0*l..1*l]) as i32 * 1000 / max) as i16;
        let g = (load_hex(&s[1*l..2*l]) as i32 * 1000 / max) as i16;
        let b = (load_hex(&s[2*l..3*l]) as i32 * 1000 / max) as i16;

        ncurses::init_color(*new_id, r, g, b);

        let color = Color { id: *new_id };
        *new_id += 1;

        Some(color)
    }


    pub fn black() -> Self {
        Color { id: 0 }
    }
    pub fn red() -> Self {
        Color { id: 1 }
    }
    pub fn green() -> Self {
        Color { id: 2 }
    }
    pub fn yellow() -> Self {
        Color { id: 3 }
    }
    pub fn blue() -> Self {
        Color { id: 4 }
    }
    pub fn magenta() -> Self {
        Color { id: 5 }
    }
    pub fn cyan() -> Self {
        Color { id: 6 }
    }
    pub fn white() -> Self {
        Color { id: 7 }
    }
}


/// Loads the default theme, and returns its representation.
pub fn load_default() -> Theme {
    let theme = Theme::default();
    theme.apply();
    theme
}

/// Loads a theme file, and returns its representation if everything worked well.
///
/// The file should be a toml file. All fields are optional. Here is are the possible entries:
///
/// ```text
/// # Every field in a theme file is optional.
/// 
/// shadow = false
/// borders = "simple", # Alternatives are "none" and "outset"
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

pub fn load_theme<P: AsRef<Path>>(filename: P) -> Result<Theme,Error> {
    let content = {
        let mut content = String::new();
        let mut file = try!(File::open(filename));
        try!(file.read_to_string(&mut content));
        content
    };

    let mut parser = toml::Parser::new(&content);
    let table = match parser.parse() {
        Some(value) => value,
        None => return Err(Error::ParseError),
    };

    let mut theme = Theme::default();
    theme.load(&table);
    theme.apply();

    Ok(theme)
}

/// Loads a hexadecimal code
fn load_hex(s: &str) -> i16 {
    let mut sum = 0;
    for c in s.chars() {
        sum *= 16;
        sum += match c {
            n @ '0' ... '9' => n as i16 - '0' as i16,
            n @ 'a' ... 'f' => n as i16 - 'a' as i16 + 10,
            n @ 'A' ... 'F' => n as i16 - 'A' as i16 + 10,
            _ => 0,
        };
    }

    sum
}
