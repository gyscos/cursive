//! User-input events and their effects.
//!
//! * Every user input the application receives is converted to an [`Event`].
//! * Each event is then given to the root, and descends the view tree down to
//!   the view currently in focus, through the [`on_event`] method.
//!     * If the view consumes the event, it may return a callback to be
//!       executed.
//!     * Otherwise, it ignores the event, and the view parent can in turn
//!       choose to consume it or not.
//! * If no view consumes the event, the [global callback] table is checked.
//!
//! [`Event`]: enum.Event.html
//! [`on_event`]: ../trait.View.html#method.on_event
//! [global callback]: ../struct.Cursive.html#method.add_global_callback

use crate::Cursive;
use crate::Vec2;
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

/// Callback is a function that can be triggered by an event.
/// It has a mutable access to the cursive root.
///
/// It is meant to be stored in views.
#[derive(Clone)]
pub struct Callback(Arc<dyn Fn(&mut Cursive) + Send + Sync>);
// TODO: remove the Box when Box<T: Sized> -> Arc<T> is possible

/// A callback that can be run on `&mut dyn View`.
///
/// It is meant to be used as parameter in `View::call_on_any`, and not much else.
pub type AnyCb<'a> = &'a mut dyn FnMut(&mut dyn crate::view::View);

/// A trigger that only selects some types of events.
///
/// It is meant to be stored in views.
pub struct EventTrigger {
    // A function called on each individual event to know if it applies.
    trigger: Box<dyn Fn(&Event) -> bool + Send + Sync>,

    // Some marker to indicate the origin.
    //
    // In practice it could be a `&'static str` describing the trigger, or an `Event` for
    // single-event triggers.
    //
    // TODO: Require `Debug` on the tag, so we could implement `Debug` for `EventTrigger`?
    tag: Box<dyn AnyTag + Send + Sync>,
}

impl std::fmt::Debug for EventTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // For some well-known types we can print something.
        if let Some(event) = self.tag.as_any().downcast_ref::<Event>() {
            return write!(f, "EventTrigger {{ {event:?} }}");
        }

        if let Some(str) = self.tag.as_any().downcast_ref::<&'static str>() {
            return write!(f, "EventTrigger {{ {str:?} }}");
        }

        // But in the general case right now we can only guess
        f.write_str("EventTrigger { ? }")
    }
}

trait AnyTag: Any + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnyTag for T
where
    T: Any + std::fmt::Debug,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EventTrigger {
    /// Create a new `EventTrigger` using the given function as filter.
    pub fn from_fn<F>(f: F) -> Self
    where
        F: 'static + Fn(&Event) -> bool + Send + Sync,
    {
        EventTrigger::from_fn_and_tag(f, "free function")
    }

    /// Create a new `EventTrigger`.
    pub fn from_fn_and_tag<F, T>(f: F, tag: T) -> Self
    where
        F: 'static + Fn(&Event) -> bool + Send + Sync,
        T: Any + std::fmt::Debug + Send + Sync,
    {
        let tag = Box::new(tag);
        let trigger = Box::new(f);
        EventTrigger { trigger, tag }
    }

    /// Check if this trigger has the given tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::event::{Event, EventTrigger};
    ///
    /// let event = Event::CtrlChar('c');
    /// let trigger: EventTrigger = event.clone().into();
    /// assert!(
    ///     trigger.has_tag(&event),
    ///     "Trigger does not recognize its own tag."
    /// );
    /// ```
    pub fn has_tag<T: PartialEq + 'static>(&self, tag: &T) -> bool {
        (*self.tag).as_any().downcast_ref::<T>() == Some(tag)
    }

    /// Checks if this trigger applies to the given `Event`.
    pub fn apply(&self, event: &Event) -> bool {
        (self.trigger)(event)
    }

    /// Returns an `EventTrigger` that only accepts arrow keys.
    ///
    /// Only bare arrow keys without modifiers (Shift, Ctrl, Alt) will be accepted.
    pub fn arrows() -> Self {
        Self::from_fn_and_tag(
            |e| {
                matches!(
                    e,
                    Event::Key(Key::Left)
                        | Event::Key(Key::Down)
                        | Event::Key(Key::Up)
                        | Event::Key(Key::Right)
                )
            },
            "arrows",
        )
    }

    /// Returns an `EventTrigger` that only accepts mouse events.
    pub fn mouse() -> Self {
        Self::from_fn_and_tag(|e| matches!(e, Event::Mouse { .. }), "mouse")
    }

    /// Returns an `EventTrigger` that accepts any event.
    pub fn any() -> Self {
        Self::from_fn_and_tag(|_| true, "any")
    }

    /// Returns an `EventTrigger` that doesn't accept any event.
    pub fn none() -> Self {
        Self::from_fn_and_tag(|_| false, "none")
    }

    /// Returns an `EventTrigger` that applies if either `self` or `other` applies.
    #[must_use]
    pub fn or<O>(self, other: O) -> Self
    where
        O: Into<EventTrigger>,
    {
        let other = other.into();

        let self_trigger = self.trigger;
        let other_trigger = other.trigger;
        let tag = (self.tag, "or", other.tag);

        Self::from_fn_and_tag(move |e| self_trigger(e) || other_trigger(e), tag)
    }
}

