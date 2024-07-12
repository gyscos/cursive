//! Ncurses-specific backend.
#![cfg(feature = "ncurses-backend")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "ncurses-backend")))]
pub use ncurses;

use log::{debug, warn};
use ncurses::mmask_t;

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::io::Write;

use crate::backend;
use crate::event::{Event, Key, MouseButton, MouseEvent};
use crate::theme::{Color, ColorPair, Effect};
use crate::utf8;
use crate::Vec2;

use super::split_i32;

// Use AHash instead of the slower SipHash
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

/// Backend using ncurses.
pub struct Backend {
    current_style: Cell<ColorPair>,

    // Maps (front, back) ncurses colors to ncurses pairs
    pairs: RefCell<HashMap<(i16, i16), i16>>,

    // Pre-computed map of ncurses codes to parsed Event
    key_codes: HashMap<i32, Event>,

    // Remember the last pressed button to correctly feed Released Event
    last_mouse_button: Option<MouseButton>,

    // Sometimes a code from ncurses should be split in two Events.
    //
    // So remember the one we didn't return.
    input_buffer: Option<Event>,
}

fn find_closest_pair(pair: ColorPair) -> (i16, i16) {
    super::find_closest_pair(pair, ncurses::COLORS() as i16)
}

/// Writes some bytes directly to `/dev/tty`
///
/// Since this is not going to be used often, we can afford to re-open the
/// file every time.
fn write_to_tty(bytes: &[u8]) -> io::Result<()> {
    let mut tty_output = File::create("/dev/tty").expect("cursive can only run with a tty");
    tty_output.write_all(bytes)?;
    // tty_output will be flushed automatically at the end of the function.
    Ok(())
}

impl Backend {
    /// Creates a new ncurses-based backend.
    ///
    /// Uses `/dev/tty` for input/output.
    pub fn init() -> io::Result<Box<dyn backend::Backend>> {
        Self::init_with_files("/dev/tty", "/dev/tty")
    }

    /// Creates a new ncurses-based backend.
    ///
    /// Uses stdin/stdout for input/output.
    pub fn init_stdio() -> io::Result<Box<dyn backend::Backend>> {
        Self::init_with_files("/dev/stdin", "/dev/stdout")
    }

