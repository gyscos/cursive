use theme::{BaseColor, Color};

#[cfg(feature = "cursive_ncurses")]
mod n;
#[cfg(feature = "cursive_ncurses")]
pub use self::n::*;

#[cfg(feature = "cursive_pancurses")]
mod pan;
#[cfg(feature = "cursive_pancurses")]
pub use self::pan::*;


fn find_closest(color: &Color) -> u8 {
    match *color {
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
            let r = 6 * r as u16 / 256;
            let g = 6 * g as u16 / 256;
            let b = 6 * b as u16 / 256;
            (16 + 36 * r + 6 * g + b) as u8
        }
        Color::RgbLowRes(r, g, b) => (16 + 36 * r + 6 * g + b) as u8,
    }
}