impl From<Event> for EventTrigger {
    fn from(event: Event) -> Self {
        let tag = event.clone();
        Self::from_fn_and_tag(move |e| *e == event, tag)
    }
}

impl From<char> for EventTrigger {
    fn from(c: char) -> Self {
        Self::from(Event::from(c))
    }
}

impl From<Key> for EventTrigger {
    fn from(k: Key) -> Self {
        Self::from(Event::from(k))
    }
}

impl<F> From<F> for EventTrigger
where
    F: 'static + Fn(&Event) -> bool + Send + Sync,
{
    fn from(f: F) -> Self {
        Self::from_fn(f)
    }
}

impl Callback {
    /// Wraps the given function into a `Callback` object.
    pub fn from_fn<F>(f: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        Callback(Arc::new(move |siv| {
            f(siv);
        }))
    }

    /// Wrap a `FnMut` into a `Callback` object.
    ///
    /// If this methods tries to call itself, nested calls will be no-ops.
    pub fn from_fn_mut<F>(f: F) -> Self
    where
        F: 'static + FnMut(&mut Cursive) + Send,
    {
        Self::from_fn(crate::immut1!(f))
    }

    /// Wrap a `FnOnce` into a `Callback` object.
    ///
    /// After being called once, the callback will become a no-op.
    pub fn from_fn_once<F>(f: F) -> Self
    where
        F: 'static + FnOnce(&mut Cursive) + Send,
    {
        Self::from_fn_mut(crate::once1!(f))
    }

    /// Returns a dummy callback that doesn't run anything.
    pub fn dummy() -> Self {
        Callback::from_fn(|_| ())
    }
}

impl Deref for Callback {
    type Target = dyn Fn(&mut Cursive) + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl From<Arc<dyn Fn(&mut Cursive) + Send + Sync>> for Callback {
    fn from(f: Arc<dyn Fn(&mut Cursive) + Send + Sync>) -> Self {
        Callback(f)
    }
}

impl From<Box<dyn Fn(&mut Cursive) + Send + Sync>> for Callback {
    fn from(f: Box<dyn Fn(&mut Cursive) + Send + Sync>) -> Self {
        Callback(Arc::from(f))
    }
}

/// Answer to an event notification.
/// The event can be consumed or ignored.
pub enum EventResult {
    /// The event was ignored. The parent can keep handling it.
    Ignored,
    /// The event was consumed. An optional callback to run is attached.
    Consumed(Option<Callback>), // TODO: make this a FnOnce?
}

impl std::fmt::Debug for EventResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventResult::Ignored => f.write_str("EventResult::Ignored"),
            EventResult::Consumed(None) => f.write_str("EventResult::Consumed(None)"),
            EventResult::Consumed(_) => f.write_str("EventResult::Consumed(Some(_))"),
        }
    }
}

