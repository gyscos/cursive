//! Module to handle colors and themes in the UI.

use std::io;
use std::io::Read;
use std::fs::File;
use std::path::Path;

use ncurses;
use toml;

/// Represents a colorpair from a Theme.
pub type ThemeColor = i16;

/// Application background, where no view is present.
pub const BACKGROUND: ThemeColor = 1;
/// Color used by view shadows. Only background matters.
pub const SHADOW: ThemeColor = 2;
/// Main text with default background.
pub const PRIMARY: ThemeColor = 3;
/// Secondary text color, with default background.
pub const SECONDARY: ThemeColor = 4;
/// Tertiary text color, with default background.
pub const TERTIARY: ThemeColor = 5;
/// Title text color with default background.
pub const TITLE_PRIMARY: ThemeColor = 6;
/// Alternative color for a title.
pub const TITLE_SECONDARY: ThemeColor = 7;
/// Alternate text with highlight background.
pub const HIGHLIGHT: ThemeColor = 8;
/// Highlight color for inactive views (not in focus).
pub const HIGHLIGHT_INACTIVE: ThemeColor = 9;


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

/// Defines a color as used by a theme.
///
/// Can be created from rgb values, or from a preset color.
pub struct Color {
    /// Red component. Between 0 and 1000.
    pub r: i16,
    /// Green component. Between 0 and 1000.
    pub g: i16,
    /// Blue component. Between 0 and 1000.
    pub b: i16,
}

impl Color {
    /// Returns a new color from the given values.
    pub fn new(r:i16, g:i16, b:i16) -> Self {
        Color {
            r:r,
            g:g,
            b:b,
        }
    }

    /// Returns a black color: (0,0,0).
    pub fn black() -> Self {
        Color::new(0,0,0)
    }

    /// Returns a white color: (1000,1000,1000).
    pub fn white() -> Self {
        Color::new(1000,1000,1000)
    }

    /// Returns a red color: (1000,0,0).
    pub fn red() -> Self {
        Color::new(1000,0,0)
    }

    /// Returns a green color: (0,1000,0).
    pub fn green() -> Self {
        Color::new(0,1000,0)
    }

    /// Returns a blue color: (0,0,1000).
    pub fn blue() -> Self {
        Color::new(0,0,1000)
    }

    /// Returns a light gray color: (700,700,700).
    pub fn gray() -> Self {
        Color::new(700,700,700)
    }

    /// Returns a dark gray color: (300,300,300).
    pub fn dark_gray() -> Self {
        Color::new(300,300,300)
    }

    /// Returns a yellow color: (1000,1000,0).
    pub fn yellow() -> Self {
        Color::new(1000,1000,0)
    }

    /// Returns a cyan color: (0,1000,1000).
    pub fn cyan() -> Self {
        Color::new(0,1000,1000)
    }

    /// Returns a magenta color: (1000,0,1000).
    pub fn magenta() -> Self {
        Color::new(1000,0,1000)
    }

    /// Applies the current color to the given color id
    fn init_color(&self, color_id: ThemeColor) {
        ncurses::init_color(color_id, self.r, self.g, self.b);
    }

    /// Read a string value into the current color.
    fn load_color(&mut self, s: &str) {

        if s.len() == 0 {
            panic!("Cannot read color: empty string");
        }

        if s.starts_with("#") {
            let s = &s[1..];
            // HTML-style
            let l = match s.len() {
                6 => 2,
                3 => 1,
                _ => panic!("Cannot parse color: {}", s),
            };

            self.r = (load_hex(&s[0*l..1*l]) as i32 * 1000 / 255) as i16;
            self.g = (load_hex(&s[1*l..2*l]) as i32 * 1000 / 255) as i16;
            self.b = (load_hex(&s[2*l..3*l]) as i32 * 1000 / 255) as i16;
        } else {
            // Unknown color. Panic.
            panic!("Cannot parse color: {}", s);
        }
    }
}

type ColorId = i16;

const BACKGROUND_COLOR: i16 = 8;
const SHADOW_COLOR: i16 = 9;
const VIEW_COLOR: i16 = 10;
const PRIMARY_COLOR: i16 = 11;
const SECONDARY_COLOR: i16 = 12;
const TERTIARY_COLOR: i16 = 13;
const TITLE_PRIMARY_COLOR: i16 = 14;
const TITLE_SECONDARY__COLOR: i16 = 15;
const HIGHLIGHT_COLOR: i16 = 16;
const HIGHLIGHT_INACTIVE_COLOR: i16 = 17;

/// Defines colors for various situations.
pub struct Theme {
    pub background: Color,
    pub shadow: Color,

    pub view_background: Color,

    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub title_primary: Color,
    pub title_secondary: Color,
    pub highlight: Color,
    pub highlight_inactive: Color,
}

