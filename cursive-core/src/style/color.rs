use enum_map::Enum;
use std::str::FromStr;

/// One of the 8 base colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum BaseColor {
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
}

impl BaseColor {
    /// Returns the regular (dark) version of this base color.
    pub const fn dark(self) -> Color {
        Color::Dark(self)
    }

    /// Returns the light version of this base color.
    pub const fn light(self) -> Color {
        Color::Light(self)
    }

    /// Returns an iterator on all possible base colors.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }

    /// Parse a string into a base color.
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Black" | "black" => BaseColor::Black,
            "Red" | "red" => BaseColor::Red,
            "Green" | "green" => BaseColor::Green,
            "Yellow" | "yellow" => BaseColor::Yellow,
            "Blue" | "blue" => BaseColor::Blue,
            "Magenta" | "magenta" => BaseColor::Magenta,
            "Cyan" | "cyan" => BaseColor::Cyan,
            "White" | "white" => BaseColor::White,
            _ => return None,
        })
    }

    /// Convert a `u8` into a `BaseColor`.
    ///
    /// For values 0 to 7, the right-most 3 bits map to (red, green, blue).
    ///
    /// Other values are considered modulo 8.
    pub const fn from_u8(n: u8) -> Self {
        match n % 8 {
            0 => BaseColor::Black,
            1 => BaseColor::Red,
            2 => BaseColor::Green,
            3 => BaseColor::Yellow,
            4 => BaseColor::Blue,
            5 => BaseColor::Magenta,
            6 => BaseColor::Cyan,
            7 => BaseColor::White,
            _ => unreachable!(),
        }
    }
}

impl From<u8> for BaseColor {
    fn from(n: u8) -> Self {
        Self::from_u8(n)
    }
}

/// RGB color.
///
/// If `T = u8` this is the usual 24-bit true color.
///
/// If `T = f32` this uses floats between 0 and 1.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Rgb<T = u8> {
    /// Red component.
    pub r: T,

    /// Green component.
    pub g: T,

    /// Blue component.
    pub b: T,
}

impl<T> From<[T; 3]> for Rgb<T> {
    fn from(o: [T; 3]) -> Self {
        let [r, g, b] = o;
        Self { r, g, b }
    }
}

impl From<u32> for Rgb<u8> {
    fn from(hex: u32) -> Self {
        Self::from_u32(hex)
    }
}

impl From<Rgb<u8>> for Rgb<f32> {
    fn from(o: Rgb<u8>) -> Self {
        o.as_f32()
    }
}

impl<T> Rgb<T> {
    /// Create a new Rgb from individual r, g and b values.
    pub fn new(r: T, g: T, b: T) -> Self {
        Self { r, g, b }
    }

    /// Run a closure on each component pair.
    pub fn zip_map<U, V>(a: Self, b: Rgb<U>, mut f: impl FnMut(T, U) -> V) -> Rgb<V> {
        Self::zip(a, b).map(|(a, b)| f(a, b))
    }

    /// Zip two Rgb into a single Rgb of tuples.
    pub fn zip<U>(a: Self, b: Rgb<U>) -> Rgb<(T, U)> {
        Rgb {
            r: (a.r, b.r),
            g: (a.g, b.g),
            b: (a.b, b.b),
        }
    }

    /// Apply a closure on each component.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Rgb<U> {
        Rgb {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }
}

impl Rgb<f32> {
    /// Casts each component to u8.
    ///
    /// Goes from `[0:1]` to `[0:255]` range.
    pub fn as_u8(self) -> Rgb<u8> {
        // Going from [0:1.0] to [0:255]
        // We add 0.5 to make sure the float is above the integer we want.
        self.map(|x| (0.5 + (x.clamp(0.0, 1.0) * 255.0).round()) as u8)
    }

    /// Convert to a Color.
    pub fn as_color(self) -> Color {
        self.as_u8().as_color()
    }
}

impl Rgb<(f32, f32)> {
    /// Interpolate each component individually.
    ///
    /// This is a simple linear interpolation per channel.
    pub fn interpolate(self, x: f32) -> Rgb<f32> {
        self.map(|(a, b)| a * (1f32 - x) + b * x)
    }
}

impl Rgb<(u8, u8)> {
    /// Cast each component to a tuple of f32.
    pub fn as_f32(self) -> Rgb<(f32, f32)> {
        self.map(|(x, y)| (x as f32 / 255.0, y as f32 / 255.0))
    }
}

impl Rgb<u8> {
    /// Convert to a Color.
    pub fn as_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    /// Return a Rgb using the lowest 24 bits.
    ///
    /// This can parse hex codes like `0xFF0000`.
    pub const fn from_u32(hex: u32) -> Self {
        let r = ((hex & 0xFF0000) >> 16) as u8;
        let g = ((hex & 0x00FF00) >> 8) as u8;
        let b = (hex & 0x0000FF) as u8;
        Self { r, g, b }
    }