impl EventResult {
    /// Convenient method to create `Consumed(Some(f))`
    pub fn with_cb<F>(f: F) -> Self
    where
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        EventResult::Consumed(Some(Callback::from_fn(f)))
    }

    /// Convenient method to create `Consumed(Some(f))`
    ///
    /// After being called once, the callback will become a no-op.
    pub fn with_cb_once<F>(f: F) -> Self
    where
        F: 'static + FnOnce(&mut Cursive) + Send,
    {
        EventResult::Consumed(Some(Callback::from_fn_once(f)))
    }

    /// Convenient method to create `Consumed(None)`
    pub fn consumed() -> Self {
        EventResult::Consumed(None)
    }

    /// Returns `true` if `self` is `EventResult::Consumed`.
    pub fn is_consumed(&self) -> bool {
        matches!(*self, EventResult::Consumed(_))
    }

    /// Returns `true` if `self` contains a callback.
    pub fn has_callback(&self) -> bool {
        matches!(*self, EventResult::Consumed(Some(_)))
    }

    /// Process this result if it is a callback.
    ///
    /// Does nothing otherwise.
    pub fn process(self, s: &mut Cursive) {
        if let EventResult::Consumed(Some(cb)) = self {
            cb(s);
        }
    }

    /// Returns `self` if it is not `EventResult::Ignored`, otherwise returns `f()`.
    #[must_use]
    pub fn or_else<F>(self, f: F) -> Self
    where
        F: FnOnce() -> EventResult,
    {
        match self {
            EventResult::Ignored => f(),
            other => other,
        }
    }

    /// Returns an event result that combines `self` and `other`.
    #[must_use]
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (EventResult::Ignored, result) | (result, EventResult::Ignored) => result,
            (EventResult::Consumed(None), EventResult::Consumed(cb))
            | (EventResult::Consumed(cb), EventResult::Consumed(None)) => EventResult::Consumed(cb),
            (EventResult::Consumed(Some(cb1)), EventResult::Consumed(Some(cb2))) => {
                EventResult::with_cb(move |siv| {
                    (cb1)(siv);
                    (cb2)(siv);
                })
            }
        }
    }

    /// Combines the given event results into a single one.
    ///
    /// If `results` is empty or if all results are `Ignored`, returns `Ignored`.
    ///
    /// Otherwise, returns a callback that runs all callback in results.
    pub fn combine(results: Vec<Self>) -> Self {
        if results.iter().all(|result| !result.is_consumed()) {
            return EventResult::Ignored;
        }

        // TODO: if all events are `Ignored` or `Consumed(None)`,
        // returns `Consumed(None)` and save the allocation?
        EventResult::with_cb_once(move |siv| {
            for res in results {
                res.process(siv);
            }
        })
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
    /// If `n > 12`
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
            _ => panic!("unknown function key: F{n}"),
        }
    }
}

/// One of the buttons present on the mouse
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
#[non_exhaustive]
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

    // TODO: handle more buttons? Wheel left/right?
    #[doc(hidden)]
    Other,
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
    /// The wheel was moved to the left.
    WheelLeft,
    /// The wheel was moved to the right.
    WheelRight,
}

impl MouseEvent {
    /// Returns the button used by this event, if any.
    ///
    /// Returns `None` if `self` is `WheelUp`, `WheelDown`, `WheelLeft` or `WheelRight`.
    pub fn button(self) -> Option<MouseButton> {
        match self {
            MouseEvent::Press(btn) | MouseEvent::Release(btn) | MouseEvent::Hold(btn) => Some(btn),
            _ => None,
        }
    }

    /// Returns `true` if `self` is an event that can grab focus.
    ///
    /// This includes `Press`, `WheelUp`, `WheelDown`, `WheelLeft` and `WheelRight`.
    ///
    /// It does _not_ include `Release` or `Hold`.
    ///
    /// It means you should be able to grab a scroll bar, and move the mouse
    /// away from the view, without actually changing the focus.
    pub fn grabs_focus(self) -> bool {
        matches!(
            self,
            MouseEvent::Press(_)
                | MouseEvent::WheelUp
                | MouseEvent::WheelDown
                | MouseEvent::WheelLeft
                | MouseEvent::WheelRight
        )
    }
}

/// Represents an event as seen by the application.
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub enum Event {
    /// Event fired when the window is resized.
    WindowResize,

    /// Event fired when the view is about to lose focus.
    FocusLost,

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

    // Maybe add a `Custom(Arc<Any>)` ?

    // Having a doc-hidden event prevents people from having exhaustive
    // matches, allowing us to add events in the future.
    //
    // In addition we may not want people to listen to the exit event?
    #[doc(hidden)]
    /// The application is about to exit.
    Exit,
}