    /// Creates a new ncurses-based backend using the given files for input/output.
    pub fn init_with_files(
        input_path: &str,
        output_path: &str,
    ) -> io::Result<Box<dyn backend::Backend>> {
        // Check the $TERM variable.
        if std::env::var("TERM")
            .map(|var| var.is_empty())
            .unwrap_or(true)
        {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "$TERM is unset. Cannot initialize ncurses interface.",
            ));
        }

        // Change the locale.
        // For some reasons it's mandatory to get some UTF-8 support.
        let buf = CString::new("").unwrap();
        unsafe { libc::setlocale(libc::LC_ALL, buf.as_ptr()) };

        // The delay is the time ncurses wait after pressing ESC
        // to see if it's an escape sequence.
        // Default delay is way too long. 25 is imperceptible yet works fine.
        ::std::env::set_var("ESCDELAY", "25");

        // Don't output to standard IO, directly feed into /dev/tty
        // This leaves stdin and stdout usable for other purposes.
        let input = {
            let mode = CString::new("r").unwrap();
            let path = CString::new(input_path).unwrap();
            unsafe { libc::fopen(path.as_ptr(), mode.as_ptr()) }
        };
        let output = {
            let mode = CString::new("w").unwrap();
            let path = CString::new(output_path).unwrap();
            unsafe { libc::fopen(path.as_ptr(), mode.as_ptr()) }
        };
        ncurses::newterm(None, output, input).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("could not call newterm: {e}"))
        })?;

        // Enable keypad (like arrows)
        ncurses::keypad(ncurses::stdscr(), true);

        // This disables mouse click detection,
        // and provides 0-delay access to mouse presses.
        ncurses::mouseinterval(0);
        // Listen to all mouse events.
        ncurses::mousemask(
            (ncurses::ALL_MOUSE_EVENTS | ncurses::REPORT_MOUSE_POSITION) as mmask_t,
            None,
        );
        // Enable non-blocking input, so getch() immediately returns.
        ncurses::timeout(0);
        // Don't echo user input, we'll take care of that
        ncurses::noecho();
        // This disables buffering and some input processing.
        ncurses::raw();
        // This enables color support.
        ncurses::start_color();
        // Pick up background and text color from the terminal theme.
        ncurses::use_default_colors();
        // Don't print cursors.
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        // This asks the terminal to provide us with mouse drag events
        // (Mouse move when a button is pressed).
        // Replacing 1002 with 1003 would give us ANY mouse move.
        write_to_tty(b"\x1B[?1002h")?;

        let c = Backend {
            current_style: Cell::new(ColorPair::from_256colors(0, 0)),
            pairs: RefCell::new(HashMap::default()),
            key_codes: initialize_keymap(),
            last_mouse_button: None,
            input_buffer: None,
        };

        Ok(Box::new(c))
    }

    /// Save a new color pair.
    fn insert_color(&self, pairs: &mut HashMap<(i16, i16), i16>, (front, back): (i16, i16)) -> i16 {
        let n = 1 + pairs.len() as i16;

        let target = if ncurses::COLOR_PAIRS() > i32::from(n) {
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
        ncurses::init_pair(target, front, back);
        target
    }

    /// Checks the pair in the cache, or re-define a color if needed.
    fn get_or_create(&self, pair: ColorPair) -> i16 {
        let mut pairs = self.pairs.borrow_mut();

        // Find if we have this color in stock
        let result = find_closest_pair(pair);
        let lookup = pairs.get(&result).copied();
        lookup.unwrap_or_else(|| self.insert_color(&mut pairs, result))
    }

    fn set_colors(&self, pair: ColorPair) {
        let i = self.get_or_create(pair);

        self.current_style.set(pair);
        let style = ncurses::COLOR_PAIR(i);
        ncurses::attron(style);
    }

    fn parse_next(&mut self) -> Option<Event> {
        if let Some(event) = self.input_buffer.take() {
            return Some(event);
        }

        let ch: i32 = ncurses::getch();

        // Non-blocking input will return -1 as long as no input is available.
        if ch == -1 {
            return None;
        }

        // Is it a UTF-8 starting point?
        let event = if (32..=255).contains(&ch) && ch != 127 {
            utf8::read_char(ch as u8, || Some(ncurses::getch() as u8))
                .map(Event::Char)
                .unwrap_or_else(|e| {
                    warn!("Error reading input: {e}");
                    Event::Unknown(vec![ch as u8])
                })
        } else {
            self.parse_ncurses_char(ch)
        };

        Some(event)
    }

    fn parse_ncurses_char(&mut self, ch: i32) -> Event {
        // eprintln!("Found {:?}", ncurses::keyname(ch));
        if ch == ncurses::KEY_MOUSE {
            self.parse_mouse_event()
        } else {
            self.key_codes
                .get(&ch)
                .cloned()
                .unwrap_or_else(|| Event::Unknown(split_i32(ch)))
        }
    }

    fn parse_mouse_event(&mut self) -> Event {
        let mut mevent = ncurses::MEVENT {
            id: 0,
            x: 0,
            y: 0,
            z: 0,
            bstate: 0,
        };
        if ncurses::getmouse(&mut mevent as *mut ncurses::MEVENT) == ncurses::OK {
            // Currently unused
            let _ctrl = (mevent.bstate & ncurses::BUTTON_CTRL as mmask_t) != 0;
            let _shift = (mevent.bstate & ncurses::BUTTON_SHIFT as mmask_t) != 0;
            let _alt = (mevent.bstate & ncurses::BUTTON_ALT as mmask_t) != 0;

            // Keep the base state, without the modifiers
            mevent.bstate &=
                !(ncurses::BUTTON_SHIFT | ncurses::BUTTON_ALT | ncurses::BUTTON_CTRL) as mmask_t;

            // This makes a full `Event` from a `MouseEvent`.
            let make_event = |event| Event::Mouse {
                offset: Vec2::zero(),
                position: Vec2::new(mevent.x as usize, mevent.y as usize),
                event,
            };

            if mevent.bstate == ncurses::REPORT_MOUSE_POSITION as mmask_t {
                // The event is either a mouse drag event,
                // or a weird double-release event. :S

                self.last_mouse_button
                    .map(MouseEvent::Hold)
                    .or_else(|| {
                        // In legacy mode, some buttons overlap,
                        // so we need to disambiguate.
                        (mevent.bstate == ncurses::BUTTON5_DOUBLE_CLICKED as mmask_t)
                            .then_some(MouseEvent::WheelDown)
                    })
                    .map(make_event)
                    .unwrap_or_else(|| Event::Unknown(vec![]))
            } else {
                // Identify the button
                let mut bare_event = mevent.bstate & ((1 << 25) - 1);

                let mut event = None;
                // ncurses encodes multiple events in the same value.
                while bare_event != 0 {
                    let single_event = 1 << bare_event.trailing_zeros();
                    bare_event ^= single_event;

                    // Process single_event
                    on_mouse_event(single_event as i32, |e| {
                        // Keep one event for later,
                        // send the rest through the channel.
                        if event.is_none() {
                            event = Some(e);
                        } else {
                            self.input_buffer = Some(make_event(e));
                        }
                    });
                }
                if let Some(event) = event {
                    match event {
                        MouseEvent::Press(btn) => {
                            self.last_mouse_button = Some(btn);
                        }
                        MouseEvent::Release(_) => {
                            self.last_mouse_button = None;
                        }
                        _ => (),
                    }
                    make_event(event)
                } else {
                    debug!("No event parsed?...");
                    Event::Unknown(vec![])
                }
            }
        } else {
            debug!("Ncurses event not recognized.");
            Event::Unknown(vec![])
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        write_to_tty(b"\x1B[?1002l").unwrap();
        ncurses::endwin();
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "ncurses"
    }

    fn is_persistent(&self) -> bool {
        true
    }

    fn set_title(&mut self, title: String) {
        write_to_tty(format!("\x1B]0;{title}\x07").as_bytes()).unwrap();
    }

    fn screen_size(&self) -> Vec2 {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut y, &mut x);
        (x, y).into()
    }

    fn has_colors(&self) -> bool {
        ncurses::has_colors()
    }

    fn poll_event(&mut self) -> Option<Event> {
        self.parse_next()
    }

    fn set_color(&self, colors: ColorPair) -> ColorPair {
        // eprintln!("Color used: {:?}", colors);
        let current = self.current_style.get();
        if current != colors {
            self.set_colors(colors);
        }

        current
    }

    fn set_effect(&self, effect: Effect) {
        let style = match effect {
            Effect::Reverse => ncurses::A_REVERSE,
            Effect::Simple => ncurses::A_NORMAL,
            Effect::Dim => ncurses::A_DIM,
            Effect::Bold => ncurses::A_BOLD,
            Effect::Blink => ncurses::A_BLINK,
            Effect::Italic => ncurses::A_ITALIC,
            Effect::Strikethrough => ncurses::A_NORMAL,
            Effect::Underline => ncurses::A_UNDERLINE,
        };
        ncurses::attron(style);
    }

    fn unset_effect(&self, effect: Effect) {
        let style = match effect {
            Effect::Reverse => ncurses::A_REVERSE,
            Effect::Simple => ncurses::A_NORMAL,
            Effect::Dim => ncurses::A_DIM,
            Effect::Bold => ncurses::A_BOLD,
            Effect::Blink => ncurses::A_BLINK,
            Effect::Italic => ncurses::A_ITALIC,
            Effect::Strikethrough => ncurses::A_NORMAL,
            Effect::Underline => ncurses::A_UNDERLINE,
        };
        ncurses::attroff(style);
    }

    fn clear(&self, color: Color) {
        let id = self.get_or_create(ColorPair {
            front: color,
            back: color,
        });
        ncurses::wbkgd(ncurses::stdscr(), ncurses::COLOR_PAIR(id));

        ncurses::clear();
    }

    fn refresh(&mut self) {
        ncurses::refresh();
    }

    fn move_to(&self, pos: Vec2) {
        ncurses::mv(pos.y as i32, pos.x as i32);
    }

    fn print(&self, text: &str) {
        // &str is assured it doesn't contain any \0 aka nuls here due to PR 786
        // thus we can ignore the return value and avoid warning: unused `Result` that must be used
        let _ = ncurses::addstr(text);
    }
}

