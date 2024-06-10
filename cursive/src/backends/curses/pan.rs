//! Pancuses-specific backend.
#![cfg(feature = "pancurses-backend")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "pancurses-backend")))]

pub use pancurses;

use log::{debug, warn};

use std::cell::{Cell, RefCell};
use std::io::{stdout, Write};

use crate::backend;
use crate::event::{Event, Key, MouseButton, MouseEvent};
use crate::theme::{Color, ColorPair, Effect};
use crate::Vec2;

use super::split_i32;
use pancurses::mmask_t;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Backend using pancurses.
pub struct Backend {
    // Used
    current_style: Cell<ColorPair>,
    pairs: RefCell<HashMap<(i16, i16), i32>>,

    // pancurses needs a handle to the current window.
    window: pancurses::Window,

    key_codes: HashMap<i32, Event>,
    last_mouse_button: Option<MouseButton>,
    input_buffer: Option<Event>,
}

fn find_closest_pair(pair: ColorPair) -> (i16, i16) {
    super::find_closest_pair(pair, pancurses::COLORS() as i16)
}

impl Backend {
    /// Creates a new pancurses-based backend.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>> {
        // Check the $TERM variable (at least on unix).
        // Otherwise we'll just abort.
        // TODO: On windows, is there anything to check?
        if cfg!(unix)
            && std::env::var("TERM")
                .map(|var| var.is_empty())
                .unwrap_or(true)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "$TERM is unset. Cannot initialize pancurses interface.",
            ));
        }

        ::std::env::set_var("ESCDELAY", "25");

        if cfg!(unix) {
            let buf = std::ffi::CString::new("").unwrap();
            unsafe { libc::setlocale(libc::LC_ALL, buf.as_ptr()) };
        }

        // TODO: use pancurses::newterm()
        let window = pancurses::initscr();

        window.keypad(true);
        window.timeout(0);
        pancurses::noecho();
        pancurses::raw();
        pancurses::start_color();
        pancurses::use_default_colors();
        pancurses::curs_set(0);
        pancurses::mouseinterval(0);
        pancurses::mousemask(
            pancurses::ALL_MOUSE_EVENTS | pancurses::REPORT_MOUSE_POSITION,
            None,
        );

        // This asks the terminal to provide us with mouse drag events
        // (Mouse move when a button is pressed).
        // Replacing 1002 with 1003 would give us ANY mouse move.
        #[cfg(not(windows))]
        print!("\x1B[?1002h");
        stdout().flush()?;

        let c = Backend {
            current_style: Cell::new(ColorPair::from_256colors(0, 0)),
            pairs: RefCell::new(HashMap::default()),
            key_codes: initialize_keymap(),
            last_mouse_button: None,
            input_buffer: None,
            window,
        };

        Ok(Box::new(c))
    }

    /// Save a new color pair.
    fn insert_color(&self, pairs: &mut HashMap<(i16, i16), i32>, (front, back): (i16, i16)) -> i32 {
        let n = 1 + pairs.len() as i32;

        // TODO: when COLORS_PAIRS is available...
        let target = if 256 > n {
            // We still have plenty of space for everyone.
            n
        } else {
            // The world is too small for both of us.
            let target = n - 1;
            // Remove the mapping to n-1
            pairs.retain(|_, &mut v| v != target);
            target
        };
        pairs.insert((front, back), target);
        pancurses::init_pair(target as i16, front, back);
        target
    }

    /// Checks the pair in the cache, or re-define a color if needed.
    fn get_or_create(&self, pair: ColorPair) -> i32 {
        let mut pairs = self.pairs.borrow_mut();
        let pair = find_closest_pair(pair);

        // Find if we have this color in stock
        if pairs.contains_key(&pair) {
            // We got it!
            pairs[&pair]
        } else {
            self.insert_color(&mut pairs, pair)
        }
    }

    fn set_colors(&self, pair: ColorPair) {
        let i = self.get_or_create(pair);

        self.current_style.set(pair);
        let style = pancurses::COLOR_PAIR(i as pancurses::chtype);
        self.window.attron(style);
    }
    fn parse_next(&mut self) -> Option<Event> {
        if let Some(event) = self.input_buffer.take() {
            return Some(event);
        }

        if let Some(ev) = self.window.getch() {
            Some(match ev {
                pancurses::Input::Character('\n') | pancurses::Input::Character('\r') => {
                    Event::Key(Key::Enter)
                }
                // TODO: wait for a very short delay. If more keys are
                // pipelined, it may be an escape sequence.
                pancurses::Input::Character('\u{7f}') | pancurses::Input::Character('\u{8}') => {
                    Event::Key(Key::Backspace)
                }
                pancurses::Input::Character('\u{9}') => Event::Key(Key::Tab),
                pancurses::Input::Character('\u{1b}') => Event::Key(Key::Esc),
                // Ctrl+C
                pancurses::Input::Character(c) if (c as u32) <= 26 => {
                    Event::CtrlChar((b'a' - 1 + c as u8) as char)
                }
                pancurses::Input::Character(c) => Event::Char(c),
                // TODO: Some key combos are not recognized by pancurses,
                // but are sent as Unknown. We could still parse them here.
                pancurses::Input::Unknown(code) => self
                    .key_codes
                    // pancurses does some weird keycode mapping
                    .get(&(code + 256 + 48))
                    .cloned()
                    .unwrap_or_else(|| {
                        warn!("Unknown: {}", code);
                        Event::Unknown(split_i32(code))
                    }),
                // TODO: I honestly have no fucking idea what KeyCodeYes is
                pancurses::Input::KeyCodeYes => Event::Refresh,
                pancurses::Input::KeyBreak => Event::Key(Key::PauseBreak),
                pancurses::Input::KeyDown => Event::Key(Key::Down),
                pancurses::Input::KeyUp => Event::Key(Key::Up),
                pancurses::Input::KeyLeft => Event::Key(Key::Left),
                pancurses::Input::KeyRight => Event::Key(Key::Right),
                pancurses::Input::KeyHome => Event::Key(Key::Home),
                pancurses::Input::KeyBackspace => Event::Key(Key::Backspace),
                pancurses::Input::KeyF0 => Event::Key(Key::F0),
                pancurses::Input::KeyF1 => Event::Key(Key::F1),
                pancurses::Input::KeyF2 => Event::Key(Key::F2),
                pancurses::Input::KeyF3 => Event::Key(Key::F3),
                pancurses::Input::KeyF4 => Event::Key(Key::F4),
                pancurses::Input::KeyF5 => Event::Key(Key::F5),
                pancurses::Input::KeyF6 => Event::Key(Key::F6),
                pancurses::Input::KeyF7 => Event::Key(Key::F7),
                pancurses::Input::KeyF8 => Event::Key(Key::F8),
                pancurses::Input::KeyF9 => Event::Key(Key::F9),
                pancurses::Input::KeyF10 => Event::Key(Key::F10),
                pancurses::Input::KeyF11 => Event::Key(Key::F11),
                pancurses::Input::KeyF12 => Event::Key(Key::F12),
                pancurses::Input::KeyF13 => Event::Shift(Key::F1),
                pancurses::Input::KeyF14 => Event::Shift(Key::F2),
                pancurses::Input::KeyF15 => Event::Shift(Key::F3),
                pancurses::Input::KeyDL => Event::Refresh,
                pancurses::Input::KeyIL => Event::Refresh,
                pancurses::Input::KeyDC => Event::Key(Key::Del),
                pancurses::Input::KeyIC => Event::Key(Key::Ins),
                pancurses::Input::KeyEIC => Event::Refresh,
                pancurses::Input::KeyClear => Event::Refresh,
                pancurses::Input::KeyEOS => Event::Refresh,
                pancurses::Input::KeyEOL => Event::Refresh,
                pancurses::Input::KeySF => Event::Shift(Key::Down),
                pancurses::Input::KeySR => Event::Shift(Key::Up),
                pancurses::Input::KeyNPage => Event::Key(Key::PageDown),
                pancurses::Input::KeyPPage => Event::Key(Key::PageUp),
                pancurses::Input::KeySTab => Event::Shift(Key::Tab),
                pancurses::Input::KeyCTab => Event::Ctrl(Key::Tab),
                pancurses::Input::KeyCATab => Event::CtrlAlt(Key::Tab),
                pancurses::Input::KeyEnter => Event::Key(Key::Enter),
                pancurses::Input::KeySReset => Event::Refresh,
                pancurses::Input::KeyReset => Event::Refresh,
                pancurses::Input::KeyPrint => Event::Refresh,
                pancurses::Input::KeyLL => Event::Refresh,
                pancurses::Input::KeyAbort => Event::Refresh,
                pancurses::Input::KeySHelp => Event::Refresh,
                pancurses::Input::KeyLHelp => Event::Refresh,
                pancurses::Input::KeyBTab => Event::Shift(Key::Tab),
                pancurses::Input::KeyBeg => Event::Refresh,
                pancurses::Input::KeyCancel => Event::Refresh,
                pancurses::Input::KeyClose => Event::Refresh,
                pancurses::Input::KeyCommand => Event::Refresh,
                pancurses::Input::KeyCopy => Event::Refresh,
                pancurses::Input::KeyCreate => Event::Refresh,
                pancurses::Input::KeyEnd => Event::Key(Key::End),
                pancurses::Input::KeyExit => Event::Refresh,
                pancurses::Input::KeyFind => Event::Refresh,
                pancurses::Input::KeyHelp => Event::Refresh,
                pancurses::Input::KeyMark => Event::Refresh,
                pancurses::Input::KeyMessage => Event::Refresh,
                pancurses::Input::KeyMove => Event::Refresh,
                pancurses::Input::KeyNext => Event::Refresh,
                pancurses::Input::KeyOpen => Event::Refresh,
                pancurses::Input::KeyOptions => Event::Refresh,
                pancurses::Input::KeyPrevious => Event::Refresh,
                pancurses::Input::KeyRedo => Event::Refresh,
                pancurses::Input::KeyReference => Event::Refresh,
                pancurses::Input::KeyRefresh => Event::Refresh,
                pancurses::Input::KeyReplace => Event::Refresh,
                pancurses::Input::KeyRestart => Event::Refresh,
                pancurses::Input::KeyResume => Event::Refresh,
                pancurses::Input::KeySave => Event::Refresh,
                pancurses::Input::KeySBeg => Event::Refresh,
                pancurses::Input::KeySCancel => Event::Refresh,
                pancurses::Input::KeySCommand => Event::Refresh,
                pancurses::Input::KeySCopy => Event::Refresh,
                pancurses::Input::KeySCreate => Event::Refresh,
                pancurses::Input::KeySDC => Event::Shift(Key::Del),
                pancurses::Input::KeySDL => Event::Refresh,
                pancurses::Input::KeySelect => Event::Refresh,
                pancurses::Input::KeySEnd => Event::Shift(Key::End),
                pancurses::Input::KeySEOL => Event::Refresh,
                pancurses::Input::KeySExit => Event::Refresh,
                pancurses::Input::KeySFind => Event::Refresh,
                pancurses::Input::KeySHome => Event::Shift(Key::Home),
                pancurses::Input::KeySIC => Event::Shift(Key::Ins),
                pancurses::Input::KeySLeft => Event::Shift(Key::Left),
                pancurses::Input::KeySMessage => Event::Refresh,
                pancurses::Input::KeySMove => Event::Refresh,
                pancurses::Input::KeySNext => Event::Shift(Key::PageDown),
                pancurses::Input::KeySOptions => Event::Refresh,
                pancurses::Input::KeySPrevious => Event::Shift(Key::PageUp),
                pancurses::Input::KeySPrint => Event::Refresh,
                pancurses::Input::KeySRedo => Event::Refresh,
                pancurses::Input::KeySReplace => Event::Refresh,
                pancurses::Input::KeySRight => Event::Shift(Key::Right),
                pancurses::Input::KeySResume => Event::Refresh,
                pancurses::Input::KeySSave => Event::Refresh,
                pancurses::Input::KeySSuspend => Event::Refresh,
                pancurses::Input::KeySUndo => Event::Refresh,
                pancurses::Input::KeySuspend => Event::Refresh,
                pancurses::Input::KeyUndo => Event::Refresh,
                pancurses::Input::KeyResize => {
                    // Let pancurses adjust their structures when the
                    // window is resized.
                    // Do it for Windows only, as 'resize_term' is not
                    // implemented for Unix
                    if cfg!(target_os = "windows") {
                        pancurses::resize_term(0, 0);
                    }
                    Event::WindowResize
                }
                pancurses::Input::KeyEvent => Event::Refresh,
                // TODO: mouse support
                pancurses::Input::KeyMouse => self.parse_mouse_event(),
                pancurses::Input::KeyA1 => Event::Refresh,
                pancurses::Input::KeyA3 => Event::Refresh,
                pancurses::Input::KeyB2 => Event::Key(Key::NumpadCenter),
                pancurses::Input::KeyC1 => Event::Refresh,
                pancurses::Input::KeyC3 => Event::Refresh,
            })
        } else {
            None
        }
    }

    fn parse_mouse_event(&mut self) -> Event {
        let mut mevent = match pancurses::getmouse() {
            Err(code) => return Event::Unknown(split_i32(code)),
            Ok(event) => event,
        };

        let _shift = (mevent.bstate & pancurses::BUTTON_SHIFT as mmask_t) != 0;
        let _alt = (mevent.bstate & pancurses::BUTTON_ALT as mmask_t) != 0;
        let _ctrl = (mevent.bstate & pancurses::BUTTON_CTRL as mmask_t) != 0;

        mevent.bstate &=
            !(pancurses::BUTTON_SHIFT | pancurses::BUTTON_ALT | pancurses::BUTTON_CTRL) as mmask_t;

        let make_event = |event| Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(mevent.x as usize, mevent.y as usize),
            event,
        };

        if mevent.bstate == pancurses::REPORT_MOUSE_POSITION as mmask_t {
            // The event is either a mouse drag event,
            // or a weird double-release event. :S
            self.last_mouse_button
                .map(MouseEvent::Hold)
                .or_else(|| {
                    // In legacy mode, some buttons overlap,
                    // so we need to disambiguate.
                    (mevent.bstate == pancurses::BUTTON5_DOUBLE_CLICKED as mmask_t)
                        .then_some(MouseEvent::WheelDown)
                })
                .map(make_event)
                .unwrap_or_else(|| {
                    debug!("We got a mouse drag, but no last mouse pressed?");
                    Event::Unknown(vec![])
                })
        } else {
            // Identify the button
            let mut bare_event = mevent.bstate & ((1 << 25) - 1);

            let mut event = None;
            while bare_event != 0 {
                let single_event = 1 << bare_event.trailing_zeros();
                bare_event ^= single_event;

                // Process single_event
                on_mouse_event(single_event, |e| {
                    if event.is_none() {
                        event = Some(e);
                    } else {
                        self.input_buffer = Some(make_event(e));
                    }
                });
            }
            if let Some(event) = event {
                if let Some(btn) = event.button() {
                    self.last_mouse_button = Some(btn);
                }
                make_event(event)
            } else {
                debug!("No event parsed?...");
                Event::Unknown(vec![])
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        print!("\x1B[?1002l");
        stdout().flush().expect("could not flush stdout");
        pancurses::endwin();
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "pancurses"
    }

    fn is_persistent(&self) -> bool {
        true
    }

    fn set_title(&mut self, title: String) {
        print!("\x1B]0;{title}\x07");
        stdout().flush().expect("could not flush stdout");
    }

    fn screen_size(&self) -> Vec2 {
        // Coordinates are reversed here
        let (y, x) = self.window.get_max_yx();
        (x, y).into()
    }

    fn has_colors(&self) -> bool {
        pancurses::has_colors()
    }

    fn set_color(&self, colors: ColorPair) -> ColorPair {
        let current = self.current_style.get();

        if current != colors {
            self.set_colors(colors);
        }

        current
    }

    fn set_effect(&self, effect: Effect) {
        let style = match effect {
            Effect::Simple => pancurses::Attribute::Normal,
            Effect::Reverse => pancurses::Attribute::Reverse,
            Effect::Dim => pancurses::Attribute::Dim,
            Effect::Bold => pancurses::Attribute::Bold,
            Effect::Blink => pancurses::Attribute::Blink,
            Effect::Italic => pancurses::Attribute::Italic,
            Effect::Strikethrough => pancurses::Attribute::Strikeout,
            Effect::Underline => pancurses::Attribute::Underline,
        };
        self.window.attron(style);
    }

    fn unset_effect(&self, effect: Effect) {
        let style = match effect {
            Effect::Simple => pancurses::Attribute::Normal,
            Effect::Reverse => pancurses::Attribute::Reverse,
            Effect::Dim => pancurses::Attribute::Dim,
            Effect::Bold => pancurses::Attribute::Bold,
            Effect::Blink => pancurses::Attribute::Blink,
            Effect::Italic => pancurses::Attribute::Italic,
            Effect::Strikethrough => pancurses::Attribute::Strikeout,
            Effect::Underline => pancurses::Attribute::Underline,
        };
        self.window.attroff(style);
    }

    fn clear(&self, color: Color) {
        let id = self.get_or_create(ColorPair {
            front: color,
            back: color,
        });
        self.window.bkgd(pancurses::ColorPair(id as u8));
        self.window.clear();
    }

    fn refresh(&mut self) {
        self.window.refresh();
    }

    fn move_to(&self, pos: Vec2) {
        self.window.mv(pos.y as i32, pos.x as i32);
    }

    fn print(&self, text: &str) {
        self.window.addstr(text);
    }

    fn poll_event(&mut self) -> Option<Event> {
        self.parse_next()
    }
}

