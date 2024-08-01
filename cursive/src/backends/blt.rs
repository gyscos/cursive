//! Backend using BearLibTerminal
//!
//! Requires the `blt-backend` feature.
#![cfg(feature = "blt-backend")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "blt-backend")))]

pub use bear_lib_terminal;

use std::cell::Cell;

use bear_lib_terminal::geometry::Size;
use bear_lib_terminal::terminal::{self, state, Event as BltEvent, KeyCode};
use bear_lib_terminal::Color as BltColor;

use unicode_width::UnicodeWidthStr;

use crate::backend;
use crate::event::{Event, Key, MouseButton, MouseEvent};
use crate::theme::{BaseColor, Color, ColorPair, Effect};
use crate::Vec2;

// Use AHash instead of the slower SipHash
type HashSet<K> = std::collections::HashSet<K, ahash::RandomState>;

enum ColorRole {
    Foreground,
    Background,
}

/// Backend using BearLibTerminal
pub struct Backend {
    buttons_pressed: HashSet<MouseButton>,
    cursor: Cell<Vec2>,
    mouse_position: Vec2,
}

impl Backend {
    /// Creates a new BearLibTerminal-based backend.
    pub fn init() -> Box<dyn backend::Backend> {
        // TODO: Add some error handling?
        terminal::open("Cursive", 80, 24);
        terminal::set(terminal::config::Window::empty().resizeable(true));
        terminal::set(vec![
            terminal::config::InputFilter::Group {
                group: terminal::config::InputFilterGroup::Keyboard,
                both: false,
            },
            terminal::config::InputFilter::Group {
                group: terminal::config::InputFilterGroup::Mouse,
                both: true,
            },
        ]);

        let c = Backend {
            buttons_pressed: HashSet::default(),
            mouse_position: Vec2::zero(),
            cursor: Cell::new(Vec2::zero()),
        };

        Box::new(c)
    }

    fn parse_next(&mut self) -> Option<Event> {
        // TODO: we could add backend-specific controls here.
        // Ex: ctrl+mouse wheel cause window cellsize to change
        terminal::read_event().map(|ev| {
            match ev {
                BltEvent::Close => Event::Exit,
                BltEvent::Resize { .. } => Event::WindowResize,
                // TODO: mouse support
                BltEvent::MouseMove { x, y } => {
                    self.mouse_position = Vec2::new(x as usize, y as usize);
                    // TODO: find out if a button is pressed?
                    match self.buttons_pressed.iter().next() {
                        None => Event::Refresh,
                        Some(btn) => Event::Mouse {
                            event: MouseEvent::Hold(*btn),
                            position: self.mouse_position,
                            offset: Vec2::zero(),
                        },
                    }
                }
                BltEvent::MouseScroll { delta } => Event::Mouse {
                    event: if delta < 0 {
                        MouseEvent::WheelUp
                    } else {
                        MouseEvent::WheelDown
                    },
                    position: self.mouse_position,
                    offset: Vec2::zero(),
                },
                BltEvent::KeyPressed { key, ctrl, shift } => {
                    self.blt_keycode_to_ev(key, shift, ctrl)
                }
                // TODO: there's no Key::Shift/Ctrl for w/e reason
                BltEvent::ShiftPressed => Event::Refresh,
                BltEvent::ControlPressed => Event::Refresh,
                // TODO: what should we do here?
                BltEvent::KeyReleased { key, .. } => {
                    // It's probably a mouse key.
                    blt_keycode_to_mouse_button(key)
                        .map(|btn| {
                            self.buttons_pressed.remove(&btn);
                            Event::Mouse {
                                event: MouseEvent::Release(btn),
                                position: self.mouse_position,
                                offset: Vec2::zero(),
                            }
                        })
                        .unwrap_or_else(|| Event::Unknown(vec![]))
                }
                BltEvent::ShiftReleased | BltEvent::ControlReleased => Event::Refresh,
                _ => Event::Unknown(vec![]),
            }
        })
    }

