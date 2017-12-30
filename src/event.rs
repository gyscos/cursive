//! User-input events and their effects.
//!
//! * Every user input the application receives is converted to an
//!   [`Event`](./enum.Event.html).
//! * Each event is then given to the root, and descends the view tree down to
//!   the view currently in focus, through the
//!   [`on_event`](../view/trait.View.html#method.on_event) method.
//!     * If the view consumes the event, it may return a callback to be
//!       executed.
//!     * Otherwise, it ignores the event, and the view parent can in turn
//!       choose to consume it or not.
//! * If no view consumes the event, the
//!   [global callback](../struct.Cursive.html#method.add_global_callback)
//!   table is checked.

use Cursive;
use std::ops::Deref;
use std::rc::Rc;
use vec::Vec2;

/// Callback is a function that can be triggered by an event.
/// It has a mutable access to the cursive root.
#[derive(Clone)]
pub struct Callback(Rc<Box<Fn(&mut Cursive)>>);
// TODO: remove the Box when Box<T: Sized> -> Rc<T> is possible

impl Callback {
    /// Wraps the given function into a `Callback` object.
    pub fn from_fn<F, R>(f: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) -> R,
    {
        Callback(Rc::new(Box::new(move |siv| {
            f(siv);
        })))
    }
}

impl Deref for Callback {
    type Target = Box<Fn(&mut Cursive)>;
    fn deref<'a>(&'a self) -> &'a Box<Fn(&mut Cursive)> {
        &self.0
    }
}

impl From<Rc<Box<Fn(&mut Cursive)>>> for Callback {
    fn from(f: Rc<Box<Fn(&mut Cursive)>>) -> Self {
        Callback(f)
    }
}

impl From<Box<Fn(&mut Cursive) + Send>> for Callback {
    fn from(f: Box<Fn(&mut Cursive) + Send>) -> Self {
        Callback(Rc::new(f))
    }
}

impl From<Box<Fn(&mut Cursive)>> for Callback {
    fn from(f: Box<Fn(&mut Cursive)>) -> Self {
        Callback(Rc::new(f))
    }
}

/// Answer to an event notification.
/// The event can be consumed or ignored.
pub enum EventResult {
    /// The event was ignored. The parent can keep handling it.
    Ignored,
    /// The event was consumed. An optionnal callback to run is attached.
    Consumed(Option<Callback>), // TODO: make this a FnOnce?
}

impl EventResult {
    /// Convenient method to create `Consumed(Some(f))`
    pub fn with_cb<F, R>(f: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) -> R,
    {
        EventResult::Consumed(Some(Callback::from_fn(f)))
    }

