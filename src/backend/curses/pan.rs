extern crate pancurses;

use self::super::find_closest;
use backend;
use event::{Event, Key};
use std::cell::{RefCell, Cell};
use std::collections::HashMap;
use theme::{Color, ColorPair, Effect};
use utf8;

pub struct Concrete {
    current_style: Cell<ColorPair>,
    pairs: RefCell<HashMap<ColorPair, i32>>,
    window: pancurses::Window,
}

impl Concrete {
    /// Save a new color pair.
    fn insert_color(&self, pairs: &mut HashMap<ColorPair, i32>,
                    pair: ColorPair)
                    -> i32 {

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
        pancurses::init_pair(target as i16,
                             find_closest(&pair.front),
                             find_closest(&pair.back));
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
}

impl backend::Backend for Concrete {
    fn init() -> Self {
        let window = pancurses::initscr();
        ::std::env::set_var("ESCDELAY", "25");
        window.keypad(true);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
        pancurses::use_default_colors();
        pancurses::curs_set(0);

        Concrete {
            current_style: Cell::new(ColorPair::from_256colors(0, 0)),
            pairs: RefCell::new(HashMap::new()),
            window: window,
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
            Effect::Reverse => pancurses::Attribute::Reverse,
            Effect::Simple => pancurses::Attribute::Normal,
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
        // TODO: there seems to not be any indication
        // of Ctrl/Alt/Shift in these :v
        if let Some(ev) = self.window.getch() {
            match ev {
                pancurses::Input::Character('\n') => Event::Key(Key::Enter),
                // TODO: wait for a very short delay. If more keys are
                // pipelined, it may be an escape sequence.
                pancurses::Input::Character('\u{7f}') |
                pancurses::Input::Character('\u{8}') => {
                    Event::Key(Key::Backspace)
                }
                pancurses::Input::Character('\u{9}') => Event::Key(Key::Tab),
                pancurses::Input::Character('\u{1b}') => Event::Key(Key::Esc),
                pancurses::Input::Character(c) if 32 <= (c as u32) &&
                                                  (c as u32) <= 255 => {
                    Event::Char(utf8::read_char(c as u8, || {
                        self.window
                            .getch()
                            .and_then(|i| match i {
                                          pancurses::Input::Character(c) => {
                                              Some(c as u8)
                                          }
                                          _ => None,
                                      })
                    })
                                        .unwrap())
                }
                pancurses::Input::Character(c) => {
                    let mut bytes = [0u8; 4];
                    Event::Unknown(c.encode_utf8(&mut bytes)
                                       .as_bytes()
                                       .to_vec())
                }
                // TODO: Some key combos are not recognized by pancurses,
                // but are sent as Unknown. We could still parse them here.
                pancurses::Input::Unknown(other) => {
                    Event::Unknown((0..4)
                                       .map(|i| {
                                                ((other >> (8 * i)) & 0xFF) as
                                                u8
                                            })
                                       .collect())
                }
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
                pancurses::Input::KeyResize => Event::WindowResize,
                pancurses::Input::KeyEvent => Event::Refresh,
                // TODO: mouse support
                pancurses::Input::KeyMouse => Event::Refresh,
                pancurses::Input::KeyA1 => Event::Refresh,
                pancurses::Input::KeyA3 => Event::Refresh,
                pancurses::Input::KeyB2 => Event::Key(Key::NumpadCenter),
                pancurses::Input::KeyC1 => Event::Refresh,
                pancurses::Input::KeyC3 => Event::Refresh,
            }
        } else {
            Event::Refresh
        }
    }

    fn set_refresh_rate(&mut self, fps: u32) {
        if fps == 0 {
            self.window.timeout(-1);
        } else {
            self.window.timeout(1000 / fps as i32);
        }
    }
}
