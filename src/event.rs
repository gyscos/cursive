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
    Backspace,

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
            // Values under 256 are chars.

            // Values 512 and above are probably extensions
            // Those keys don't seem to be documented...
            519 => Key::AltDel,
            520 => Key::AltShiftDel,
            521 => Key::CtrlDel,
            522 => Key::CtrlShiftDel,
            // 523: CtrlAltDel?

            // 524?

            525 => Key::AltDown,
            526 => Key::AltShiftDown,
            527 => Key::CtrlDown,
            528 => Key::CtrlShiftDown,
            529 => Key::CtrlAltDown,

            530 => Key::AltEnd,
            531 => Key::AltShiftEnd,
            532 => Key::CtrlEnd,
            533 => Key::CtrlShiftEnd,
            534 => Key::CtrlAltEnd,

            535 => Key::AltHome,
            536 => Key::AltShiftHome,
            537 => Key::CtrlHome,
            538 => Key::CtrlShiftHome,
            539 => Key::CtrlAltHome,

            540 => Key::AltIns,
            // 541: AltShiftIns?
            542 => Key::CtrlIns,
            // 543: CtrlShiftIns?
            544 => Key::CtrlAltIns,

            545 => Key::AltLeft,
            546 => Key::AltShiftLeft,
            547 => Key::CtrlLeft,
            548 => Key::CtrlShiftLeft,
            549 => Key::CtrlAltLeft,

            550 => Key::AltPageDown,
            551 => Key::AltShiftPageDown,
            552 => Key::CtrlPageDown,
            553 => Key::CtrlShiftPageDown,
            554 => Key::CtrlAltPageDown,

            555 => Key::AltPageUp,
            556 => Key::AltShiftPageUp,
            557 => Key::CtrlPageUp,
            558 => Key::CtrlShiftPageUp,
            559 => Key::CtrlAltPageUp,

            560 => Key::AltRight,
            561 => Key::AltShiftRight,
            562 => Key::CtrlRight,
            563 => Key::CtrlShiftRight,
            564 => Key::CtrlAltRight,
            // 565?

            566 => Key::AltUp,
            567 => Key::AltShiftUp,
            568 => Key::CtrlUp,
            569 => Key::CtrlShiftUp,
            570 => Key::CtrlAltUp,

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
            ncurses::KEY_SNEXT => Key::ShiftPageDown,
            ncurses::KEY_SPREVIOUS => Key::ShiftPageUp,
            // All Fn keys use the same enum with associated number
            f @ ncurses::KEY_F1...ncurses::KEY_F12 => Key::F((f - ncurses::KEY_F0) as u8),
            f @ 277...288 => Key::ShiftF((f - 281 + 5) as u8),
            f @ 289...300 => Key::CtrlF((f - 293 + 5) as u8),
            f @ 301...312 => Key::CtrlShiftF((f - 305 + 5) as u8),
            // Shift and Ctrl F{1-4} need escape sequences...
            //
            // TODO: shift and ctrl Fn keys
            // Avoids 8-10 (H,I,J), they are used by other commands.
            c @ 1...7 | c @ 11...25 => Key::CtrlChar(('a' as u8 + (c - 1) as u8) as char),
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
            key => {
                write!(f,
                       "{}",
                       match key {
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
