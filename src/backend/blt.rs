extern crate bear_lib_terminal;

use ::backend;
use ::event::{Event, Key};
use self::bear_lib_terminal::Color as BltColor;
use self::bear_lib_terminal::geometry::Size;
use self::bear_lib_terminal::terminal::{self, Event as BltEvent, KeyCode};
use std::collections::BTreeMap;
use ::theme::{BaseColor, Color, ColorStyle, Effect};

pub struct Concrete {
    colours: BTreeMap<i16, (BltColor, BltColor)>,
}

impl backend::Backend for Concrete {
    fn init() -> Self {
        terminal::open("Cursive", 80, 24);
        terminal::set(terminal::config::Window::empty().resizeable(true));

        Concrete { colours: BTreeMap::new() }
    }

    fn finish(&mut self) {
        terminal::close();
    }

    fn init_color_style(&mut self, style: ColorStyle, foreground: &Color,
                        background: &Color) {
        self.colours.insert(style.id(),
                            (colour_to_blt_colour(foreground),
                             colour_to_blt_colour(background)));
    }

    fn with_color<F: FnOnce()>(&self, color: ColorStyle, f: F) {
        let (fg, bg) = self.colours[&color.id()];
        terminal::with_colors(fg, bg, f);
    }

    fn with_effect<F: FnOnce()>(&self, effect: Effect, f: F) {
        match effect {
            Effect::Simple => f(),
            // TODO: how to do this correctly?`
            //       BLT itself doesn't do this kind of thing,
            //       we'd need the colours in our position,
            //       but `f()` can do whatever
            Effect::Reverse => {
                terminal::with_colors(BltColor::from_rgb(0, 0, 0),
                                      BltColor::from_rgb(255, 255, 255),
                                      f)
            }
        }
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> (usize, usize) {
        let Size { width, height } = terminal::state::size();
        (width as usize, height as usize)
    }

    fn clear(&self) {
        terminal::clear(None);
    }

    fn refresh(&mut self) {
        terminal::refresh();
    }

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        terminal::print_xy(x as i32, y as i32, text);
    }

    fn set_refresh_rate(&mut self, _: u32) {
        // TODO: unsupported
    }

    fn poll_event(&self) -> Event {
        // TODO: we could add backend-specific controls here.
        // Ex: ctrl+mouse wheel cause window cellsize to change
        if let Some(ev) = terminal::wait_event() {
            match ev {
                BltEvent::Close => Event::Exit,
                BltEvent::Resize { .. } => Event::WindowResize,
                // TODO: mouse support
                BltEvent::MouseMove { .. } => Event::Refresh,
                BltEvent::MouseScroll { .. } => Event::Refresh,
                BltEvent::KeyPressed { key, ctrl, shift } => {
                    blt_keycode_to_ev(key, shift, ctrl)
                }
                // TODO: there's no Key::Shift/Ctrl for w/e reason
                BltEvent::ShiftPressed => Event::Refresh,
                BltEvent::ControlPressed => Event::Refresh,
                // TODO: what should we do here?
                BltEvent::KeyReleased { .. } |
                BltEvent::ShiftReleased |
                BltEvent::ControlReleased => Event::Refresh,
            }
        } else {
            Event::Refresh
        }
    }
}

fn colour_to_blt_colour(clr: &Color) -> BltColor {
    let (r, g, b) = match *clr {
        // Colours taken from
        // https://en.wikipedia.org/wiki/ANSI_escape_code#Colors
        Color::Dark(BaseColor::Black) => (0, 0, 0),
        Color::Dark(BaseColor::Red) => (170, 0, 0),
        Color::Dark(BaseColor::Green) => (0, 170, 0),
        Color::Dark(BaseColor::Yellow) => (170, 85, 0),
        Color::Dark(BaseColor::Blue) => (0, 0, 170),
        Color::Dark(BaseColor::Magenta) => (170, 0, 170),
        Color::Dark(BaseColor::Cyan) => (0, 170, 170),
        Color::Dark(BaseColor::White) => (170, 170, 170),

        Color::Light(BaseColor::Black) => (85, 85, 85),
        Color::Light(BaseColor::Red) => (255, 85, 85),
        Color::Light(BaseColor::Green) => (85, 255, 85),
        Color::Light(BaseColor::Yellow) => (255, 255, 85),
        Color::Light(BaseColor::Blue) => (85, 85, 255),
        Color::Light(BaseColor::Magenta) => (255, 85, 255),
        Color::Light(BaseColor::Cyan) => (85, 255, 255),
        Color::Light(BaseColor::White) => (255, 255, 255),

        Color::Rgb(r, g, b) => (r, g, b),
        Color::RgbLowRes(r, g, b) => {
            ((r as f32 / 5.0 * 255.0) as u8,
             (g as f32 / 5.0 * 255.0) as u8,
             (b as f32 / 5.0 * 255.0) as u8)
        }
    };
    BltColor::from_rgb(r, g, b)
}