    /// Returns `true` if `self` is `EventResult::Consumed`.
    pub fn is_consumed(&self) -> bool {
        match *self {
            EventResult::Consumed(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if `self` contains a callback.
    pub fn has_callback(&self) -> bool {
        match *self {
            EventResult::Consumed(Some(_)) => true,
            _ => false,
        }
    }

    /// Process this result if it is a callback.
    ///
    /// Does nothing otherwise.
    pub fn process(self, s: &mut Cursive) {
        if let EventResult::Consumed(Some(cb)) = self {
            cb(s);
        }
    }

    /// Returns `self` if it is not `Event::Ignored`, otherwise returns `f()`.
    pub fn or_else<F>(self, f: F) -> Self
    where
        F: FnOnce() -> EventResult,
    {
        match self {
            EventResult::Ignored => f(),
            other => other,
        }
    }
}

/// A non-character key on the keyboard
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
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

    /// Pause Break key
    PauseBreak,

    /// The 5 in the center of the keypad, when numlock is disabled.
    NumpadCenter,

    /// F0 key
    F0,
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
            0 => Key::F0,
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

/// One of the buttons present on the mouse
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum MouseButton {
    /// The left button, used for main actions.
    Left,
    /// Middle button, probably the wheel. Often pastes text in X11 on linux.
    Middle,
    /// The right button, for special actions.
    Right,

    /// Fourth button if the mouse supports it.
    Button4,
    /// Fifth button if the mouse supports it.
    Button5,

    // TODO: handle more buttons?
    #[doc(hidden)] Other,
}

/// Represents a possible event sent by the mouse.
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum MouseEvent {
    /// A button was pressed.
    Press(MouseButton),
    /// A button was released.
    Release(MouseButton),
    /// A button is being held.
    Hold(MouseButton),
    /// The wheel was moved up.
    WheelUp,
    /// The wheel was moved down.
    WheelDown,
}

impl MouseEvent {
    /// Returns the button used by this event, if any.
    ///
    /// Returns `None` if `self` is `WheelUp` or `WheelDown`.
    pub fn button(&self) -> Option<MouseButton> {
        match *self {
            MouseEvent::Press(btn)
            | MouseEvent::Release(btn)
            | MouseEvent::Hold(btn) => Some(btn),
            _ => None,
        }
    }

    /// Returns `true` if `self` is an event that can grab focus.
    ///
    /// This includes `Press`, `WheelUp` and `WheelDown`.
    pub fn grabs_focus(self) -> bool {
        match self {
            MouseEvent::Press(_)
            | MouseEvent::WheelUp
            | MouseEvent::WheelDown => true,
            _ => false,
        }
    }
}

/// Represents an event as seen by the application.
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub enum Event {
    /// Event fired when the window is resized.
    WindowResize,

    /// Event fired regularly when a auto-refresh is set.
    Refresh,

    // TODO: have Char(modifier, char) and Key(modifier, key) enums?
    /// A character was entered (includes numbers, punctuation, ...).
    Char(char),
    /// A character was entered with the Ctrl key pressed.
    CtrlChar(char),
    /// A character was entered with the Alt key pressed.
    AltChar(char),

    /// A non-character key was pressed.
    Key(Key),
    /// A non-character key was pressed with the Shift key pressed.
    Shift(Key),
    /// A non-character key was pressed with the Alt key pressed.
    Alt(Key),
    /// A non-character key was pressed with the Shift and Alt keys pressed.
    AltShift(Key),
    /// A non-character key was pressed with the Ctrl key pressed.
    Ctrl(Key),
    /// A non-character key was pressed with the Ctrl and Shift keys pressed.
    CtrlShift(Key),
    /// A non-character key was pressed with the Ctrl and Alt keys pressed.
    CtrlAlt(Key),

    /// A mouse event was sent.
    Mouse {
        /// Position of the top-left corner of the view receiving this event.
        offset: Vec2,
        /// Position of the mouse when this event was fired.
        position: Vec2,
        /// The mouse event itself.
        event: MouseEvent,
    },

    // TODO: use a backend-dependent type for the unknown values?
    /// An unknown event was received.
    Unknown(Vec<u8>),

    // Maybe add a `Custom(Rc<Any>)` ?

    // Having a doc-hidden event prevents people from having exhaustive
    // matches, allowing us to add events in the future.
    #[doc(hidden)]
    /// The application is about to exit.
    Exit,
}

impl Event {
    /// Returns the position of the mouse, if `self` is a mouse event.
    pub fn mouse_position(&self) -> Option<Vec2> {
        if let Event::Mouse { position, .. } = *self {
            Some(position)
        } else {
            None
        }
    }

    /// Update `self` with the given offset.
    ///
    /// If `self` is a mouse event, adds `top_left` to its offset.
    /// Otherwise, do nothing.
    pub fn relativize<V>(&mut self, top_left: V)
    where
        V: Into<Vec2>,
    {
        if let Event::Mouse { ref mut offset, .. } = *self {
            *offset = *offset + top_left;
        }
    }

    /// Returns a cloned, relativized event.
    ///
    /// If `self` is a mouse event, adds `top_left` to its offset.
    /// Otherwise, returns a simple clone.
    pub fn relativized<V>(&self, top_left: V) -> Self
    where
        V: Into<Vec2>,
    {
        let mut result = self.clone();
        result.relativize(top_left);
        result
    }
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