    /// Cast each component to f32.
    pub fn as_f32(self) -> Rgb<f32> {
        self.map(|x| x as f32 / 255.0)
    }

    /// Returns a pure red RGB color.
    pub const fn red() -> Self {
        Self::from_u32(0xFF0000)
    }

    /// Returns an orange RGB color.
    pub const fn orange() -> Self {
        Self::from_u32(0xFFA500)
    }

    /// Returns a violet RGB color.
    pub const fn violet() -> Self {
        Self::from_u32(0x7F00FF)
    }

    /// Returns a turquoise color.
    pub const fn turquoise() -> Self {
        Self::from_u32(0x40E0D0)
    }

    /// Returns a pure green RGB color.
    pub const fn green() -> Self {
        Self::from_u32(0x00FF00)
    }

    /// Returns a pure blue RGB color.
    pub const fn blue() -> Self {
        Self::from_u32(0x0000FF)
    }

    /// Returns a yellow (red + green) RGB color.
    pub const fn yellow() -> Self {
        Self::from_u32(0xFFFF00)
    }
    /// Returns a magenta (red + blue) RGB color.
    pub const fn magenta() -> Self {
        Self::from_u32(0xFF00FF)
    }
    /// Returns a cyan (green + blue) RGB color.
    pub const fn cyan() -> Self {
        Self::from_u32(0x00FFFF)
    }
    /// Returns a pure white RGB color.
    pub const fn white() -> Self {
        Self::from_u32(0xFFFFFF)
    }
    /// Returns a pure black RGB color.
    pub const fn black() -> Self {
        Self::from_u32(0x000000)
    }
}

impl FromStr for Rgb<u8> {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "red" | "Red" => Ok(Self::red()),
            "green" | "Green" => Ok(Self::green()),
            "blue" | "Blue" => Ok(Self::blue()),
            "yellow" | "Yellow" => Ok(Self::yellow()),
            "magenta" | "Magenta" => Ok(Self::magenta()),
            "cyan" | "Cyan" => Ok(Self::cyan()),
            "white" | "White" => Ok(Self::white()),
            "black" | "Black" => Ok(Self::black()),
            s => {
                // Remove `#` or `0x` prefix
                let s = s
                    .strip_prefix('#')
                    .or_else(|| s.strip_prefix("0x"))
                    .unwrap_or(s);
                if let Some(rgb) = parse_hex(s) {
                    Ok(rgb)
                } else {
                    Err(super::NoSuchColor)
                }
            }
        }
    }
}

impl From<Rgb<u8>> for Color {
    fn from(rgb: Rgb<u8>) -> Self {
        rgb.as_color()
    }
}

/// Represents a color used by the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    /// Represents a color, preset by terminal.
    TerminalDefault,

    /// One of the 8 base colors.
    ///
    /// These colors should work on any terminal.
    ///
    /// Note: the actual color used depends on the terminal configuration.
    Dark(BaseColor),

    /// Lighter version of a base color.
    ///
    /// The native linux TTY usually doesn't support these colors, but almost
    /// all terminal emulators should.
    ///
    /// Note: the actual color used depends on the terminal configuration.
    Light(BaseColor),

    /// True-color, 24-bit.
    ///
    /// On terminals that don't support this, the color will be "downgraded"
    /// to the closest one available.
    Rgb(u8, u8, u8),

    /// Low-resolution color.
    ///
    /// Each value should be `<= 5` (you'll get panics otherwise).
    ///
    /// These 216 possible colors are part of the terminal color palette (256 colors).
    RgbLowRes(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::RgbLowRes(0, 0, 0)
    }
}

impl Color {
    /// Creates a color from its ID in the 256 colors list.
    ///
    /// * Colors 0-7 are base dark colors.
    /// * Colors 8-15 are base light colors.
    /// * Colors 16-231 are rgb colors with 6 values per channel (216 colors).
    /// * Colors 232-255 are grayscale colors.
    pub const fn from_256colors(n: u8) -> Self {
        if n < 8 {
            Color::Dark(BaseColor::from_u8(n))
        } else if n < 16 {
            Color::Light(BaseColor::from_u8(n))
        } else if n >= 232 {
            let n = n - 232;
            let value = 8 + 10 * n;

            // TODO: returns a Grayscale value?
            Color::Rgb(value, value, value)
        } else {
            let n = n - 16;
            // We support 6*6*6 = 216 colors here
            assert!(n < 216);

            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;

            // Each color is in the range [0, 5] (6 possible values)
            assert!(r < 6);
            assert!(g < 6);
            assert!(b < 6);

            Color::RgbLowRes(r, g, b)
        }
    }