/// Parse the given code into one or more event.
///
/// If the given event code should expend into multiple events
/// (for instance click expends into PRESS + RELEASE),
/// the returned Vec will include those queued events.
///
/// The main event is returned separately to avoid allocation in most cases.
fn on_mouse_event<F>(bare_event: mmask_t, mut f: F)
where
    F: FnMut(MouseEvent),
{
    let button = get_mouse_button(bare_event);
    match bare_event {
        pancurses::BUTTON4_PRESSED => f(MouseEvent::WheelUp),
        pancurses::BUTTON5_PRESSED => f(MouseEvent::WheelDown),
        pancurses::BUTTON1_RELEASED
        | pancurses::BUTTON2_RELEASED
        | pancurses::BUTTON3_RELEASED
        | pancurses::BUTTON4_RELEASED
        | pancurses::BUTTON5_RELEASED => f(MouseEvent::Release(button)),
        pancurses::BUTTON1_PRESSED | pancurses::BUTTON2_PRESSED | pancurses::BUTTON3_PRESSED => {
            f(MouseEvent::Press(button))
        }
        pancurses::BUTTON1_CLICKED
        | pancurses::BUTTON2_CLICKED
        | pancurses::BUTTON3_CLICKED
        | pancurses::BUTTON4_CLICKED
        | pancurses::BUTTON5_CLICKED => {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        }
        // Well, we disabled click detection
        pancurses::BUTTON1_DOUBLE_CLICKED
        | pancurses::BUTTON2_DOUBLE_CLICKED
        | pancurses::BUTTON3_DOUBLE_CLICKED
        | pancurses::BUTTON4_DOUBLE_CLICKED
        | pancurses::BUTTON5_DOUBLE_CLICKED => {
            for _ in 0..2 {
                f(MouseEvent::Press(button));
                f(MouseEvent::Release(button));
            }
        }
        pancurses::BUTTON1_TRIPLE_CLICKED
        | pancurses::BUTTON2_TRIPLE_CLICKED
        | pancurses::BUTTON3_TRIPLE_CLICKED
        | pancurses::BUTTON4_TRIPLE_CLICKED
        | pancurses::BUTTON5_TRIPLE_CLICKED => {
            for _ in 0..3 {
                f(MouseEvent::Press(button));
                f(MouseEvent::Release(button));
            }
        }
        _ => debug!("Unknown event: {:032b}", bare_event),
    }
}

