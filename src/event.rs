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
    ShiftTab,
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
    // Left arrow while shift is pressed
    CtrlPageUp,
    CtrlPageDown,
    ShiftLeft,
    ShiftRight,
    ShiftCtrlLeft,
    ShiftCtrlRight,
    CtrlLeft,
    CtrlRight,
    CtrlUp,
    CtrlDown,
    CtrlDel,
    F(u8),
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
            // Those keys don't seem to be documented...
            515 => Key::CtrlDel,
            521 => Key::CtrlDown,
            541 => Key::CtrlLeft,
            542 => Key::ShiftCtrlLeft,
            546 => Key::CtrlPageDown,
            551 => Key::CtrlPageUp,
            556 => Key::CtrlRight,
            557 => Key::ShiftCtrlRight,
            562 => Key::CtrlUp,
            ncurses::KEY_BTAB => Key::ShiftTab,
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
            f @ ncurses::KEY_F1 ... ncurses::KEY_F15 => Key::F((f - ncurses::KEY_F0) as u8),
            // Avoids 8-10 (H,I,J), they are used by other commands.
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
            Key::F(n) => write!(f, "F{}", n),
            key => write!(f, "{}", match key {
                Key::Left => "Left",
                Key::Right => "Right",
                Key::Down => "Down",
                Key::Up => "Up",
                Key::ShiftCtrlLeft => "Shift-Ctrl-Left",
                Key::ShiftCtrlRight => "Shift-Ctrl-Right",
                Key::ShiftLeft => "Shift-Left",
                Key::ShiftRight => "Shift-Right",
                Key::CtrlDel => "Ctrl-Del",
                Key::CtrlLeft => "Ctrl-Left",
                Key::CtrlRight => "Ctrl-Right",
                Key::CtrlUp => "Ctrl-Up",
                Key::CtrlDown => "Ctrl-Down",
                Key::CtrlPageUp => "Ctrl-PageUp",
                Key::CtrlPageDown => "Ctrl-PageDown",
                Key::PageUp => "PageUp",
                Key::PageDown => "PageDown",
                Key::Home => "Home",
                Key::End => "End",
                Key::Backspace => "Backspace",
                Key::Del => "Del",
                Key::Enter => "Enter",
                Key::ShiftTab => "Shift-Tab",
                Key::Tab => "Tab",
                Key::Ins => "Ins",
                Key::Esc => "Esc",
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
