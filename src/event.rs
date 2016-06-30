//! User-input events and their effects.

use std::fmt;
use std::rc::Rc;

use Cursive;

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

/// Represents a key, or a combination of keys.
#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub enum Key {
    /// Both Enter and numpad Enter
    Enter,
    /// Tabulation key
    Tab,
    ShiftTab,
    Backspace,

    /// Indicates the window was resized
    ///
    /// (Not really a key)
    Resize,

    /// Escape key.
    Esc,
    /// The 5 in the center of the keypad, when numlock is disabled.
    NumpadCenter,

    Left,
    /// Left arrow while shift is pressed.
    ShiftLeft,
    AltLeft,
    AltShiftLeft,
    CtrlLeft,
    CtrlShiftLeft,
    CtrlAltLeft,

    Right,
    /// Right arrow while shift is pressed.
    ShiftRight,
    AltRight,
    AltShiftRight,
    CtrlRight,
    CtrlShiftRight,
    CtrlAltRight,

    Up,
    ShiftUp,
    AltUp,
    AltShiftUp,
    CtrlUp,
    CtrlShiftUp,
    CtrlAltUp,

    Down,
    ShiftDown,
    AltDown,
    AltShiftDown,
    CtrlDown,
    CtrlShiftDown,
    CtrlAltDown,

    PageUp,
    ShiftPageUp,
    AltPageUp,
    AltShiftPageUp,
    CtrlPageUp,
    CtrlShiftPageUp,
    CtrlAltPageUp,

    PageDown,
    ShiftPageDown,
    AltPageDown,
    AltShiftPageDown,
    CtrlPageDown,
    CtrlShiftPageDown,
    CtrlAltPageDown,

    Home,
    ShiftHome,
    AltHome,
    AltShiftHome,
    CtrlHome,
    CtrlShiftHome,
    CtrlAltHome,

    End,
    ShiftEnd,
    AltEnd,
    AltShiftEnd,
    CtrlEnd,
    CtrlShiftEnd,
    CtrlAltEnd,

    /// Delete key
    Del,
    ShiftDel,
    AltDel,
    AltShiftDel,
    CtrlDel,
    CtrlShiftDel,

    /// Insert key.
    Ins,
    /// Insert key while ctrl is pressed.
    CtrlIns,
    AltIns,
    CtrlAltIns,

    F(u8),
    ShiftF(u8),
    AltF(u8),
    CtrlF(u8),
    CtrlShiftF(u8),
    CtrlChar(char),
    Unknown(i32),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::Unknown(ch) => write!(f, "Unknown: {}", ch),
            Key::CtrlChar(ch) => write!(f, "Ctrl-{}", ch),
            Key::F(n) => write!(f, "F{}", n),
            Key::ShiftF(n) => write!(f, "Shift-F{}", n),
            Key::AltF(n) => write!(f, "Alt-F{}", n),
            Key::CtrlF(n) => write!(f, "Ctrl-F{}", n),
            Key::CtrlShiftF(n) => write!(f, "Ctrl-Shift-F{}", n),
            key => {
                write!(f,
                       "{}",
                       match key {
                           Key::Resize => "Screen resize",
                           Key::NumpadCenter => "Numpad center",
                           Key::Backspace => "Backspace",
                           Key::Enter => "Enter",
                           Key::Tab => "Tab",
                           Key::ShiftTab => "Shift-Tab",

                           Key::PageUp => "PageUp",
                           Key::ShiftPageUp => "Shift-PageUp",
                           Key::AltPageUp => "Alt-PageUp",
                           Key::AltShiftPageUp => "Alt-Shift-PageUp",
                           Key::CtrlPageUp => "Ctrl-PageUp",
                           Key::CtrlShiftPageUp => "Ctrl-Shift-PageUp",
                           Key::CtrlAltPageUp => "Ctrl-Alt-PageUp",

                           Key::PageDown => "PageDown",
                           Key::ShiftPageDown => "Shift-PageDown",
                           Key::AltPageDown => "Alt-PageDown",
                           Key::AltShiftPageDown => "Alt-Shift-PageDown",
                           Key::CtrlPageDown => "Ctrl-PageDown",
                           Key::CtrlShiftPageDown => "Ctrl-Shift-PageDown",
                           Key::CtrlAltPageDown => "Ctrl-Alt-PageDown",

                           Key::Left => "Left",
                           Key::ShiftLeft => "Shift-Left",
                           Key::AltLeft => "Alt-Left",
                           Key::AltShiftLeft => "Alt-Shift-Left",
                           Key::CtrlLeft => "Ctrl-Left",
                           Key::CtrlShiftLeft => "Ctrl-Shift-Left",
                           Key::CtrlAltLeft => "Ctrl-Alt-Left",

                           Key::Right => "Right",
                           Key::ShiftRight => "Shift-Right",
                           Key::AltRight => "Alt-Right",
                           Key::AltShiftRight => "Alt-Shift-Right",
                           Key::CtrlRight => "Ctrl-Right",
                           Key::CtrlShiftRight => "Ctrl-Shift-Right",
                           Key::CtrlAltRight => "Ctrl-Alt-Right",

                           Key::Down => "Down",
                           Key::ShiftDown => "Shift-Down",
                           Key::AltDown => "Alt-Down",
                           Key::AltShiftDown => "Alt-Shift-Down",
                           Key::CtrlDown => "Ctrl-Down",
                           Key::CtrlShiftDown => "Ctrl-Shift-Down",
                           Key::CtrlAltDown => "Ctrl-Alt-Down",

                           Key::Up => "Up",
                           Key::ShiftUp => "Shift-Up",
                           Key::AltUp => "Alt-Up",
                           Key::AltShiftUp => "Alt-Shift-Up",
                           Key::CtrlUp => "Ctrl-Up",
                           Key::CtrlShiftUp => "Ctrl-Shift-Up",
                           Key::CtrlAltUp => "Ctrl-Alt-Up",

                           Key::Del => "Del",
                           Key::ShiftDel => "Shift-Del",
                           Key::AltDel => "Alt-Del",
                           Key::AltShiftDel => "Alt-Shift-Del",
                           Key::CtrlDel => "Ctrl-Del",
                           Key::CtrlShiftDel => "Ctrl-Shift-Del",

                           Key::Home => "Home",
                           Key::ShiftHome => "Shift-Home",
                           Key::AltHome => "Alt-Home",
                           Key::AltShiftHome => "Alt-Shift-Home",
                           Key::CtrlHome => "Ctrl-Home",
                           Key::CtrlShiftHome => "Ctrl-Shift-Home",
                           Key::CtrlAltHome => "Ctrl-Alt-Home",

                           Key::End => "End",
                           Key::ShiftEnd => "Shift-End",
                           Key::AltEnd => "Alt-End",
                           Key::AltShiftEnd => "Alt-Shift-End",
                           Key::CtrlEnd => "Ctrl-End",
                           Key::CtrlShiftEnd => "Ctrl-Shift-End",
                           Key::CtrlAltEnd => "Ctrl-Alt-End",

                           Key::Ins => "Ins",
                           Key::AltIns => "Alt-Ins",
                           Key::CtrlIns => "Ctrl-Ins",
                           Key::CtrlAltIns => "Ctrl-Alt-Ins",

                           Key::Esc => "Esc",
                           _ => "Missing key label",
                       })
            }
        }
    }
}

/// Represents an event as seen by the application.
///
#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub enum Event {
    /// A text character was entered.
    Char(char),
    /// A key was pressed.
    Key(Key),
}

/// Generic trait to convert a value to an event.
pub trait ToEvent {
    fn to_event(self) -> Event;
}

impl ToEvent for char {
    fn to_event(self) -> Event {
        Event::Char(self)
    }
}

impl ToEvent for Key {
    fn to_event(self) -> Event {
        Event::Key(self)
    }
}

impl ToEvent for Event {
    fn to_event(self) -> Event {
        self
    }
}