fn blt_keycode_to_ev(kc: KeyCode, shift: bool, ctrl: bool) -> Event {
    match kc {
        KeyCode::F1 | KeyCode::F2 | KeyCode::F3 | KeyCode::F4 |
        KeyCode::F5 | KeyCode::F6 | KeyCode::F7 | KeyCode::F8 |
        KeyCode::F9 | KeyCode::F10 | KeyCode::F11 | KeyCode::F12 |
        KeyCode::Enter | KeyCode::Escape | KeyCode::Backspace |
        KeyCode::Tab | KeyCode::Pause | KeyCode::Insert | KeyCode::Home |
        KeyCode::PageUp | KeyCode::Delete | KeyCode::End |
        KeyCode::PageDown | KeyCode::Right | KeyCode::Left |
        KeyCode::Down | KeyCode::Up => {
            match (shift, ctrl) {
                (true, true) => Event::CtrlShift(blt_keycode_to_key(kc)),
                (true, false) => Event::Shift(blt_keycode_to_key(kc)),
                (false, true) => Event::Ctrl(blt_keycode_to_key(kc)),
                (false, false) => Event::Key(blt_keycode_to_key(kc)),
            }
        }
        // TODO: mouse support
        KeyCode::MouseLeft | KeyCode::MouseRight | KeyCode::MouseMiddle |
        KeyCode::MouseFourth | KeyCode::MouseFifth => Event::Refresh,
        KeyCode::A | KeyCode::B | KeyCode::C | KeyCode::D | KeyCode::E |
        KeyCode::F | KeyCode::G | KeyCode::H | KeyCode::I | KeyCode::J |
        KeyCode::K | KeyCode::L | KeyCode::M | KeyCode::N | KeyCode::O |
        KeyCode::P | KeyCode::Q | KeyCode::R | KeyCode::S | KeyCode::T |
        KeyCode::U | KeyCode::V | KeyCode::W | KeyCode::X | KeyCode::Y |
        KeyCode::Z | KeyCode::Row1 | KeyCode::Row2 | KeyCode::Row3 |
        KeyCode::Row4 | KeyCode::Row5 | KeyCode::Row6 | KeyCode::Row7 |
        KeyCode::Row8 | KeyCode::Row9 | KeyCode::Row0 | KeyCode::Grave |
        KeyCode::Minus | KeyCode::Equals | KeyCode::LeftBracket |
        KeyCode::RightBracket | KeyCode::Backslash | KeyCode::Semicolon |
        KeyCode::Apostrophe | KeyCode::Comma | KeyCode::Period |
        KeyCode::Slash | KeyCode::Space | KeyCode::NumDivide |
        KeyCode::NumMultiply | KeyCode::NumMinus | KeyCode::NumPlus |
        KeyCode::NumEnter | KeyCode::NumPeriod | KeyCode::Num1 |
        KeyCode::Num2 | KeyCode::Num3 | KeyCode::Num4 | KeyCode::Num5 |
        KeyCode::Num6 | KeyCode::Num7 | KeyCode::Num8 | KeyCode::Num9 |
        KeyCode::Num0 => {
            if ctrl {
                Event::CtrlChar(blt_keycode_to_char(kc, shift))
            } else {
                Event::Char(blt_keycode_to_char(kc, shift))
            }
        }
    }
}