/// Returns the Key enum corresponding to the given ncurses event.
fn get_mouse_button(bare_event: i32) -> MouseButton {
    match bare_event {
        ncurses::BUTTON1_RELEASED
        | ncurses::BUTTON1_PRESSED
        | ncurses::BUTTON1_CLICKED
        | ncurses::BUTTON1_DOUBLE_CLICKED
        | ncurses::BUTTON1_TRIPLE_CLICKED => MouseButton::Left,
        ncurses::BUTTON2_RELEASED
        | ncurses::BUTTON2_PRESSED
        | ncurses::BUTTON2_CLICKED
        | ncurses::BUTTON2_DOUBLE_CLICKED
        | ncurses::BUTTON2_TRIPLE_CLICKED => MouseButton::Middle,
        ncurses::BUTTON3_RELEASED
        | ncurses::BUTTON3_PRESSED
        | ncurses::BUTTON3_CLICKED
        | ncurses::BUTTON3_DOUBLE_CLICKED
        | ncurses::BUTTON3_TRIPLE_CLICKED => MouseButton::Right,
        ncurses::BUTTON4_RELEASED
        | ncurses::BUTTON4_PRESSED
        | ncurses::BUTTON4_CLICKED
        | ncurses::BUTTON4_DOUBLE_CLICKED
        | ncurses::BUTTON4_TRIPLE_CLICKED => MouseButton::Button4,
        ncurses::BUTTON5_RELEASED
        | ncurses::BUTTON5_PRESSED
        | ncurses::BUTTON5_CLICKED
        | ncurses::BUTTON5_DOUBLE_CLICKED
        | ncurses::BUTTON5_TRIPLE_CLICKED => MouseButton::Button5,
        _ => MouseButton::Other,
    }
}