    /// Creates a `Color::RgbLowRes` from the given values for red, green and
    /// blue.
    ///
    /// Returns `None` if any of the values exceeds 5.
    pub const fn low_res(r: u8, g: u8, b: u8) -> Option<Self> {
        if r <= 5 && g <= 5 && b <= 5 {
            Some(Color::RgbLowRes(r, g, b))
        } else {
            None
        }
    }

    /// Parse a string into a color.
    ///
    /// Examples:
    /// * `"red"` becomes `Color::Dark(BaseColor::Red)`
    /// * `"light green"` becomes `Color::Light(BaseColor::Green)`
    /// * `"default"` becomes `Color::TerminalDefault`
    /// * `"#123456"` becomes `Color::Rgb(0x12, 0x34, 0x56)`
    pub fn parse(value: &str) -> Option<Self> {
        // These values cannot be prefixed.
        if value == "default" || value == "terminal default" {
            return Some(Color::TerminalDefault);
        }

        // "light " prefix?
        if let Some(base) = value.strip_prefix("light ").and_then(BaseColor::parse) {
            return Some(Color::Light(base));
        }

        // "dark " prefix is optional.
        if let Some(base) = BaseColor::parse(value.strip_prefix("dark ").unwrap_or(value)) {
            return Some(Color::Dark(base));
        }

        // This includes hex colors
        if let Some(color) = parse_hex_color(value) {
            return Some(color);
        }

        None
    }
}

impl FromStr for BaseColor {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(super::NoSuchColor)
    }
}

impl FromStr for Color {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(super::NoSuchColor)
    }
}

/// Parses a hex representation of a color.
///
/// Optionally prefixed with `#` or `0x`.
fn parse_hex_color(value: &str) -> Option<Color> {
    if let Some(value) = value.strip_prefix('#') {
        parse_hex(value).map(Color::from)
    } else if let Some(value) = value.strip_prefix("0x") {
        parse_hex(value).map(Color::from)
    } else if value.len() == 6 {
        parse_hex(value).map(Color::from)
    } else if value.len() == 3 {
        // RGB values between 0 and 5 maybe?
        // Like 050 for green
        let rgb: Vec<_> = value.chars().map(|c| c as i16 - '0' as i16).collect();

        assert_eq!(rgb.len(), 3);
        if rgb.iter().all(|i| (0..6).contains(i)) {
            Some(Color::RgbLowRes(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8))
        } else {
            None
        }
    } else {
        None
    }
}

/// This parses a purely hex string (either rrggbb or rgb) into a color.
fn parse_hex(value: &str) -> Option<Rgb<u8>> {
    // Compute per-color length, and amplitude
    let (l, multiplier) = match value.len() {
        6 => (2, 1),
        3 => (1, 17),
        _ => return None,
    };
    let r = load_hex(&value[0..l]) * multiplier;
    let g = load_hex(&value[l..2 * l]) * multiplier;
    let b = load_hex(&value[2 * l..3 * l]) * multiplier;

    Some(Rgb::new(r as u8, g as u8, b as u8))
}

/// Loads a hexadecimal code
fn load_hex(s: &str) -> u16 {
    s.chars()
        .rev()
        .filter_map(|c| {
            Some(match c {
                '0'..='9' => c as u16 - '0' as u16,
                'a'..='f' => c as u16 - 'a' as u16 + 10,
                'A'..='F' => c as u16 - 'A' as u16 + 10,
                other => {
                    log::warn!(
                        "Invalid character `{}` in hexadecimal value `{}`.",
                        other,
                        s
                    );
                    return None;
                }
            })
        })
        .enumerate()
        .map(|(i, c)| c * 16u16.pow(i as u32))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn test_256_colors() {
        // Make sure Color::from_256colors never panics

        for i in 0..=255u8 {
            Color::from_256colors(i);
        }
    }

    #[test]
    fn test_parse() {
        assert_eq!(Color::parse("#fff"), Some(Color::Rgb(255, 255, 255)));

        assert_eq!(Color::parse("#abcdef"), Some(Color::Rgb(0xab, 0xcd, 0xef)));

        assert_eq!(Color::parse("0xFEDCBA"), Some(Color::Rgb(0xfe, 0xdc, 0xba)));
    }

    #[test]
    fn test_low_res() {
        // Make sure Color::low_res always works with valid ranges.
        for r in 0..=5 {
            for g in 0..=5 {
                for b in 0..=5 {
                    assert!(
                        Color::low_res(r, g, b).is_some(),
                        "Could not create lowres color {r}:{g}:{b}",
                    );
                }
            }
        }

        for r in 6..=10 {
            for g in 6..=10 {
                for b in 6..=10 {
                    assert_eq!(
                        Color::low_res(r, g, b),
                        None,
                        "Created invalid lowres color {r}:{g}:{b}",
                    );
                }
            }
        }
    }
}
