//! User-input events and their effects.

use std::rc::Rc;

use Cursive;

/// Callback is a function that can be triggered by an event.
/// It has a mutable access to the cursive root.
pub type Callback = Rc<Fn(&mut Cursive)>;

/// Answer to an event notification.
/// The event can be consumed or ignored.
pub enum EventResult {
    /// The event was ignored. The parent can keep handling it.
    Ignored,
    /// The event was consumed. An optionnal callback to run is attached.
    Consumed(Option<Callback>),
}

impl EventResult {
    /// Convenient method to create `Consumed(Some(f))`
    pub fn with_cb<F: 'static + Fn(&mut Cursive)>(f: F) -> Self {
        EventResult::Consumed(Some(Rc::new(f)))
    }
}

/// A non-character key on the keyboard
#[derive(PartialEq,Eq,Clone,Copy,Hash,Debug)]
pub enum Key {
    /// Both Enter (or Return) and numpad Enter
    Enter,
    /// Tabulation key
    Tab,
    /// Backspace key
    Backspace,
    /// Escape key
    Esc,

    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,

    /// Insert key
    Ins,
    /// Delete key
    Del,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,

    /// The 5 in the center of the keypad, when numlock is disabled.
    NumpadCenter,

    /// F1 key
    F1,
    /// F2 key
    F2,
    /// F3 key
    F3,
    /// F4 key
    F4,
    /// F5 key
    F5,
    /// F6 key
    F6,
    /// F7 key
    F7,
    /// F8 key
    F8,
    /// F9 key
    F9,
    /// F10 key
    F10,
    /// F11 key
    F11,
    /// F12 key
    F12,
}

impl Key {
    /// Returns the function key corresponding to the given number
    ///
    /// 1 -> F1, etc...
    ///
    /// # Panics
    ///
    /// If `n == 0 || n > 12`
    pub fn from_f(n: u8) -> Key {
        match n {
            1 => Key::F1,
            2 => Key::F2,
            3 => Key::F3,
            4 => Key::F4,
            5 => Key::F5,
            6 => Key::F6,
            7 => Key::F7,
            8 => Key::F8,
            9 => Key::F9,
            10 => Key::F10,
            11 => Key::F11,
            12 => Key::F12,
            _ => panic!("unknown function key: F{}", n),
        }
    }
}

/// Represents an event as seen by the application.
#[derive(PartialEq,Eq,Clone,Copy,Hash,Debug)]
pub enum Event {
    /// Event fired when the window is resized
    WindowResize,

    /// A character was entered (includes numbers, punctuation, ...)
    Char(char),
    /// A character was entered with the Ctrl key pressed
    CtrlChar(char),
    /// A character was entered with the Alt key pressed
    AltChar(char),

    /// A non-character key was pressed
    Key(Key),
    /// A non-character key was pressed with the Shift key pressed
    Shift(Key),
    /// A non-character key was pressed with the Alt key pressed
    Alt(Key),
    /// A non-character key was pressed with the Shift and Alt keys pressed
    AltShift(Key),
    /// A non-character key was pressed with the Ctrl key pressed
    Ctrl(Key),
    /// A non-character key was pressed with the Ctrl and Shift keys pressed
    CtrlShift(Key),
    /// A non-character key was pressed with the Ctrl and Alt keys pressed
    CtrlAlt(Key),

    /// An unknown event was received.
    Unknown(i32),
}

impl From<char> for Event {
    fn from(c: char) -> Event {
        Event::Char(c)
    }
}

impl From<Key> for Event {
    fn from(k: Key) -> Event {
        Event::Key(k)
    }
}