impl Event {
    /// Returns the character, if `self` is a char event.
    pub fn char(&self) -> Option<char> {
        match *self {
            Event::Char(c) => Some(c),
            Event::AltChar(c) => Some(c),
            Event::CtrlChar(c) => Some(c),
            _ => None,
        }
    }

    /// Returns the position of the mouse, if `self` is a mouse event.
    pub fn mouse_position(&self) -> Option<Vec2> {
        if let Event::Mouse { position, .. } = *self {
            Some(position)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the position of the mouse.
    ///
    /// Returns `None` if `self` is not a mouse event.
    pub fn mouse_position_mut(&mut self) -> Option<&mut Vec2> {
        if let Event::Mouse {
            ref mut position, ..
        } = *self
        {
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
    #[must_use]
    pub fn relativized<V>(&self, top_left: V) -> Self
    where
        V: Into<Vec2>,
    {
        let mut result = self.clone();
        result.relativize(top_left);
        result
    }
}

/// Error returned when a string cannot be parsed as a [`Key`] or an [`Event`].
///
/// The message names the offending input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEventError {
    input: String,
    reason: &'static str,
}

impl ParseEventError {
    fn new(input: &str, reason: &'static str) -> Self {
        ParseEventError {
            input: input.to_string(),
            reason,
        }
    }

    /// The input that could not be parsed.
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl std::fmt::Display for ParseEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot parse {:?} as an event: {}",
            self.input, self.reason
        )
    }
}

impl std::error::Error for ParseEventError {}

// Canonical name of each key. `Display` prints these, `FromStr` accepts them in any case.
const KEY_NAMES: &[(Key, &str)] = &[
    (Key::Enter, "Enter"),
    (Key::Tab, "Tab"),
    (Key::Backspace, "Backspace"),
    (Key::Esc, "Esc"),
    (Key::Left, "Left"),
    (Key::Right, "Right"),
    (Key::Up, "Up"),
    (Key::Down, "Down"),
    (Key::Ins, "Ins"),
    (Key::Del, "Del"),
    (Key::Home, "Home"),
    (Key::End, "End"),
    (Key::PageUp, "PageUp"),
    (Key::PageDown, "PageDown"),
    (Key::PauseBreak, "PauseBreak"),
    (Key::NumpadCenter, "NumpadCenter"),
    (Key::F0, "F0"),
    (Key::F1, "F1"),
    (Key::F2, "F2"),
    (Key::F3, "F3"),
    (Key::F4, "F4"),
    (Key::F5, "F5"),
    (Key::F6, "F6"),
    (Key::F7, "F7"),
    (Key::F8, "F8"),
    (Key::F9, "F9"),
    (Key::F10, "F10"),
    (Key::F11, "F11"),
    (Key::F12, "F12"),
];

// Extra names accepted when parsing (compared case-insensitively).
const KEY_ALIASES: &[(Key, &str)] = &[
    (Key::Enter, "return"),
    (Key::Esc, "escape"),
    (Key::Ins, "insert"),
    (Key::Del, "delete"),
    (Key::PageUp, "pgup"),
    (Key::PageDown, "pgdn"),
];

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = KEY_NAMES
            .iter()
            .find(|(k, _)| k == self)
            .map_or("?", |(_, name)| name);
        f.write_str(name)
    }
}

/// Parses a key from its name.
///
/// The name is the name of the variant, case-insensitive: `Esc`, `enter`, `PageUp`, `F5`...
/// A few aliases are also accepted: `return`, `escape`, `insert`, `delete`, `pgup` and `pgdn`.
///
/// `Display` prints the variant name, which parses back to the same key.
///
/// # Examples
///
/// ```
/// use cursive_core::event::Key;
///
/// assert_eq!("Esc".parse::<Key>(), Ok(Key::Esc));
/// assert_eq!("pageup".parse::<Key>(), Ok(Key::PageUp));
/// assert!("nope".parse::<Key>().is_err());
/// ```
impl std::str::FromStr for Key {
    type Err = ParseEventError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        KEY_NAMES
            .iter()
            .chain(KEY_ALIASES)
            .find(|(_, name)| name.eq_ignore_ascii_case(s))
            .map(|(key, _)| *key)
            .ok_or_else(|| ParseEventError::new(s, "unknown key name"))
    }
}

