extern crate pancurses;

use self::pancurses::mmask_t;
use self::super::{find_closest, split_i32};
use backend;
use event::{Event, Key, MouseButton, MouseEvent};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use theme::{Color, ColorPair, Effect};
use utf8;
use vec::Vec2;

pub struct Concrete {
    current_style: Cell<ColorPair>,
    pairs: RefCell<HashMap<ColorPair, i32>>,
    window: pancurses::Window,

    last_mouse_button: Option<MouseButton>,
    event_queue: Vec<Event>,
}

impl Concrete {
    /// Save a new color pair.
    fn insert_color(
        &self, pairs: &mut HashMap<ColorPair, i32>, pair: ColorPair
    ) -> i32 {
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
        pairs.insert(pair, target);
        pancurses::init_pair(
            target as i16,
            find_closest(&pair.front),
            find_closest(&pair.back),
        );
        target
    }

    /// Checks the pair in the cache, or re-define a color if needed.
    fn get_or_create(&self, pair: ColorPair) -> i32 {
        let mut pairs = self.pairs.borrow_mut();

        // Find if we have this color in stock
        if pairs.contains_key(&pair) {
            // We got it!
            pairs[&pair]
        } else {
            self.insert_color(&mut *pairs, pair)
        }
    }

    fn set_colors(&self, pair: ColorPair) {
        let i = self.get_or_create(pair);

        self.current_style.set(pair);
        let style = pancurses::COLOR_PAIR(i as pancurses::chtype);
        self.window.attron(style);
    }

