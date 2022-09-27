//! Common module for the ncurses and pancurses backends.
//!
//! Requires either of `ncurses-backend` or `pancurses-backend`.
#![cfg(any(feature = "ncurses-backend", feature = "pancurses-backend"))]

use crate::event::{Event, Key};
use crate::theme::{BaseColor, Color, ColorPair};
use maplit::hashmap;

pub mod n;
pub mod pan;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Split a i32 into individual bytes, little endian (least significant byte first).
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

/// Finds the closest index in the 256-color palette.
///
/// If `max_colors` is less than 256 (like 8 or 16), the color will be
/// downgraded to the closest one available.
fn find_closest(color: Color, max_colors: i16) -> i16 {
    let max_colors = std::cmp::max(max_colors, 8);
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
            // Grayscale colors have a bit higher resolution than the rest of
            // the palette, so if we can use it we should!
            //
            // r=g=b < 8 should go to pure black instead.
            // r=g=b >= 247 should go to pure white.

            // TODO: project almost-gray colors as well?
            if r == g && g == b && (8..247).contains(&r) {
                // The grayscale palette says the colors 232+n are:
                // (r = g = b) = 8 + 10 * n
                // With 0 <= n <= 23. This gives:
                // (r - 8) / 10 = n
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
            // Have to hack it down to 8 colors.
            let r = i16::from(r > 127);
            let g = i16::from(g > 127);
            let b = i16::from(b > 127);
            r + 2 * g + 4 * b
        }
        Color::RgbLowRes(r, g, b) if max_colors >= 256 => {
            i16::from(16 + 36 * r + 6 * g + b)
        }
        Color::RgbLowRes(r, g, b) => {
            let r = i16::from(r > 2);
            let g = i16::from(g > 2);
            let b = i16::from(b > 2);
            r + 2 * g + 4 * b
        }
    }
}