/// Parse the given code into one or more event.
///
/// If the given event code should expend into multiple events
/// (for instance click expends into PRESS + RELEASE),
/// the returned Vec will include those queued events.
///
/// The main event is returned separately to avoid allocation in most cases.
fn on_mouse_event<F>(bare_event: i32, mut f: F)
where
    F: FnMut(MouseEvent),
{
    let button = get_mouse_button(bare_event);
    match bare_event {
        ncurses::BUTTON1_RELEASED | ncurses::BUTTON2_RELEASED | ncurses::BUTTON3_RELEASED => {
            f(MouseEvent::Release(button))
        }
        ncurses::BUTTON1_PRESSED | ncurses::BUTTON2_PRESSED | ncurses::BUTTON3_PRESSED => {
            f(MouseEvent::Press(button))
        }
        ncurses::BUTTON4_PRESSED => f(MouseEvent::WheelUp),
        ncurses::BUTTON5_PRESSED => f(MouseEvent::WheelDown),
        // BUTTON4_RELEASED? BUTTON5_RELEASED?
        // Do they ever happen?
        _ => debug!("Unknown event: {:032b}", bare_event),
    }
}

fn add_fn<F>(start: i32, with_key: F, map: &mut HashMap<i32, Event>)
where
    F: Fn(Key) -> Event,
{
    for i in 0..12 {
        map.insert(start + i, with_key(Key::from_f((i + 1) as u8)));
    }
}