/// Parses an event from a string like `ctrl+c`, `alt+shift+up` or `F5`.
///
/// The syntax is some optional modifiers (`ctrl`, `alt`, `shift`; `control` is also accepted),
/// each followed by `+`, and then a key name (see [`Key`]) or a single character.
/// Modifiers and key names are case-insensitive, and modifiers can come in any order.
/// A single character is case-sensitive: `a` and `A` are different events.
///
/// Only combinations that exist as an [`Event`] variant are accepted, so `ctrl+alt+c`,
/// `shift+a` or `ctrl+alt+shift+left` are errors, as are repeated modifiers. With a character,
/// the possible modifiers are `ctrl` or `alt` (alone). With a key, any of `shift`, `alt`,
/// `ctrl`, `alt+shift`, `ctrl+shift` or `ctrl+alt`.
///
/// A trailing `+` is the character `+`: `+` is `Char('+')` and `ctrl++` is `CtrlChar('+')`.
/// A single space is `Char(' ')`, and `space` is accepted as well.
/// Surrounding whitespace is otherwise ignored.
///
/// Three events without data have a name: `refresh` ([`Event::Refresh`]), `resize`
/// ([`Event::WindowResize`]) and `focus_lost` ([`Event::FocusLost`]). They take no modifiers.
///
/// Mouse events, unknown events and the exit event cannot be parsed.
///
/// # Examples
///
/// ```
/// use cursive_core::event::{Event, Key};
///
/// assert_eq!("ctrl+c".parse::<Event>(), Ok(Event::CtrlChar('c')));
/// assert_eq!(
///     "Alt+Shift+Up".parse::<Event>(),
///     Ok(Event::AltShift(Key::Up))
/// );
/// assert_eq!("a".parse::<Event>(), Ok(Event::Char('a')));
/// assert_eq!("F5".parse::<Event>(), Ok(Event::Key(Key::F5)));
/// assert_eq!("ctrl++".parse::<Event>(), Ok(Event::CtrlChar('+')));
///
/// // There is no event for this combination.
/// assert!("ctrl+alt+c".parse::<Event>().is_err());
/// ```
impl std::str::FromStr for Event {
    type Err = ParseEventError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let err = |reason| ParseEventError::new(input, reason);

        // A single character is always that character (even a space).
        let mut it = input.chars();
        if let (Some(c), None) = (it.next(), it.next()) {
            return Ok(Event::Char(c));
        }

        let s = input.trim();
        if s.is_empty() {
            return Err(err("empty input"));
        }

        // A trailing `+` is the character, not a separator.
        let (modifiers, key) = if let Some(prefix) = s.strip_suffix("++") {
            (Some(prefix), "+")
        } else if s == "+" {
            (None, "+")
        } else if s.ends_with('+') {
            return Err(err("missing key after '+'"));
        } else {
            match s.rsplit_once('+') {
                Some((prefix, key)) => (Some(prefix), key),
                None => (None, s),
            }
        };

        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        for modifier in modifiers.into_iter().flat_map(|m| m.split('+')) {
            let flag = match modifier.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => &mut ctrl,
                "alt" => &mut alt,
                "shift" => &mut shift,
                "" => return Err(err("empty modifier")),
                _ => return Err(err("unknown modifier")),
            };
            if std::mem::replace(flag, true) {
                return Err(err("repeated modifier"));
            }
        }
        let mods = (ctrl, alt, shift);

        // Trim the key, unless it is a lone character (which could be whitespace).
        let key = if key.chars().count() == 1 {
            key
        } else {
            key.trim()
        };
        let mut it = key.chars();
        let single = match (it.next(), it.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        let lower = key.to_ascii_lowercase();

        let c = match (single, lower.as_str()) {
            (Some(c), _) => Some(c),
            (None, "space") => Some(' '),
            _ => None,
        };
        if let Some(c) = c {
            return match mods {
                (false, false, false) => Ok(Event::Char(c)),
                (true, false, false) => Ok(Event::CtrlChar(c)),
                (false, true, false) => Ok(Event::AltChar(c)),
                _ => Err(err(
                    "no event for this combination of modifiers and character",
                )),
            };
        }

        let simple = match lower.as_str() {
            "refresh" => Some(Event::Refresh),
            "resize" => Some(Event::WindowResize),
            "focus_lost" => Some(Event::FocusLost),
            _ => None,
        };
        if let Some(event) = simple {
            return if mods == (false, false, false) {
                Ok(event)
            } else {
                Err(err("this event takes no modifiers"))
            };
        }

        let key: Key = key.parse().map_err(|_| err("unknown key name"))?;
        match mods {
            (false, false, false) => Ok(Event::Key(key)),
            (false, false, true) => Ok(Event::Shift(key)),
            (false, true, false) => Ok(Event::Alt(key)),
            (false, true, true) => Ok(Event::AltShift(key)),
            (true, false, false) => Ok(Event::Ctrl(key)),
            (true, false, true) => Ok(Event::CtrlShift(key)),
            (true, true, false) => Ok(Event::CtrlAlt(key)),
            (true, true, true) => Err(err("no event for ctrl+alt+shift")),
        }
    }
}

