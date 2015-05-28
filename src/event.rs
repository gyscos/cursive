//! User-input events and their effects.

use std::fmt;
use std::rc::Rc;

use ncurses;

use ::Cursive;

/// Callback is a function that can be triggered by an event.
/// It has a mutable access to the cursive root.
pub type Callback = Box<Fn(&mut Cursive)>;

/// Answer to an event notification.
/// The event can be consumed or ignored.
pub enum EventResult {
    /// The event was ignored. The parent can keep handling it.
    Ignored,
    /// The event was consumed. An optionnal callback to run is attached.
    Consumed(Option<Rc<Callback>>),
}

#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub enum Key {
    /// Both Enter and numpad Enter
    Enter,
    /// Tabulation key
    Tab,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Backspace,
    Home,
    End,
    /// Delete key
    Del,
    /// Insert key
    Ins,
    /// Escape key. Often buffered by the terminal,
    /// may appear with a delay or along with the next key.
    Esc,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    // Left arrow while shift is pressed
    ShiftLeft,
    ShiftRight,
    CtrlLeft,
    CtrlRight,
    CtrlUp,
    CtrlDown,
    CtrlChar(char),
    Unknown(i32),
}

impl Key {
    pub fn from_ncurses(ch: i32) -> Self {
        match ch {
            9 => Key::Tab,
            // Treat Return and the numpad Enter the same
            10 | ncurses::KEY_ENTER => Key::Enter,
            27 => Key::Esc,
            127 | ncurses::KEY_BACKSPACE => Key::Backspace,
            330 => Key::Del,
            331 => Key::Ins,
            521 => Key::CtrlDown,
            541 => Key::CtrlLeft,
            556 => Key::CtrlRight,
            562 => Key::CtrlUp,
            ncurses::KEY_SLEFT => Key::ShiftLeft,
            ncurses::KEY_SRIGHT => Key::ShiftRight,
            ncurses::KEY_LEFT => Key::Left,
            ncurses::KEY_RIGHT => Key::Right,
            ncurses::KEY_UP => Key::Up,
            ncurses::KEY_DOWN => Key::Down,
            ncurses::KEY_PPAGE => Key::PageUp,
            ncurses::KEY_NPAGE => Key::PageDown,
            ncurses::KEY_HOME => Key::Home,
            ncurses::KEY_END => Key::End,
            ncurses::KEY_F1 => Key::F1,
            ncurses::KEY_F2 => Key::F2,
            ncurses::KEY_F3 => Key::F3,
            ncurses::KEY_F4 => Key::F4,
            ncurses::KEY_F5 => Key::F5,
            ncurses::KEY_F6 => Key::F6,
            ncurses::KEY_F7 => Key::F7,
            ncurses::KEY_F8 => Key::F8,
            ncurses::KEY_F9 => Key::F9,
            ncurses::KEY_F10 => Key::F10,
            ncurses::KEY_F11 => Key::F11,
            ncurses::KEY_F12 => Key::F12,
            ncurses::KEY_F13 => Key::F13,
            ncurses::KEY_F14 => Key::F14,
            ncurses::KEY_F15 => Key::F15,
            c @ 1 ... 7 | c @ 11 ... 25 => Key::CtrlChar(('a' as u8 + (c-1) as u8) as char),
            _ => Key::Unknown(ch),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::Unknown(ch) => write!(f, "Unknown: {}", ch),
            Key::CtrlChar(ch) => write!(f, "Ctrl-{}", ch),
            key => write!(f, "{}", match key {
                Key::Left => "Left",
                Key::Right => "Right",
                Key::Down => "Down",
                Key::Up => "Up",
                Key::ShiftLeft => "Shift-Left",
                Key::ShiftRight => "Shift-Right",
                Key::CtrlLeft => "Ctrl-Left",
                Key::CtrlRight => "Ctrl-Right",
                Key::CtrlUp => "Ctrl-Up",
                Key::CtrlDown => "Ctrl-Down",
                Key::PageUp => "PageUp",
                Key::PageDown => "PageDown",
                Key::Home => "Home",
                Key::End => "End",
                Key::Backspace => "Backspace",
                Key::Del => "Del",
                Key::Enter => "Enter",
                Key::Tab => "Tab",
                Key::Ins => "Ins",
                Key::Esc => "Esc",
                Key::F1 => "F1",
                Key::F2 => "F2",
                Key::F3 => "F3",
                Key::F4 => "F4",
                Key::F5 => "F5",
                Key::F6 => "F6",
                Key::F7 => "F7",
                Key::F8 => "F8",
                Key::F9 => "F9",
                Key::F10 => "F10",
                Key::F11 => "F11",
                Key::F12 => "F12",
                Key::F13 => "F13",
                Key::F14 => "F14",
                Key::F15 => "F15",
                _ => "",
            }),
        }
    }
}

#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub enum Event {
    CharEvent(char),
    KeyEvent(Key),
}

pub trait ToEvent {
    fn to_event(self) -> Event;
}

impl ToEvent for char {
    fn to_event(self) -> Event {
        Event::CharEvent(self)
    }
}

impl ToEvent for Key {
    fn to_event(self) -> Event {
        Event::KeyEvent(self)
    }
}

impl ToEvent for Event {
    fn to_event(self) -> Event {
        self
    }
}
