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
    Enter,
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
    Del,
    Unknown(i32),
}

impl Key {
    pub fn from_ncurses(ch: i32) -> Self {
        match ch {
            9 => Key::Tab,
            10 => Key::Enter,
            127 => Key::Backspace,
            330 => Key::Del,
            ncurses::KEY_LEFT => Key::ArrowLeft,
            ncurses::KEY_RIGHT => Key::ArrowRight,
            ncurses::KEY_UP => Key::ArrowUp,
            ncurses::KEY_DOWN => Key::ArrowDown,
            ncurses::KEY_PPAGE => Key::PageUp,
            ncurses::KEY_NPAGE => Key::PageDown,
            ncurses::KEY_HOME => Key::Home,
            ncurses::KEY_END => Key::End,
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