fn initialize_keymap() -> HashMap<i32, Event> {
    // First, define the static mappings.
    let mut map = HashMap::default();

    // Value sent by ncurses when nothing happens
    map.insert(-1, Event::Refresh);

    // Values under 256 are chars and control values
    // Tab is '\t'
    map.insert(9, Event::Key(Key::Tab));
    // Treat '\n' and the numpad Enter the same
    map.insert(10, Event::Key(Key::Enter));
    map.insert(ncurses::KEY_ENTER, Event::Key(Key::Enter));
    // This is the escape key when pressed by itself.
    // When used for control sequences,
    // it should have been caught earlier.
    map.insert(27, Event::Key(Key::Esc));
    // `Backspace` sends 127, but Ctrl-H sends `Backspace`
    map.insert(127, Event::Key(Key::Backspace));
    map.insert(ncurses::KEY_BACKSPACE, Event::Key(Key::Backspace));

    map.insert(410, Event::WindowResize);

    map.insert(ncurses::KEY_B2, Event::Key(Key::NumpadCenter));
    map.insert(ncurses::KEY_DC, Event::Key(Key::Del));
    map.insert(ncurses::KEY_IC, Event::Key(Key::Ins));
    map.insert(ncurses::KEY_BTAB, Event::Shift(Key::Tab));
    map.insert(ncurses::KEY_SLEFT, Event::Shift(Key::Left));
    map.insert(ncurses::KEY_SRIGHT, Event::Shift(Key::Right));
    map.insert(ncurses::KEY_LEFT, Event::Key(Key::Left));
    map.insert(ncurses::KEY_RIGHT, Event::Key(Key::Right));
    map.insert(ncurses::KEY_UP, Event::Key(Key::Up));
    map.insert(ncurses::KEY_DOWN, Event::Key(Key::Down));
    map.insert(ncurses::KEY_SR, Event::Shift(Key::Up));
    map.insert(ncurses::KEY_SF, Event::Shift(Key::Down));
    map.insert(ncurses::KEY_PPAGE, Event::Key(Key::PageUp));
    map.insert(ncurses::KEY_NPAGE, Event::Key(Key::PageDown));
    map.insert(ncurses::KEY_HOME, Event::Key(Key::Home));
    map.insert(ncurses::KEY_END, Event::Key(Key::End));
    map.insert(ncurses::KEY_SHOME, Event::Shift(Key::Home));
    map.insert(ncurses::KEY_SEND, Event::Shift(Key::End));
    map.insert(ncurses::KEY_SDC, Event::Shift(Key::Del));
    map.insert(ncurses::KEY_SNEXT, Event::Shift(Key::PageDown));
    map.insert(ncurses::KEY_SPREVIOUS, Event::Shift(Key::PageUp));

    // Then add some dynamic ones

    for c in 1..=26 {
        let event = match c {
            // Ctrl-i and Ctrl-j are special, they use the same codes as Tab
            // and Enter respectively. There's just no way to detect them. :(
            9 => Event::Key(Key::Tab),
            10 => Event::Key(Key::Enter),
            other => Event::CtrlChar((b'a' - 1 + other as u8) as char),
        };
        map.insert(c, event);
    }

    // Ncurses provides a F1 variable, but no modifiers
    add_fn(ncurses::KEY_F(1), Event::Key, &mut map);
    add_fn(277, Event::Shift, &mut map);
    add_fn(289, Event::Ctrl, &mut map);
    add_fn(301, Event::CtrlShift, &mut map);
    add_fn(313, Event::Alt, &mut map);

    // Those codes actually vary between ncurses versions...
    super::fill_key_codes(&mut map, ncurses::keyname);

    map
}
