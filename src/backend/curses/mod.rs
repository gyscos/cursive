//! Common module for the ncurses and pancurses backends.
//!
//! Requires either of `ncurses-backend` or `pancurses-backend`.
#![cfg(any(feature = "ncurses-backend", feature = "pancurses-backend"))]

use hashbrown::HashMap;

use crate::event::{Event, Key};
use crate::theme::{BaseColor, Color, ColorPair};
use maplit::hashmap;

#[cfg(feature = "ncurses-backend")]
pub mod n;

#[cfg(feature = "pancurses-backend")]
pub mod pan;

fn split_i32(code: i32) -> Vec<u8> {
    (0..4).map(|i| ((code >> (8 * i)) & 0xFF) as u8).collect()
}

fn fill_key_codes<F>(target: &mut HashMap<i32, Event>, f: F)
where
    F: Fn(i32) -> Option<String>,
{
    let key_names = hashmap! {
        "DC" => Key::Del,
        "DN" => Key::Down,
        "END" => Key::End,
        "HOM" => Key::Home,
        "IC" => Key::Ins,
        "LFT" => Key::Left,
        "NXT" => Key::PageDown,
        "PRV" => Key::PageUp,
        "RIT" => Key::Right,
        "UP" => Key::Up,
    };

    for code in 512..1024 {
        let name = match f(code) {
            Some(name) => name,
            None => continue,
        };

        if !name.starts_with('k') {
            continue;
        }

        let (key_name, modifier) = name[1..].split_at(name.len() - 2);
        let key = match key_names.get(key_name) {
            Some(&key) => key,
            None => continue,
        };
        let event = match modifier {
            "3" => Event::Alt(key),
            "4" => Event::AltShift(key),
            "5" => Event::Ctrl(key),
            "6" => Event::CtrlShift(key),
            "7" => Event::CtrlAlt(key),
            _ => continue,
        };
        target.insert(code, event);
    }
}

fn find_closest_pair(pair: ColorPair, max_colors: i16) -> (i16, i16) {
    (
        find_closest(pair.front, max_colors),
        find_closest(pair.back, max_colors),
    )
}

fn find_closest(color: Color, max_colors: i16) -> i16 {
    match color {
        Color::TerminalDefault => -1,
        Color::Dark(BaseColor::Black) => 0,
        Color::Dark(BaseColor::Red) => 1,
        Color::Dark(BaseColor::Green) => 2,
        Color::Dark(BaseColor::Yellow) => 3,
        Color::Dark(BaseColor::Blue) => 4,
        Color::Dark(BaseColor::Magenta) => 5,
        Color::Dark(BaseColor::Cyan) => 6,
        Color::Dark(BaseColor::White) => 7,
        Color::Light(BaseColor::Black) => 8 % max_colors,
        Color::Light(BaseColor::Red) => 9 % max_colors,
        Color::Light(BaseColor::Green) => 10 % max_colors,
        Color::Light(BaseColor::Yellow) => 11 % max_colors,
        Color::Light(BaseColor::Blue) => 12 % max_colors,
        Color::Light(BaseColor::Magenta) => 13 % max_colors,
        Color::Light(BaseColor::Cyan) => 14 % max_colors,
        Color::Light(BaseColor::White) => 15 % max_colors,
        Color::Rgb(r, g, b) if max_colors >= 256 => {
            // If r = g = b, it may be a grayscale value!
            if r == g && g == b && r != 0 && r < 250 {
                // Grayscale
                // (r = g = b) = 8 + 10 * n
                // (r - 8) / 10 = n
                //
                let n = (r - 8) / 10;
                i16::from(232 + n)
            } else {
                // Generic RGB
                let r = 6 * u16::from(r) / 256;
                let g = 6 * u16::from(g) / 256;
                let b = 6 * u16::from(b) / 256;
                (16 + 36 * r + 6 * g + b) as i16
            }
        }
        Color::Rgb(r, g, b) => {
            let r = if r > 127 { 1 } else { 0 };
            let g = if g > 127 { 1 } else { 0 };
            let b = if b > 127 { 1 } else { 0 };
            (r + 2 * g + 4 * b) as i16
        }
        Color::RgbLowRes(r, g, b) if max_colors >= 256 => {
            i16::from(16 + 36 * r + 6 * g + b)
        }
        Color::RgbLowRes(r, g, b) => {
            let r = if r > 2 { 1 } else { 0 };
            let g = if g > 2 { 1 } else { 0 };
            let b = if b > 2 { 1 } else { 0 };
            (r + 2 * g + 4 * b) as i16
        }
    }
}
