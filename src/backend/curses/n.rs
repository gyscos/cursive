extern crate ncurses;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use libc;
use signal_hook::iterator::Signals;

use backend;
use event::{Event, Key, MouseButton, MouseEvent};
use theme::{Color, ColorPair, Effect};
use utf8;
use vec::Vec2;

use self::super::split_i32;
use self::ncurses::mmask_t;

pub struct Backend {
    current_style: Cell<ColorPair>,

    // Maps (front, back) ncurses colors to ncurses pairs
    pairs: RefCell<HashMap<(i16, i16), i16>>,

    // This is set by the SIGWINCH-triggered thread.
    // When TRUE, we should tell ncurses about the new terminal size.
    needs_resize: Arc<AtomicBool>,

    // The signal hook to receive SIGWINCH (window resize)
    signals: Option<Signals>,
}

struct InputParser {
    key_codes: HashMap<i32, Event>,
    last_mouse_button: Option<MouseButton>,
    event_sink: Sender<Option<Event>>,
}

impl InputParser {
    fn new(event_sink: Sender<Option<Event>>) -> Self {
        InputParser {
            key_codes: initialize_keymap(),
            last_mouse_button: None,
            event_sink,
        }
    }

    fn parse_next(&mut self) {
        let ch: i32 = ncurses::getch();

        if ch == 410 {
            // Ignore resize events.
            self.parse_next();
            return;
        }

        if ch == -1 {
            self.event_sink.send(None);
            return;
        }

        // Is it a UTF-8 starting point?
        let event = if 32 <= ch && ch <= 255 && ch != 127 {
            utf8::read_char(ch as u8, || Some(ncurses::getch() as u8))
                .map(Event::Char)
                .unwrap_or_else(|e| {
                    warn!("Error reading input: {}", e);
                    Event::Unknown(vec![ch as u8])
                })
        } else {
            self.parse_ncurses_char(ch)
        };
        self.event_sink.send(Some(event));
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
        if ncurses::getmouse(&mut mevent as *mut ncurses::MEVENT)
            == ncurses::OK
        {
            // Currently unused
            let _ctrl = (mevent.bstate & ncurses::BUTTON_CTRL as mmask_t) != 0;
            let _shift =
                (mevent.bstate & ncurses::BUTTON_SHIFT as mmask_t) != 0;
            let _alt = (mevent.bstate & ncurses::BUTTON_ALT as mmask_t) != 0;

            // Keep the base state, without the modifiers
            mevent.bstate &= !(ncurses::BUTTON_SHIFT
                | ncurses::BUTTON_ALT
                | ncurses::BUTTON_CTRL)
                as mmask_t;

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
                    .map(&make_event)
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
                            self.event_sink.send(Some(make_event(e)));
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
        } else {
            debug!("Ncurses event not recognized.");
            Event::Unknown(vec![])
        }
    }
}

fn find_closest_pair(pair: ColorPair) -> (i16, i16) {
    super::find_closest_pair(pair, ncurses::COLORS() as i16)
}

/// Writes some bytes directly to `/dev/tty`
///
/// Since this is not going to be used often, we can afford to re-open the
/// file every time.
fn write_to_tty(bytes: &[u8]) -> io::Result<()> {
    let mut tty_output =
        File::create("/dev/tty").expect("cursive can only run with a tty");
    tty_output.write_all(bytes)?;
    // tty_output will be flushed automatically at the end of the function.
    Ok(())
}

impl Backend {
    pub fn init() -> Box<backend::Backend> {

        let signals = Some(Signals::new(&[libc::SIGWINCH]).unwrap());

        // Change the locale.
        // For some reasons it's mandatory to get some UTF-8 support.
        ncurses::setlocale(ncurses::LcCategory::all, "");

        // The delay is the time ncurses wait after pressing ESC
        // to see if it's an escape sequence.
        // Default delay is way too long. 25 is imperceptible yet works fine.
        ::std::env::set_var("ESCDELAY", "25");

        let tty_path = CString::new("/dev/tty").unwrap();
        let mode = CString::new("r+").unwrap();
        let tty = unsafe { libc::fopen(tty_path.as_ptr(), mode.as_ptr()) };
        ncurses::newterm(None, tty, tty);
        ncurses::keypad(ncurses::stdscr(), true);

        // This disables mouse click detection,
        // and provides 0-delay access to mouse presses.
        ncurses::mouseinterval(0);
        // Listen to all mouse events.
        ncurses::mousemask(
            (ncurses::ALL_MOUSE_EVENTS | ncurses::REPORT_MOUSE_POSITION)
                as mmask_t,
            None,
        );
        ncurses::noecho();
        ncurses::cbreak();
        ncurses::start_color();
        // Pick up background and text color from the terminal theme.
        ncurses::use_default_colors();
        // No cursor
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

        // This asks the terminal to provide us with mouse drag events
        // (Mouse move when a button is pressed).
        // Replacing 1002 with 1003 would give us ANY mouse move.
        write_to_tty(b"\x1B[?1002h").unwrap();

        let c = Backend {
            current_style: Cell::new(ColorPair::from_256colors(0, 0)),
            pairs: RefCell::new(HashMap::new()),
            needs_resize: Arc::new(AtomicBool::new(false)),
            signals,
        };

        Box::new(c)
    }

