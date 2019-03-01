/// One of the 8 base colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Note: the actual color used depends on the terminal configuration.
    Dark(BaseColor),

    /// Lighter version of a base color.
    ///
    /// Note: the actual color used depends on the terminal configuration.
    Light(BaseColor),

    /// True-color, 24-bit.
    Rgb(u8, u8, u8),

    /// Low-resolution
    ///
    /// Each value should be `<= 5` (you'll get panics otherwise).
    ///
    /// These 216 possible colors are part of the default color palette.
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

    /// Parse a string into a color.
    ///
    /// Examples:
    /// * `"red"` becomes `Color::Dark(BaseColor::Red)`
    /// * `"light green"` becomes `Color::Light(BaseColor::Green)`
    /// * `"default"` becomes `Color::TerminalDefault`
    /// * `"#123456"` becomes `Color::Rgb(0x12, 0x34, 0x56)`
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "black" => Color::Dark(BaseColor::Black),
            "red" => Color::Dark(BaseColor::Red),
            "green" => Color::Dark(BaseColor::Green),
            "yellow" => Color::Dark(BaseColor::Yellow),
            "blue" => Color::Dark(BaseColor::Blue),
            "magenta" => Color::Dark(BaseColor::Magenta),
            "cyan" => Color::Dark(BaseColor::Cyan),
            "white" => Color::Dark(BaseColor::White),
            "light black" => Color::Light(BaseColor::Black),
            "light red" => Color::Light(BaseColor::Red),
            "light green" => Color::Light(BaseColor::Green),
            "light yellow" => Color::Light(BaseColor::Yellow),
            "light blue" => Color::Light(BaseColor::Blue),
            "light magenta" => Color::Light(BaseColor::Magenta),
            "light cyan" => Color::Light(BaseColor::Cyan),
            "light white" => Color::Light(BaseColor::White),
            "default" => Color::TerminalDefault,
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
            let r = load_hex(&value[0..l]) * multiplier;
            let g = load_hex(&value[l..2 * l]) * multiplier;
            let b = load_hex(&value[2 * l..3 * l]) * multiplier;

            Some(Color::Rgb(r as u8, g as u8, b as u8))
        } else if value.len() == 3 {
            // RGB values between 0 and 5 maybe?
            // Like 050 for green
            let rgb: Vec<_> =
                value.chars().map(|c| c as i16 - '0' as i16).collect();

            assert_eq!(rgb.len(), 3);
            if rgb.iter().all(|&i| i >= 0 && i < 6) {
                Some(Color::RgbLowRes(
                    rgb[0] as u8,
                    rgb[1] as u8,
                    rgb[2] as u8,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Loads a hexadecimal code
fn load_hex(s: &str) -> u16 {
    let mut sum = 0;
    for c in s.chars() {
        sum *= 16;
        sum += match c {
            n @ '0'..='9' => n as i16 - '0' as i16,
            n @ 'a'..='f' => n as i16 - 'a' as i16 + 10,
            n @ 'A'..='F' => n as i16 - 'A' as i16 + 10,
            _ => 0,
        };
    }

    sum as u16
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_256_colors() {
        // Make sure Color::from_256colors never panics
        use super::Color;

        // TODO: use inclusive range when it gets stable
        for i in 0..256u16 {
            Color::from_256colors(i as u8);
        }
    }
}
