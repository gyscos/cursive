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
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Backspace,
    Home,
    End,
    /// Delete key
    Del,
    /// Insert key
    Ins,
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
    Unknown(i32),
}

impl Key {
    pub fn from_ncurses(ch: i32) -> Self {
        match ch {
            9 => Key::Tab,
            // Treat Return and the numpad Enter the same
            10 | ncurses::KEY_ENTER => Key::Enter,
            127 => Key::Backspace,
            330 => Key::Del,
            331 => Key::Ins,
            ncurses::KEY_LEFT => Key::ArrowLeft,
            ncurses::KEY_RIGHT => Key::ArrowRight,
            ncurses::KEY_UP => Key::ArrowUp,
            ncurses::KEY_DOWN => Key::ArrowDown,
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
            _ => Key::Unknown(ch),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::Unknown(ch) => write!(f, "Unknown: {}", ch),
            key => write!(f, "{}", match key {
                Key::ArrowLeft => "Left",
                Key::ArrowRight => "Right",
                Key::ArrowDown => "Down",
                Key::ArrowUp => "Up",
                Key::PageUp => "PageUp",
                Key::PageDown => "PageDown",
                Key::Home => "Home",
                Key::End => "End",
                Key::Backspace => "Backspace",
                Key::Del => "Del",
                Key::Enter => "Enter",
                Key::Tab => "Tab",
                Key::Ins => "Ins",
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
