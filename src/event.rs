//! User-input events and their effects.

use std::fmt;
use std::rc::Rc;

use ncurses;

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
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Backspace,
    Home,
    ShiftHome,
    CtrlHome,
    CtrlShiftHome,
    End,
    ShiftEnd,
    CtrlEnd,
    CtrlShiftEnd,
    /// Delete key
    Del,
    ShiftDel,
    CtrlDel,
    CtrlShiftDel,
    /// Insert key.
    Ins,
    /// Insert key while ctrl is pressed.
    CtrlIns,
    /// Escape key.
    Esc,
    /// The 5 in the center of the keypad, when numlock is disabled.
    NumpadCenter,
    CtrlPageUp,
    CtrlPageDown,
    ShiftUp,
    ShiftDown,
    /// Left arrow while shift is pressed.
    ShiftLeft,
    /// Right arrow while shift is pressed.
    ShiftRight,
    CtrlShiftLeft,
    CtrlShiftRight,
    CtrlShiftUp,
    CtrlShiftDown,
    CtrlLeft,
    CtrlRight,
    CtrlUp,
    CtrlDown,
    F(u8),
    CtrlF(u8),
    ShiftF(u8),
    CtrlShiftF(u8),
    CtrlChar(char),
    Unknown(i32),
}

impl Key {
    /// Returns the Key enum corresponding to the given ncurses event.
    pub fn from_ncurses(ch: i32) -> Self {
        match ch {
            // Tab is '\t'
            9 => Key::Tab,
            // Treat '\n' and the numpad Enter the same
            10 | ncurses::KEY_ENTER => Key::Enter,
            // This is the escape key when pressed by itself.
            // When used for control sequences, it should have been caught earlier.
            27 => Key::Esc,
            // `Backspace` sends 127, but Ctrl-H sends `Backspace`
            127 | ncurses::KEY_BACKSPACE => Key::Backspace,
            // Those keys don't seem to be documented...
            515 => Key::CtrlDel,
            516 => Key::CtrlShiftDel,
            521 => Key::CtrlDown,
            522 => Key::CtrlShiftDown,
            526 => Key::CtrlEnd,
            527 => Key::CtrlShiftEnd,
            531 => Key::CtrlHome,
            532 => Key::CtrlShiftHome,
            536 => Key::CtrlIns,
            541 => Key::CtrlLeft,
            542 => Key::CtrlShiftLeft,
            546 => Key::CtrlPageDown,
            551 => Key::CtrlPageUp,
            556 => Key::CtrlRight,
            557 => Key::CtrlShiftRight,
            562 => Key::CtrlUp,
            563 => Key::CtrlShiftUp,
            ncurses::KEY_B2 => Key::NumpadCenter,
            ncurses::KEY_DC => Key::Del,
            ncurses::KEY_IC => Key::Ins,
            ncurses::KEY_BTAB => Key::ShiftTab,
            ncurses::KEY_SLEFT => Key::ShiftLeft,
            ncurses::KEY_SRIGHT => Key::ShiftRight,
            ncurses::KEY_LEFT => Key::Left,
            ncurses::KEY_RIGHT => Key::Right,
            ncurses::KEY_UP => Key::Up,
            ncurses::KEY_DOWN => Key::Down,
            ncurses::KEY_SR => Key::ShiftUp,
            ncurses::KEY_SF => Key::ShiftDown,
            ncurses::KEY_PPAGE => Key::PageUp,
            ncurses::KEY_NPAGE => Key::PageDown,
            ncurses::KEY_HOME => Key::Home,
            ncurses::KEY_END => Key::End,
            ncurses::KEY_SHOME => Key::ShiftHome,
            ncurses::KEY_SEND => Key::ShiftEnd,
            ncurses::KEY_SDC => Key::ShiftDel,
            // All Fn keys use the same enum with associated number
            f @ ncurses::KEY_F1 ... ncurses::KEY_F15 => Key::F((f - ncurses::KEY_F0) as u8),
            f @ 281 ... 291 => Key::ShiftF((f - 281 + 5) as u8),
            f @ 293 ... 303 => Key::CtrlF((f - 293 + 5) as u8),
            f @ 305 ... 315 => Key::CtrlShiftF((f - 305 + 5) as u8),
            // Shift and Ctrl F{1-4} need escape sequences...
            //
            // TODO: shift and ctrl Fn keys
            // Avoids 8-10 (H,I,J), they are used by other commands.
            c @ 1 ... 7 | c @ 11 ... 25 => Key::CtrlChar(('a' as u8 + (c - 1) as u8) as char),
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
            Key::ShiftF(n) => write!(f, "Shift-F{}", n),
            Key::CtrlF(n) => write!(f, "Ctrl-F{}", n),
            Key::CtrlShiftF(n) => write!(f, "Ctrl-Shift-F{}", n),
            key => write!(f,
                          "{}",
                          match key {
                              Key::NumpadCenter => "Numpad center",
                              Key::Left => "Left",
                              Key::Right => "Right",
                              Key::Down => "Down",
                              Key::Up => "Up",
                              Key::CtrlShiftLeft => "Ctrl-Shift-Left",
                              Key::CtrlShiftRight => "Ctrl-Shift-Right",
                              Key::CtrlShiftUp => "Ctrl-Shift-Up",
                              Key::CtrlShiftDown => "Ctrl-Shift-Down",
                              Key::ShiftUp => "Shift-Up",
                              Key::ShiftDown => "Shift-Down",
                              Key::ShiftLeft => "Shift-Left",
                              Key::ShiftRight => "Shift-Right",
                              Key::ShiftDel => "Shift-Del",
                              Key::CtrlDel => "Ctrl-Del",
                              Key::CtrlShiftDel => "Ctrl-Shift-Del",
                              Key::CtrlLeft => "Ctrl-Left",
                              Key::CtrlRight => "Ctrl-Right",
                              Key::CtrlUp => "Ctrl-Up",
                              Key::CtrlDown => "Ctrl-Down",
                              Key::CtrlPageUp => "Ctrl-PageUp",
                              Key::CtrlPageDown => "Ctrl-PageDown",
                              Key::PageUp => "PageUp",
                              Key::PageDown => "PageDown",
                              Key::Home => "Home",
                              Key::ShiftHome => "Shift-Home",
                              Key::CtrlHome => "Ctrl-Home",
                              Key::CtrlShiftHome => "Ctrl-Shift-Home",
                              Key::End => "End",
                              Key::ShiftEnd => "Shift-End",
                              Key::CtrlEnd => "Ctrl-End",
                              Key::CtrlShiftEnd => "Ctrl-Shift-End",
                              Key::Backspace => "Backspace",
                              Key::Del => "Del",
                              Key::Enter => "Enter",
                              Key::ShiftTab => "Shift-Tab",
                              Key::Tab => "Tab",
                              Key::Ins => "Ins",
                              Key::CtrlIns => "Ctrl-Ins",
                              Key::Esc => "Esc",
                              _ => "Missing key label",
                          }),
        }
    }
}

/// Represents an event as seen by the application.
///
#[derive(PartialEq,Eq,Clone,Copy,Hash)]
pub enum Event {
    /// A text character was entered.
    CharEvent(char),
    /// A key was pressed.
    KeyEvent(Key),
}

/// Generic trait to convert a value to an event.
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