fn blt_keycode_to_char(kc: KeyCode, shift: bool) -> char {
    match kc {
        KeyCode::A => if shift { 'A' } else { 'a' },
        KeyCode::B => if shift { 'B' } else { 'b' },
        KeyCode::C => if shift { 'C' } else { 'c' },
        KeyCode::D => if shift { 'D' } else { 'd' },
        KeyCode::E => if shift { 'E' } else { 'e' },
        KeyCode::F => if shift { 'F' } else { 'f' },
        KeyCode::G => if shift { 'G' } else { 'g' },
        KeyCode::H => if shift { 'H' } else { 'h' },
        KeyCode::I => if shift { 'I' } else { 'i' },
        KeyCode::J => if shift { 'J' } else { 'j' },
        KeyCode::K => if shift { 'K' } else { 'k' },
        KeyCode::L => if shift { 'L' } else { 'l' },
        KeyCode::M => if shift { 'M' } else { 'm' },
        KeyCode::N => if shift { 'N' } else { 'n' },
        KeyCode::O => if shift { 'O' } else { 'o' },
        KeyCode::P => if shift { 'P' } else { 'p' },
        KeyCode::Q => if shift { 'Q' } else { 'q' },
        KeyCode::R => if shift { 'R' } else { 'r' },
        KeyCode::S => if shift { 'S' } else { 's' },
        KeyCode::T => if shift { 'T' } else { 't' },
        KeyCode::U => if shift { 'U' } else { 'u' },
        KeyCode::V => if shift { 'V' } else { 'v' },
        KeyCode::W => if shift { 'W' } else { 'w' },
        KeyCode::X => if shift { 'X' } else { 'x' },
        KeyCode::Y => if shift { 'Y' } else { 'y' },
        KeyCode::Z => if shift { 'Z' } else { 'z' },
        KeyCode::Row1 => if shift { '!' } else { '1' },
        KeyCode::Row2 => if shift { '@' } else { '2' },
        KeyCode::Row3 => if shift { '#' } else { '3' },
        KeyCode::Row4 => if shift { '$' } else { '4' },
        KeyCode::Row5 => if shift { '%' } else { '5' },
        KeyCode::Row6 => if shift { '^' } else { '6' },
        KeyCode::Row7 => if shift { '&' } else { '7' },
        KeyCode::Row8 => if shift { '*' } else { '8' },
        KeyCode::Row9 => if shift { '(' } else { '9' },
        KeyCode::Row0 => if shift { ')' } else { '0' },
        KeyCode::Grave => if shift { '~' } else { '`' },
        KeyCode::Minus => if shift { '_' } else { '-' },
        KeyCode::Equals => if shift { '+' } else { '=' },
        KeyCode::LeftBracket => if shift { '{' } else { '[' },
        KeyCode::RightBracket => if shift { '}' } else { ']' },
        KeyCode::Backslash => if shift { '|' } else { '\\' },
        KeyCode::Semicolon => if shift { ':' } else { ';' },
        KeyCode::Apostrophe => if shift { '"' } else { '\'' },
        KeyCode::Comma => if shift { '<' } else { ',' },
        KeyCode::Period => if shift { '>' } else { '.' },
        KeyCode::Slash => if shift { '?' } else { '/' },
        KeyCode::Space => ' ',
        KeyCode::NumDivide => '/',
        KeyCode::NumMultiply => '*',
        KeyCode::NumMinus => '-',
        KeyCode::NumPlus => '+',
        KeyCode::NumPeriod => '.',
        KeyCode::Num1 => '1',
        KeyCode::Num2 => '2',
        KeyCode::Num3 => '3',
        KeyCode::Num4 => '4',
        KeyCode::Num5 => '5',
        KeyCode::Num6 => '6',
        KeyCode::Num7 => '7',
        KeyCode::Num8 => '8',
        KeyCode::Num9 => '9',
        KeyCode::Num0 => '0',
        _ => unreachable!(),
    }
}

fn blt_keycode_to_key(kc: KeyCode) -> Key {
    match kc {
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::Pause => Key::PauseBreak,
        KeyCode::Insert => Key::Ins,
        KeyCode::Home => Key::Home,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::Delete => Key::Del,
        KeyCode::End => Key::End,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Right => Key::Right,
        KeyCode::Left => Key::Left,
        KeyCode::Down => Key::Down,
        KeyCode::Up => Key::Up,
        _ => unreachable!(),
    }
}