/// Returns the Key enum corresponding to the given pancurses event.
fn get_mouse_button(bare_event: mmask_t) -> MouseButton {
    match bare_event {
        pancurses::BUTTON1_RELEASED
        | pancurses::BUTTON1_PRESSED
        | pancurses::BUTTON1_CLICKED
        | pancurses::BUTTON1_DOUBLE_CLICKED
        | pancurses::BUTTON1_TRIPLE_CLICKED => MouseButton::Left,
        pancurses::BUTTON2_RELEASED
        | pancurses::BUTTON2_PRESSED
        | pancurses::BUTTON2_CLICKED
        | pancurses::BUTTON2_DOUBLE_CLICKED
        | pancurses::BUTTON2_TRIPLE_CLICKED => MouseButton::Middle,
        pancurses::BUTTON3_RELEASED
        | pancurses::BUTTON3_PRESSED
        | pancurses::BUTTON3_CLICKED
        | pancurses::BUTTON3_DOUBLE_CLICKED
        | pancurses::BUTTON3_TRIPLE_CLICKED => MouseButton::Right,
        pancurses::BUTTON4_RELEASED
        | pancurses::BUTTON4_PRESSED
        | pancurses::BUTTON4_CLICKED
        | pancurses::BUTTON4_DOUBLE_CLICKED
        | pancurses::BUTTON4_TRIPLE_CLICKED => MouseButton::Button4,
        pancurses::BUTTON5_RELEASED
        | pancurses::BUTTON5_PRESSED
        | pancurses::BUTTON5_CLICKED
        | pancurses::BUTTON5_DOUBLE_CLICKED
        | pancurses::BUTTON5_TRIPLE_CLICKED => MouseButton::Button5,
        _ => MouseButton::Other,
    }
}

fn initialize_keymap() -> HashMap<i32, Event> {
    let mut map = HashMap::default();

    super::fill_key_codes(&mut map, pancurses::keyname);

    map
}
