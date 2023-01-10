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
    pub fn dark(self) -> Color {
        Color::Dark(self)
    }

    /// Returns the light version of this base color.
    pub fn light(self) -> Color {
        Color::Light(self)
    }

    /// Returns an iterator on all possible base colors.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}

impl From<u8> for BaseColor {
    fn from(n: u8) -> Self {
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

impl Color {
    /// Creates a color from its ID in the 256 colors list.
    ///
    /// * Colors 0-7 are base dark colors.
    /// * Colors 8-15 are base light colors.
    /// * Colors 16-231 are rgb colors with 6 values per channel (216 colors).
    /// * Colors 232-255 are grayscale colors.
    pub fn from_256colors(n: u8) -> Self {
        if n < 8 {
            Color::Dark(BaseColor::from(n))
        } else if n < 16 {
            Color::Light(BaseColor::from(n))
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
    pub fn low_res(r: u8, g: u8, b: u8) -> Option<Self> {
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
        Some(match value {
            "dark black" | "black" => Color::Dark(BaseColor::Black),
            "dark red" | "red" => Color::Dark(BaseColor::Red),
            "dark green" | "green" => Color::Dark(BaseColor::Green),
            "dark yellow" | "yellow" => Color::Dark(BaseColor::Yellow),
            "dark blue" | "blue" => Color::Dark(BaseColor::Blue),
            "dark magenta" | "magenta" => Color::Dark(BaseColor::Magenta),
            "dark cyan" | "cyan" => Color::Dark(BaseColor::Cyan),
            "dark white" | "white" => Color::Dark(BaseColor::White),
            "light black" => Color::Light(BaseColor::Black),
            "light red" => Color::Light(BaseColor::Red),
            "light green" => Color::Light(BaseColor::Green),
            "light yellow" => Color::Light(BaseColor::Yellow),
            "light blue" => Color::Light(BaseColor::Blue),
            "light magenta" => Color::Light(BaseColor::Magenta),
            "light cyan" => Color::Light(BaseColor::Cyan),
            "light white" => Color::Light(BaseColor::White),
            "default" => Color::TerminalDefault,
            value => return parse_special(value),
        })
    }
}

impl FromStr for Color {
    type Err = super::NoSuchColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Color::parse(s).ok_or(super::NoSuchColor)
    }
}

fn parse_special(value: &str) -> Option<Color> {
    if let Some(value) = value.strip_prefix('#') {
        parse_hex(value)
    } else if let Some(value) = value.strip_prefix("0x") {
        parse_hex(value)
    } else if value.len() == 6 {
        parse_hex(value)
    } else if value.len() == 3 {
        // RGB values between 0 and 5 maybe?
        // Like 050 for green
        let rgb: Vec<_> =
            value.chars().map(|c| c as i16 - '0' as i16).collect();

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

fn parse_hex(value: &str) -> Option<Color> {
    // Compute per-color length, and amplitude
    let (l, multiplier) = match value.len() {
        6 => (2, 1),
        3 => (1, 17),
        _ => return None,
    };
    let r = load_hex(&value[0..l]) * multiplier;
    let g = load_hex(&value[l..2 * l]) * multiplier;
    let b = load_hex(&value[2 * l..3 * l]) * multiplier;

    Some(Color::Rgb(r as u8, g as u8, b as u8))
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

        assert_eq!(
            Color::parse("#abcdef"),
            Some(Color::Rgb(0xab, 0xcd, 0xef))
        );

        assert_eq!(
            Color::parse("0xFEDCBA"),
            Some(Color::Rgb(0xfe, 0xdc, 0xba))
        );
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
