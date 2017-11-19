use theme::{BaseColor, Color};

#[cfg(feature = "ncurses")]
mod n;
#[cfg(feature = "ncurses")]
pub use self::n::*;

#[cfg(feature = "pancurses")]
mod pan;
#[cfg(feature = "pancurses")]
pub use self::pan::*;

fn split_u32(code: i32) -> Vec<u8> {
    (0..4).map(|i| ((code >> (8 * i)) & 0xFF) as u8).collect()
}

fn find_closest(color: &Color) -> i16 {
    match *color {
        Color::TerminalDefault => -1,
        Color::Dark(BaseColor::Black) => 0,
        Color::Dark(BaseColor::Red) => 1,
        Color::Dark(BaseColor::Green) => 2,
        Color::Dark(BaseColor::Yellow) => 3,
        Color::Dark(BaseColor::Blue) => 4,
        Color::Dark(BaseColor::Magenta) => 5,
        Color::Dark(BaseColor::Cyan) => 6,
        Color::Dark(BaseColor::White) => 7,
        Color::Light(BaseColor::Black) => 8,
        Color::Light(BaseColor::Red) => 9,
        Color::Light(BaseColor::Green) => 10,
        Color::Light(BaseColor::Yellow) => 11,
        Color::Light(BaseColor::Blue) => 12,
        Color::Light(BaseColor::Magenta) => 13,
        Color::Light(BaseColor::Cyan) => 14,
        Color::Light(BaseColor::White) => 15,
        Color::Rgb(r, g, b) => {
            let r = 6 * u16::from(r) / 256;
            let g = 6 * u16::from(g) / 256;
            let b = 6 * u16::from(b) / 256;
            (16 + 36 * r + 6 * g + b) as i16
        }
        Color::RgbLowRes(r, g, b) => i16::from(16 + 36 * r + 6 * g + b),
    }
}