/// Prints an event in the syntax accepted by `FromStr`: `ctrl+c`, `alt+shift+Up`, `space`...
///
/// Modifiers are printed in the order `ctrl`, `alt`, `shift`, in lowercase.
///
/// Mouse, unknown and exit events cannot be parsed back: they are printed in their `Debug`
/// form, which `FromStr` rejects.
///
/// # Examples
///
/// ```
/// use cursive_core::event::{Event, Key};
///
/// assert_eq!(Event::CtrlChar('c').to_string(), "ctrl+c");
/// assert_eq!(Event::CtrlAlt(Key::Left).to_string(), "ctrl+alt+Left");
/// assert_eq!(Event::Char(' ').to_string(), "space");
/// ```
impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn ch(f: &mut std::fmt::Formatter<'_>, prefix: &str, c: char) -> std::fmt::Result {
            if c == ' ' {
                write!(f, "{prefix}space")
            } else {
                write!(f, "{prefix}{c}")
            }
        }

        match self {
            Event::WindowResize => f.write_str("resize"),
            Event::FocusLost => f.write_str("focus_lost"),
            Event::Refresh => f.write_str("refresh"),
            Event::Char(c) => ch(f, "", *c),
            Event::CtrlChar(c) => ch(f, "ctrl+", *c),
            Event::AltChar(c) => ch(f, "alt+", *c),
            Event::Key(k) => write!(f, "{k}"),
            Event::Shift(k) => write!(f, "shift+{k}"),
            Event::Alt(k) => write!(f, "alt+{k}"),
            Event::AltShift(k) => write!(f, "alt+shift+{k}"),
            Event::Ctrl(k) => write!(f, "ctrl+{k}"),
            Event::CtrlShift(k) => write!(f, "ctrl+shift+{k}"),
            Event::CtrlAlt(k) => write!(f, "ctrl+alt+{k}"),
            // These do not round-trip.
            Event::Mouse { .. } | Event::Unknown(_) | Event::Exit => write!(f, "{self:?}"),
        }
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

#[cfg(test)]
mod parse_tests {
    use super::*;

    fn parse(s: &str) -> Result<Event, ParseEventError> {
        s.parse()
    }

    #[test]
    fn key_round_trip() {
        assert_eq!(KEY_NAMES.len(), 29);
        for (key, name) in KEY_NAMES {
            assert_eq!(key.to_string(), *name);
            assert_eq!(name.parse::<Key>(), Ok(*key));
            assert_eq!(name.to_lowercase().parse::<Key>(), Ok(*key));
            assert_eq!(name.to_uppercase().parse::<Key>(), Ok(*key));
        }
        for (key, alias) in KEY_ALIASES {
            assert_eq!(alias.parse::<Key>(), Ok(*key));
        }
        assert!("".parse::<Key>().is_err());
        assert!("F13".parse::<Key>().is_err());
    }