    fn parse_mouse_event(&mut self) -> Event {
        let mut mevent = match pancurses::getmouse() {
            Err(code) => return Event::Unknown(split_i32(code)),
            Ok(event) => event,
        };

        let _shift = (mevent.bstate & pancurses::BUTTON_SHIFT as mmask_t) != 0;
        let _alt = (mevent.bstate & pancurses::BUTTON_ALT as mmask_t) != 0;
        let _ctrl = (mevent.bstate & pancurses::BUTTON_CTRL as mmask_t) != 0;

        mevent.bstate &= !(pancurses::BUTTON_SHIFT | pancurses::BUTTON_ALT
            | pancurses::BUTTON_CTRL) as mmask_t;

        let make_event = |event| Event::Mouse {
            offset: Vec2::zero(),
            position: Vec2::new(mevent.x as usize, mevent.y as usize),
            event: event,
        };

        if mevent.bstate == pancurses::REPORT_MOUSE_POSITION as mmask_t {
            // The event is either a mouse drag event,
            // or a weird double-release event. :S
            self.last_mouse_button
                .map(MouseEvent::Hold)
                .map(&make_event)
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
                        self.event_queue.push(make_event(e));
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

impl backend::Backend for Concrete {
    fn init() -> Self {
        ::std::env::set_var("ESCDELAY", "25");

        let window = pancurses::initscr();
        window.keypad(true);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
        pancurses::use_default_colors();
        pancurses::curs_set(0);
        pancurses::mouseinterval(0);
        pancurses::mousemask(
            pancurses::ALL_MOUSE_EVENTS | pancurses::REPORT_MOUSE_POSITION,
            ::std::ptr::null_mut(),
        );

        // This asks the terminal to provide us with mouse drag events
        // (Mouse move when a button is pressed).
        // Replacing 1002 with 1003 would give us ANY mouse move.
        println!("\x1B[?1002h");

        Concrete {
            current_style: Cell::new(ColorPair::from_256colors(0, 0)),
            pairs: RefCell::new(HashMap::new()),
            window: window,
            last_mouse_button: None,
            event_queue: Vec::new(),
        }
    }

    fn screen_size(&self) -> (usize, usize) {
        let (y, x) = self.window.get_max_yx();
        (x as usize, y as usize)
    }

    fn has_colors(&self) -> bool {
        pancurses::has_colors()
    }

    fn finish(&mut self) {
        pancurses::endwin();
    }

    fn with_color<F: FnOnce()>(&self, colors: ColorPair, f: F) {
        let current = self.current_style.get();

        if current != colors {
            self.set_colors(colors);
        }

        f();

        if current != colors {
            self.set_colors(current);
        }
    }

    fn with_effect<F: FnOnce()>(&self, effect: Effect, f: F) {
        let style = match effect {
            Effect::Simple => pancurses::Attribute::Normal,
            Effect::Reverse => pancurses::Attribute::Reverse,
            Effect::Bold => pancurses::Attribute::Bold,
            Effect::Italic => pancurses::Attribute::Italic,
            Effect::Underline => pancurses::Attribute::Underline,
        };
        self.window.attron(style);
        f();
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

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        self.window.mvaddstr(y as i32, x as i32, text);
    }

    fn poll_event(&mut self) -> Event {
        self.event_queue.pop().unwrap_or_else(|| {
            if let Some(ev) = self.window.getch() {
                match ev {
                    pancurses::Input::Character('\n') => {
                        Event::Key(Key::Enter)
                    }
                    // TODO: wait for a very short delay. If more keys are
                    // pipelined, it may be an escape sequence.
                    pancurses::Input::Character('\u{7f}')
                    | pancurses::Input::Character('\u{8}') => {
                        Event::Key(Key::Backspace)
                    }
                    pancurses::Input::Character('\u{9}') => {
                        Event::Key(Key::Tab)
                    }
                    pancurses::Input::Character('\u{1b}') => {
                        Event::Key(Key::Esc)
                    }
                    pancurses::Input::Character(c)
                        if 32 <= (c as u32) && (c as u32) <= 255 =>
                    {
                        Event::Char(
                            utf8::read_char(c as u8, || {
                                self.window.getch().and_then(|i| match i {
                                    pancurses::Input::Character(c) => {
                                        Some(c as u8)
                                    }
                                    _ => None,
                                })
                            }).unwrap(),
                        )
                    }
                    pancurses::Input::Character(c) if (c as u32) <= 26 => {
                        Event::CtrlChar((b'a' - 1 + c as u8) as char)
                    }
                    pancurses::Input::Character(c) => {
                        let mut bytes = [0u8; 4];
                        Event::Unknown(
                            c.encode_utf8(&mut bytes).as_bytes().to_vec(),
                        )
                    }
                    // TODO: Some key combos are not recognized by pancurses,
                    // but are sent as Unknown. We could still parse them here.
                    pancurses::Input::Unknown(code) => match code {
                        220 => Event::Ctrl(Key::Del),

                        224 => Event::Alt(Key::Down),
                        225 => Event::AltShift(Key::Down),
                        226 => Event::Ctrl(Key::Down),
                        227 => Event::CtrlShift(Key::Down),

                        229 => Event::Alt(Key::End),
                        230 => Event::AltShift(Key::End),
                        231 => Event::Ctrl(Key::End),
                        232 => Event::CtrlShift(Key::End),

                        235 => Event::Alt(Key::Home),
                        236 => Event::AltShift(Key::Home),
                        237 => Event::Ctrl(Key::Home),
                        238 => Event::CtrlShift(Key::Home),

                        246 => Event::Alt(Key::Left),
                        247 => Event::AltShift(Key::Left),
                        248 => Event::Ctrl(Key::Left),
                        249 => Event::CtrlShift(Key::Left),

                        251 => Event::Alt(Key::PageDown),
                        252 => Event::AltShift(Key::PageDown),
                        253 => Event::Ctrl(Key::PageDown),
                        254 => Event::CtrlShift(Key::PageDown),

                        256 => Event::Alt(Key::PageUp),
                        257 => Event::AltShift(Key::PageUp),
                        258 => Event::Ctrl(Key::PageUp),
                        259 => Event::CtrlShift(Key::PageUp),

                        261 => Event::Alt(Key::Right),
                        262 => Event::AltShift(Key::Right),
                        263 => Event::Ctrl(Key::Right),
                        264 => Event::CtrlShift(Key::Right),

                        267 => Event::Alt(Key::Up),
                        268 => Event::AltShift(Key::Up),
                        269 => Event::Ctrl(Key::Up),
                        270 => Event::CtrlShift(Key::Up),
                        other => {
                            warn!("Unknown: {}", other);
                            Event::Unknown(split_i32(other))
                        }
                    },
                    // TODO: I honestly have no fucking idea what KeyCodeYes is
                    pancurses::Input::KeyCodeYes => Event::Refresh,
                    pancurses::Input::KeyBreak => Event::Key(Key::PauseBreak),
                    pancurses::Input::KeyDown => Event::Key(Key::Down),
                    pancurses::Input::KeyUp => Event::Key(Key::Up),
                    pancurses::Input::KeyLeft => Event::Key(Key::Left),
                    pancurses::Input::KeyRight => Event::Key(Key::Right),
                    pancurses::Input::KeyHome => Event::Key(Key::Home),
                    pancurses::Input::KeyBackspace => {
                        Event::Key(Key::Backspace)
                    }
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
                    pancurses::Input::KeySPrevious => {
                        Event::Shift(Key::PageUp)
                    }
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
                }
            } else {
                Event::Refresh
            }
        })
    }

    fn set_refresh_rate(&mut self, fps: u32) {
        if fps == 0 {
            self.window.timeout(-1);
        } else {
            self.window.timeout(1000 / fps as i32);
        }
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
        pancurses::BUTTON1_PRESSED
        | pancurses::BUTTON2_PRESSED
        | pancurses::BUTTON3_PRESSED => f(MouseEvent::Press(button)),
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
        | pancurses::BUTTON5_DOUBLE_CLICKED => for _ in 0..2 {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        },
        pancurses::BUTTON1_TRIPLE_CLICKED
        | pancurses::BUTTON2_TRIPLE_CLICKED
        | pancurses::BUTTON3_TRIPLE_CLICKED
        | pancurses::BUTTON4_TRIPLE_CLICKED
        | pancurses::BUTTON5_TRIPLE_CLICKED => for _ in 0..3 {
            f(MouseEvent::Press(button));
            f(MouseEvent::Release(button));
        },
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