    /// Save a new color pair.
    fn insert_color(
        &self, pairs: &mut HashMap<(i16, i16), i16>, (front, back): (i16, i16),
    ) -> i16 {
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
        let (front, back) = find_closest_pair(pair);
        if pairs.contains_key(&(front, back)) {
            // We got it!
            pairs[&(front, back)]
        } else {
            self.insert_color(&mut *pairs, (front, back))
        }
    }

    fn set_colors(&self, pair: ColorPair) {
        let i = self.get_or_create(pair);

        self.current_style.set(pair);
        let style = ncurses::COLOR_PAIR(i);
        ncurses::attron(style);
    }
}

/// Called when a resize event is detected.
///
/// We need to have ncurses update its representation of the screen.
fn on_resize() {
    // Get size
    let size = super::terminal_size();

    // Send the size to ncurses
    ncurses::resize_term(size.y as i32, size.x as i32);
}

impl backend::Backend for Backend {
    fn screen_size(&self) -> Vec2 {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut y, &mut x);
        (x, y).into()
    }

    fn has_colors(&self) -> bool {
        ncurses::has_colors()
    }

    fn start_input_thread(
        &mut self, event_sink: Sender<Option<Event>>,
        input_request: Receiver<backend::InputRequest>,
    ) {
        let running = Arc::new(AtomicBool::new(true));
        let needs_resize = Arc::clone(&self.needs_resize);

        let resize_running = Arc::clone(&running);
        let resize_sender = event_sink.clone();
        let signals = self.signals.take().unwrap();

        thread::spawn(move || {
            // This thread will listen to SIGWINCH events and report them.
            while resize_running.load(Ordering::Relaxed) {
                // We know it will only contain SIGWINCH signals, so no need to check.
                for _ in signals.pending() {
                    // Tell ncurses about the new terminal size.
                    // Well, do the actual resizing later on, in the main thread.
                    // Ncurses isn't really thread-safe so calling resize_term() can crash
                    // other calls like clear() or refresh().
                    needs_resize.store(true, Ordering::Relaxed);
                    resize_sender.send(Some(Event::WindowResize));
                }
            }
        });

        let mut parser = InputParser::new(event_sink);

        // This thread will take input from ncurses for each request.
        thread::spawn(move || {
            for req in input_request {
                match req {
                    backend::InputRequest::Peek => {
                        ncurses::timeout(0);
                    }
                    backend::InputRequest::Block => {
                        ncurses::timeout(-1);
                    }
                }
                parser.parse_next();
            }
            running.store(false, Ordering::Relaxed);
        });
    }

    fn finish(&mut self) {
        write_to_tty(b"\x1B[?1002l").unwrap();
        ncurses::endwin();
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
            Effect::Reverse => ncurses::A_REVERSE(),
            Effect::Simple => ncurses::A_NORMAL(),
            Effect::Bold => ncurses::A_BOLD(),
            Effect::Italic => ncurses::A_ITALIC(),
            Effect::Underline => ncurses::A_UNDERLINE(),
        };
        ncurses::attron(style);
    }

    fn unset_effect(&self, effect: Effect) {
        let style = match effect {
            Effect::Reverse => ncurses::A_REVERSE(),
            Effect::Simple => ncurses::A_NORMAL(),
            Effect::Bold => ncurses::A_BOLD(),
            Effect::Italic => ncurses::A_ITALIC(),
            Effect::Underline => ncurses::A_UNDERLINE(),
        };
        ncurses::attroff(style);
    }

    fn clear(&self, color: Color) {
        if self.needs_resize.swap(false, Ordering::Relaxed) {
            on_resize();
        }

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

    fn print_at(&self, pos: Vec2, text: &str) {
        ncurses::mvaddstr(pos.y as i32, pos.x as i32, text);
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
        ncurses::BUTTON4_PRESSED => f(MouseEvent::WheelUp),
        ncurses::BUTTON5_PRESSED => f(MouseEvent::WheelDown),
        ncurses::BUTTON1_RELEASED
        | ncurses::BUTTON2_RELEASED
        | ncurses::BUTTON3_RELEASED
        | ncurses::BUTTON4_RELEASED
        | ncurses::BUTTON5_RELEASED => f(MouseEvent::Release(button)),
        ncurses::BUTTON1_PRESSED
        | ncurses::BUTTON2_PRESSED
        | ncurses::BUTTON3_PRESSED => f(MouseEvent::Press(button)),
        ncurses::BUTTON1_CLICKED
        | ncurses::BUTTON2_CLICKED
        | ncurses::BUTTON3_CLICKED
        | ncurses::BUTTON4_CLICKED
        | ncurses::BUTTON5_CLICKED => {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        }
        // Well, we disabled click detection
        ncurses::BUTTON1_DOUBLE_CLICKED
        | ncurses::BUTTON2_DOUBLE_CLICKED
        | ncurses::BUTTON3_DOUBLE_CLICKED
        | ncurses::BUTTON4_DOUBLE_CLICKED
        | ncurses::BUTTON5_DOUBLE_CLICKED => for _ in 0..2 {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        },
        ncurses::BUTTON1_TRIPLE_CLICKED
        | ncurses::BUTTON2_TRIPLE_CLICKED
        | ncurses::BUTTON3_TRIPLE_CLICKED
        | ncurses::BUTTON4_TRIPLE_CLICKED
        | ncurses::BUTTON5_TRIPLE_CLICKED => for _ in 0..3 {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        },
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
    let mut map = hashmap!{

        // Value sent by ncurses when nothing happens
        -1 => Event::Refresh,

        // Values under 256 are chars and control values
        //
        // Tab is '\t'
        9 => Event::Key(Key::Tab),
        // Treat '\n' and the numpad Enter the same
        10 => Event::Key(Key::Enter),
        ncurses::KEY_ENTER => Event::Key(Key::Enter),
        // This is the escape key when pressed by itself.
        // When used for control sequences,
        // it should have been caught earlier.
        27 => Event::Key(Key::Esc),
        // `Backspace` sends 127, but Ctrl-H sends `Backspace`
        127 => Event::Key(Key::Backspace),
        ncurses::KEY_BACKSPACE => Event::Key(Key::Backspace),

        410 => Event::WindowResize,

        ncurses::KEY_B2 => Event::Key(Key::NumpadCenter),
        ncurses::KEY_DC => Event::Key(Key::Del),
        ncurses::KEY_IC => Event::Key(Key::Ins),
        ncurses::KEY_BTAB => Event::Shift(Key::Tab),
        ncurses::KEY_SLEFT => Event::Shift(Key::Left),
        ncurses::KEY_SRIGHT => Event::Shift(Key::Right),
        ncurses::KEY_LEFT => Event::Key(Key::Left),
        ncurses::KEY_RIGHT => Event::Key(Key::Right),
        ncurses::KEY_UP => Event::Key(Key::Up),
        ncurses::KEY_DOWN => Event::Key(Key::Down),
        ncurses::KEY_SR => Event::Shift(Key::Up),
        ncurses::KEY_SF => Event::Shift(Key::Down),
        ncurses::KEY_PPAGE => Event::Key(Key::PageUp),
        ncurses::KEY_NPAGE => Event::Key(Key::PageDown),
        ncurses::KEY_HOME => Event::Key(Key::Home),
        ncurses::KEY_END => Event::Key(Key::End),
        ncurses::KEY_SHOME => Event::Shift(Key::Home),
        ncurses::KEY_SEND => Event::Shift(Key::End),
        ncurses::KEY_SDC => Event::Shift(Key::Del),
        ncurses::KEY_SNEXT => Event::Shift(Key::PageDown),
        ncurses::KEY_SPREVIOUS => Event::Shift(Key::PageUp),
    };

    // Then add some dynamic ones

    for c in 1..26 {
        let event = match c {
            9 => Event::Key(Key::Tab),
            10 => Event::Key(Key::Enter),
            other => Event::CtrlChar((b'a' - 1 + other as u8) as char),
        };
        map.insert(c, event);
    }

    // Ncurses provides a F1 variable, but no modifiers
    add_fn(ncurses::KEY_F1, Event::Key, &mut map);
    add_fn(277, Event::Shift, &mut map);
    add_fn(289, Event::Ctrl, &mut map);
    add_fn(301, Event::CtrlShift, &mut map);
    add_fn(313, Event::Alt, &mut map);

    // Those codes actually vary between ncurses versions...
    super::fill_key_codes(&mut map, ncurses::keyname);

    map
}