    #[test]
    fn table() {
        let cases: Vec<(&str, Event)> = vec![
            ("a", Event::Char('a')),
            ("A", Event::Char('A')),
            ("5", Event::Char('5')),
            ("é", Event::Char('é')),
            ("Esc", Event::Key(Key::Esc)),
            ("enter", Event::Key(Key::Enter)),
            ("PageUp", Event::Key(Key::PageUp)),
            ("pgdn", Event::Key(Key::PageDown)),
            ("escape", Event::Key(Key::Esc)),
            ("F5", Event::Key(Key::F5)),
            ("ctrl+c", Event::CtrlChar('c')),
            ("control+c", Event::CtrlChar('c')),
            ("CTRL+C", Event::CtrlChar('C')),
            ("alt+x", Event::AltChar('x')),
            ("shift+tab", Event::Shift(Key::Tab)),
            ("alt+left", Event::Alt(Key::Left)),
            ("ctrl+left", Event::Ctrl(Key::Left)),
            ("alt+shift+up", Event::AltShift(Key::Up)),
            ("shift+alt+up", Event::AltShift(Key::Up)),
            ("ctrl+shift+Home", Event::CtrlShift(Key::Home)),
            ("ctrl+alt+left", Event::CtrlAlt(Key::Left)),
            ("Alt+Ctrl+Left", Event::CtrlAlt(Key::Left)),
            ("refresh", Event::Refresh),
            ("Resize", Event::WindowResize),
            ("FOCUS_LOST", Event::FocusLost),
            ("  ctrl+c  ", Event::CtrlChar('c')),
            ("ctrl + c", Event::CtrlChar('c')),
        ];
        for (s, e) in cases {
            assert_eq!(parse(s), Ok(e), "parsing {s:?}");
        }
    }

    #[test]
    fn plus_and_space() {
        assert_eq!(parse("+"), Ok(Event::Char('+')));
        assert_eq!(parse("ctrl++"), Ok(Event::CtrlChar('+')));
        assert_eq!(parse("alt++"), Ok(Event::AltChar('+')));
        assert_eq!(parse(" "), Ok(Event::Char(' ')));
        assert_eq!(parse("space"), Ok(Event::Char(' ')));
        assert_eq!(parse("Space"), Ok(Event::Char(' ')));
        assert_eq!(parse("ctrl+space"), Ok(Event::CtrlChar(' ')));
        assert!(parse("ctrl+").is_err());
        assert!(parse("++").is_err());
        assert!(parse("+ctrl").is_err());
        assert!(parse("   ").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn rejected() {
        for s in [
            "ctrl+alt+c",
            "shift+a",
            "alt+shift+a",
            "ctrl+alt+shift+left",
            "ctrl+ctrl+c",
            "ctrl+ctrl+left",
            "shift+shift+left",
            "ctrl+refresh",
            "shift+resize",
            "alt+focus_lost",
            "meta+c",
            "super+left",
            "ctrl+nope",
            "nope",
            "ab",
            "ctrl+ab",
            "ctrl++x",
            "+ctrl+c",
            "ctrl+c+",
            "Mouse",
            "Unknown",
            "exit",
        ] {
            assert!(parse(s).is_err(), "{s:?} should be rejected");
        }
        let err = parse("ctrl+alt+c").unwrap_err();
        assert!(err.to_string().contains("ctrl+alt+c"));
        assert_eq!(err.input(), "ctrl+alt+c");
    }

    #[test]
    fn display_round_trip() {
        let mut events = vec![Event::Refresh, Event::WindowResize, Event::FocusLost];
        for c in ['a', 'A', '+', ' ', '5', 'é'] {
            events.push(Event::Char(c));
            events.push(Event::CtrlChar(c));
            events.push(Event::AltChar(c));
        }
        for (k, _) in KEY_NAMES {
            let k = *k;
            events.extend([
                Event::Key(k),
                Event::Shift(k),
                Event::Alt(k),
                Event::AltShift(k),
                Event::Ctrl(k),
                Event::CtrlShift(k),
                Event::CtrlAlt(k),
            ]);
        }
        for e in events {
            let s = e.to_string();
            assert_eq!(parse(&s), Ok(e.clone()), "round-tripping {s:?}");
        }

        assert_eq!(Event::Char(' ').to_string(), "space");
        assert_eq!(Event::CtrlChar('c').to_string(), "ctrl+c");
        assert_eq!(Event::AltShift(Key::Up).to_string(), "alt+shift+Up");
    }

    #[test]
    fn display_unsupported() {
        let events = [
            Event::Unknown(vec![1, 2]),
            Event::Exit,
            Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::zero(),
                event: MouseEvent::WheelUp,
            },
        ];
        for e in events {
            assert!(parse(&e.to_string()).is_err());
        }
    }
}