    fn blt_keycode_to_ev(&mut self, kc: KeyCode, shift: bool, ctrl: bool) -> Event {
        match kc {
            KeyCode::F1
            | KeyCode::F2
            | KeyCode::F3
            | KeyCode::F4
            | KeyCode::F5
            | KeyCode::F6
            | KeyCode::F7
            | KeyCode::F8
            | KeyCode::F9
            | KeyCode::F10
            | KeyCode::F11
            | KeyCode::F12
            | KeyCode::NumEnter
            | KeyCode::Enter
            | KeyCode::Escape
            | KeyCode::Backspace
            | KeyCode::Tab
            | KeyCode::Pause
            | KeyCode::Insert
            | KeyCode::Home
            | KeyCode::PageUp
            | KeyCode::Delete
            | KeyCode::End
            | KeyCode::PageDown
            | KeyCode::Right
            | KeyCode::Left
            | KeyCode::Down
            | KeyCode::Up => match (shift, ctrl) {
                (true, true) => Event::CtrlShift(blt_keycode_to_key(kc)),
                (true, false) => Event::Shift(blt_keycode_to_key(kc)),
                (false, true) => Event::Ctrl(blt_keycode_to_key(kc)),
                (false, false) => Event::Key(blt_keycode_to_key(kc)),
            },
            // TODO: mouse support
            KeyCode::MouseLeft
            | KeyCode::MouseRight
            | KeyCode::MouseMiddle
            | KeyCode::MouseFourth
            | KeyCode::MouseFifth => blt_keycode_to_mouse_button(kc)
                .map(|btn| {
                    self.buttons_pressed.insert(btn);
                    Event::Mouse {
                        event: MouseEvent::Press(btn),
                        position: self.mouse_position,
                        offset: Vec2::zero(),
                    }
                })
                .unwrap_or_else(|| Event::Unknown(vec![])),
            KeyCode::A
            | KeyCode::B
            | KeyCode::C
            | KeyCode::D
            | KeyCode::E
            | KeyCode::F
            | KeyCode::G
            | KeyCode::H
            | KeyCode::I
            | KeyCode::J
            | KeyCode::K
            | KeyCode::L
            | KeyCode::M
            | KeyCode::N
            | KeyCode::O
            | KeyCode::P
            | KeyCode::Q
            | KeyCode::R
            | KeyCode::S
            | KeyCode::T
            | KeyCode::U
            | KeyCode::V
            | KeyCode::W
            | KeyCode::X
            | KeyCode::Y
            | KeyCode::Z
            | KeyCode::Row1
            | KeyCode::Row2
            | KeyCode::Row3
            | KeyCode::Row4
            | KeyCode::Row5
            | KeyCode::Row6
            | KeyCode::Row7
            | KeyCode::Row8
            | KeyCode::Row9
            | KeyCode::Row0
            | KeyCode::Grave
            | KeyCode::Minus
            | KeyCode::Equals
            | KeyCode::LeftBracket
            | KeyCode::RightBracket
            | KeyCode::Backslash
            | KeyCode::Semicolon
            | KeyCode::Apostrophe
            | KeyCode::Comma
            | KeyCode::Period
            | KeyCode::Slash
            | KeyCode::Space
            | KeyCode::NumDivide
            | KeyCode::NumMultiply
            | KeyCode::NumMinus
            | KeyCode::NumPlus
            | KeyCode::NumPeriod
            | KeyCode::Num1
            | KeyCode::Num2
            | KeyCode::Num3
            | KeyCode::Num4
            | KeyCode::Num5
            | KeyCode::Num6
            | KeyCode::Num7
            | KeyCode::Num8
            | KeyCode::Num9
            | KeyCode::Num0 => {
                if ctrl {
                    Event::CtrlChar(blt_keycode_to_char(kc, shift))
                } else {
                    Event::Char(blt_keycode_to_char(kc, shift))
                }
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        terminal::close();
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "bear-lib-terminal"
    }

    fn set_title(&mut self, title: String) {
        terminal::set(terminal::config::Window::empty().title(title));
    }

    fn set_color(&self, color: ColorPair) -> ColorPair {
        let current = ColorPair {
            front: blt_colour_to_colour(state::foreground()),
            back: blt_colour_to_colour(state::background()),
        };

        let fg = colour_to_blt_colour(color.front, ColorRole::Foreground);
        let bg = colour_to_blt_colour(color.back, ColorRole::Background);

        terminal::set_colors(fg, bg);

        current
    }

    fn set_effect(&self, effect: Effect) {
        match effect {
            // TODO: does BLT support bold/italic/strikethrough/underline?
            Effect::Dim
            | Effect::Bold
            | Effect::Italic
            | Effect::Underline
            | Effect::Strikethrough
            | Effect::Blink
            | Effect::Simple => {}
            // TODO: implement this correctly.
            // Add a `reverse` flag in the Backend, and check it when calling `set_colors`.
            Effect::Reverse => terminal::set_colors(state::background(), state::foreground()),
        }
    }

    fn unset_effect(&self, effect: Effect) {
        match effect {
            // TODO: does BLT support bold/italic/strikethrough/underline?
            Effect::Dim
            | Effect::Bold
            | Effect::Italic
            | Effect::Underline
            | Effect::Strikethrough
            | Effect::Blink
            | Effect::Simple => {}
            // The process of reversing is the same as unreversing
            Effect::Reverse => terminal::set_colors(state::background(), state::foreground()),
        }
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        let Size { width, height } = terminal::state::size();
        (width, height).into()
    }

    fn clear(&self, color: Color) {
        terminal::set_background(colour_to_blt_colour(color, ColorRole::Background));
        terminal::clear(None);
    }

    fn refresh(&mut self) {
        terminal::refresh();
    }

    fn move_to(&self, pos: Vec2) {
        self.cursor.set(pos);
    }

    fn print(&self, text: &str) {
        let pos = self.cursor.get();
        terminal::print_xy(pos.x as i32, pos.y as i32, text);
        self.cursor.set(pos + (text.width(), 0));
    }

    fn poll_event(&mut self) -> Option<Event> {
        self.parse_next()
    }
}

fn blt_colour_to_colour(c: BltColor) -> Color {
    Color::Rgb(c.red, c.green, c.blue)
}

fn colour_to_blt_colour(clr: Color, role: ColorRole) -> BltColor {
    let (r, g, b) = match clr {
        Color::TerminalDefault => {
            let clr = match role {
                ColorRole::Foreground => state::foreground(),
                ColorRole::Background => state::background(),
            };

            return clr;
        }

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
        Color::RgbLowRes(r, g, b) => (
            (f32::from(r) / 5.0 * 255.0) as u8,
            (f32::from(g) / 5.0 * 255.0) as u8,
            (f32::from(b) / 5.0 * 255.0) as u8,
        ),
    };
    BltColor::from_rgb(r, g, b)
}

fn blt_keycode_to_char(kc: KeyCode, shift: bool) -> char {
    match bear_lib_terminal::terminal::state::char() {
        '\u{0}' => blt_keycode_to_char_impl(kc, shift),
        c => c,
    }
}

#[allow(clippy::cognitive_complexity)]
fn blt_keycode_to_char_impl(kc: KeyCode, shift: bool) -> char {
    match kc {
        KeyCode::A if shift => 'A',
        KeyCode::A => 'a',
        KeyCode::B if shift => 'B',
        KeyCode::B => 'b',
        KeyCode::C if shift => 'C',
        KeyCode::C => 'c',
        KeyCode::D if shift => 'D',
        KeyCode::D => 'd',
        KeyCode::E if shift => 'E',
        KeyCode::E => 'e',
        KeyCode::F if shift => 'F',
        KeyCode::F => 'f',
        KeyCode::G if shift => 'G',
        KeyCode::G => 'g',
        KeyCode::H if shift => 'H',
        KeyCode::H => 'h',
        KeyCode::I if shift => 'I',
        KeyCode::I => 'i',
        KeyCode::J if shift => 'J',
        KeyCode::J => 'j',
        KeyCode::K if shift => 'K',
        KeyCode::K => 'k',
        KeyCode::L if shift => 'L',
        KeyCode::L => 'l',
        KeyCode::M if shift => 'M',
        KeyCode::M => 'm',
        KeyCode::N if shift => 'N',
        KeyCode::N => 'n',
        KeyCode::O if shift => 'O',
        KeyCode::O => 'o',
        KeyCode::P if shift => 'P',
        KeyCode::P => 'p',
        KeyCode::Q if shift => 'Q',
        KeyCode::Q => 'q',
        KeyCode::R if shift => 'R',
        KeyCode::R => 'r',
        KeyCode::S if shift => 'S',
        KeyCode::S => 's',
        KeyCode::T if shift => 'T',
        KeyCode::T => 't',
        KeyCode::U if shift => 'U',
        KeyCode::U => 'u',
        KeyCode::V if shift => 'V',
        KeyCode::V => 'v',
        KeyCode::W if shift => 'W',
        KeyCode::W => 'w',
        KeyCode::X if shift => 'X',
        KeyCode::X => 'x',
        KeyCode::Y if shift => 'Y',
        KeyCode::Y => 'y',
        KeyCode::Z if shift => 'Z',
        KeyCode::Z => 'z',
        KeyCode::Row1 if shift => '!',
        KeyCode::Row1 => '1',
        KeyCode::Row2 if shift => '@',
        KeyCode::Row2 => '2',
        KeyCode::Row3 if shift => '#',
        KeyCode::Row3 => '3',
        KeyCode::Row4 if shift => '$',
        KeyCode::Row4 => '4',
        KeyCode::Row5 if shift => '%',
        KeyCode::Row5 => '5',
        KeyCode::Row6 if shift => '^',
        KeyCode::Row6 => '6',
        KeyCode::Row7 if shift => '&',
        KeyCode::Row7 => '7',
        KeyCode::Row8 if shift => '*',
        KeyCode::Row8 => '8',
        KeyCode::Row9 if shift => '(',
        KeyCode::Row9 => '9',
        KeyCode::Row0 if shift => ')',
        KeyCode::Row0 => '0',
        KeyCode::Grave if shift => '~',
        KeyCode::Grave => '`',
        KeyCode::Minus if shift => '_',
        KeyCode::Minus => '-',
        KeyCode::Equals if shift => '+',
        KeyCode::Equals => '=',
        KeyCode::LeftBracket if shift => '{',
        KeyCode::LeftBracket => '[',
        KeyCode::RightBracket if shift => '}',
        KeyCode::RightBracket => ']',
        KeyCode::Backslash if shift => '|',
        KeyCode::Backslash => '\\',
        KeyCode::Semicolon if shift => ':',
        KeyCode::Semicolon => ';',
        KeyCode::Apostrophe if shift => '"',
        KeyCode::Apostrophe => '\'',
        KeyCode::Comma if shift => '<',
        KeyCode::Comma => ',',
        KeyCode::Period if shift => '>',
        KeyCode::Period => '.',
        KeyCode::Slash if shift => '?',
        KeyCode::Slash => '/',
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
        _ => unreachable!("Found unknown input: {:?}", kc),
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
        KeyCode::NumEnter | KeyCode::Enter => Key::Enter,
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

fn blt_keycode_to_mouse_button(kc: KeyCode) -> Option<MouseButton> {
    Some(match kc {
        KeyCode::MouseLeft => MouseButton::Left,
        KeyCode::MouseRight => MouseButton::Right,
        KeyCode::MouseMiddle => MouseButton::Middle,
        _ => return None,
    })
}