impl Theme {
    /// Apply the theme. Effective immediately.
    pub fn apply(&self) {
        // First, init the colors
        self.background.init_color(BACKGROUND_COLOR);
        self.shadow.init_color(SHADOW_COLOR);
        self.view_background.init_color(VIEW_COLOR);
        self.primary.init_color(PRIMARY_COLOR);
        self.secondary.init_color(SECONDARY_COLOR);
        self.tertiary.init_color(TERTIARY_COLOR);
        self.title_primary.init_color(TITLE_PRIMARY_COLOR);
        self.title_secondary.init_color(TITLE_SECONDARY__COLOR);
        self.highlight.init_color(HIGHLIGHT_COLOR);
        self.highlight_inactive.init_color(HIGHLIGHT_INACTIVE_COLOR);

        // Then init the pairs
        ncurses::init_pair(BACKGROUND, BACKGROUND_COLOR, BACKGROUND_COLOR);
        ncurses::init_pair(SHADOW, SHADOW_COLOR, SHADOW_COLOR);
        ncurses::init_pair(PRIMARY, PRIMARY_COLOR, VIEW_COLOR);
        ncurses::init_pair(SECONDARY, SECONDARY_COLOR, VIEW_COLOR);
        ncurses::init_pair(TERTIARY, TERTIARY_COLOR, VIEW_COLOR);
        ncurses::init_pair(TITLE_PRIMARY, TITLE_PRIMARY_COLOR, VIEW_COLOR);
        ncurses::init_pair(TITLE_SECONDARY, TITLE_SECONDARY__COLOR, VIEW_COLOR);
        ncurses::init_pair(HIGHLIGHT, VIEW_COLOR, HIGHLIGHT_COLOR);
        ncurses::init_pair(HIGHLIGHT_INACTIVE, VIEW_COLOR, HIGHLIGHT_INACTIVE_COLOR);
    }

    /// Returns the default theme.
    pub fn default() -> Theme {
        Theme {
            background: Color::blue(),
            shadow: Color::black(),
            view_background: Color::gray(),
            primary: Color::black(),
            secondary: Color::white(),
            tertiary: Color::dark_gray(),
            title_primary: Color::red(),
            title_secondary: Color::yellow(),
            highlight: Color::red(),
            highlight_inactive: Color::blue(),
        }
    }

    /// Load a single value into a color
    fn load(color: &mut Color, value: Option<&toml::Value>) {
        match value {
            Some(&toml::Value::String(ref s)) => color.load_color(s),
            _ => (),
        }
    }

    /// Loads the color content from a TOML configuration
    fn load_colors(&mut self, table: toml::Table) {
        Theme::load(&mut self.background, table.get("background"));
        Theme::load(&mut self.shadow, table.get("shadow"));
        Theme::load(&mut self.view_background, table.get("view"));
        Theme::load(&mut self.primary, table.get("primary"));
        Theme::load(&mut self.secondary, table.get("secondary"));
        Theme::load(&mut self.tertiary, table.get("tertiary"));
        Theme::load(&mut self.title_primary, table.get("title_primaryy"));
        Theme::load(&mut self.title_secondary, table.get("title_secondary"));
        Theme::load(&mut self.highlight, table.get("highlight"));
        Theme::load(&mut self.highlight_inactive, table.get("highlight_primary"));
    }
}

/// Loads the default theme.
pub fn load_default() {
    Theme::default().apply();
}

/// Loads a simple default theme using built-in colors.
pub fn load_legacy() {
    ncurses::init_pair(BACKGROUND, ncurses::COLOR_WHITE, ncurses::COLOR_BLUE);
    ncurses::init_pair(SHADOW, ncurses::COLOR_WHITE, ncurses::COLOR_BLACK);
    ncurses::init_pair(PRIMARY, ncurses::COLOR_BLACK, ncurses::COLOR_WHITE);
    ncurses::init_pair(SECONDARY, ncurses::COLOR_BLUE, ncurses::COLOR_WHITE);
    ncurses::init_pair(TERTIARY, ncurses::COLOR_WHITE, ncurses::COLOR_WHITE);
    ncurses::init_pair(TITLE_PRIMARY, ncurses::COLOR_RED, ncurses::COLOR_WHITE);
    ncurses::init_pair(TITLE_SECONDARY, ncurses::COLOR_YELLOW, ncurses::COLOR_WHITE);
    ncurses::init_pair(HIGHLIGHT, ncurses::COLOR_WHITE, ncurses::COLOR_RED);
    ncurses::init_pair(HIGHLIGHT_INACTIVE, ncurses::COLOR_WHITE, ncurses::COLOR_BLUE);
}

/// Possible error returned when loading a theme.
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

/// Loads a theme file.
///
/// The file should be a toml file containing any of the following entries
/// (missing entries will have default value):
///
/// - `background`
/// - `shadow`
/// - `view`
/// - `primary`
/// - `secondary`
/// - `tertiary`
/// - `title_primary`
/// - `title_secondary`
/// - `highlight`
/// - `highlight_inactive`
///
/// Here is an example file:
///
/// ```
/// background = "#5555FF"
/// shadow     = "#000000"
/// view       = "#888888"
///
/// primary   = "#111111"
/// secondary = "#EEEEEE"
/// tertiary  = "#444444"
///
/// title_primary   = "#ff5555"
/// title_secondary = "#ffff55"
///
/// highlight          = "#FF0000"
/// highlight_inactive = "#5555FF"
/// ```
pub fn load_theme<P: AsRef<Path>>(filename: P) -> Result<(),Error> {
    let mut file = try!(File::open(filename));
    let mut content = String::new();

    try!(file.read_to_string(&mut content));

    let mut parser = toml::Parser::new(&content);
    let value = match parser.parse() {
        Some(value) => value,
        None => return Err(Error::ParseError),
    };

    let mut theme = Theme::default();
    theme.load_colors(value);

    theme.apply();

    Ok(())
}
