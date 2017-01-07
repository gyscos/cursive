extern crate pancurses;


use backend;
use event::{Event, Key};

use self::super::find_closest;
use std::cell::Cell;
use theme::{Color, ColorStyle, Effect};
use utf8;

pub struct Concrete {
    window: pancurses::Window,
    current_style: Cell<ColorStyle>,
}

impl backend::Backend for Concrete {
    fn init() -> Self {
        ::std::env::set_var("ESCDELAY", "25");
        let window = pancurses::initscr();
        window.keypad(true);
        pancurses::noecho();
        pancurses::cbreak();
        pancurses::start_color();
        pancurses::curs_set(0);
        window.bkgd(pancurses::COLOR_PAIR(ColorStyle::Background.id() as pancurses::chtype));

        Concrete {
            window: window,
            current_style: Cell::new(ColorStyle::Background),
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

    fn init_color_style(&mut self, style: ColorStyle, foreground: &Color,
                        background: &Color) {
        pancurses::init_pair(style.id(),
                             find_closest(foreground) as i16,
                             find_closest(background) as i16);
    }

    fn with_color<F: FnOnce()>(&self, color: ColorStyle, f: F) {
        // TODO: pancurses doesn't have an `attr_get` equivalent
        // let mut current_style: pancurses::attr_t = 0;
        // let mut current_color: i16 = 0;
        // pancurses::attr_get(&mut current_style, &mut current_color);
        let current_style = self.current_style.get();

        let style = pancurses::COLOR_PAIR(color.id() as pancurses::chtype);
        self.window.attron(style);

        self.current_style.set(color);
        f();
        self.current_style.set(current_style);

        // self.window.attroff(style);
        self.window.attron(pancurses::COLOR_PAIR(current_style.id() as pancurses::chtype));
    }

    fn with_effect<F: FnOnce()>(&self, effect: Effect, f: F) {
        let style = match effect {
            // A_REVERSE, taken from ncurses
            Effect::Reverse => 1 << (10 + 8u32),
            Effect::Simple => pancurses::A_NORMAL,
        };
        self.window.attron(style);
        f();
        self.window.attroff(style);
    }

    fn clear(&self) {
        self.window.clear();
    }

    fn refresh(&mut self) {
        self.window.refresh();
    }

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        self.window.mvaddstr(y as i32, x as i32, text);
    }

    fn poll_event(&self) -> Event {
        // TODO: there seems to not be any indication
        // of Ctrl/Alt/Shift in these :v
        if let Some(ev) = self.window.getch() {
            match ev {
                pancurses::Input::Character('\n') => Event::Key(Key::Enter),
                // TODO: wait for a very short delay. If more keys are
                // pipelined, it may be an escape sequence.
                pancurses::Input::Character('\u{7f}') |
                pancurses::Input::Character('\u{8}') => Event::Key(Key::Backspace),
                pancurses::Input::Character('\u{9}') => Event::Key(Key::Tab),
                pancurses::Input::Character('\u{1b}') => Event::Key(Key::Esc),
                pancurses::Input::Character(c) if 32 <= (c as u32) &&
                                                  (c as u32) <= 255 => {
                    Event::Char(utf8::read_char(c as u8, || {
                            self.window.getch().and_then(|i| match i {
                                pancurses::Input::Character(c) => {
                                    Some(c as u8)
                                }
                                _ => None,
                            })
                        })
                        .unwrap())
                }
                pancurses::Input::Character(c) => Event::Unknown(c as i32),
                // TODO: Some key combos are not recognized by pancurses,
                // but are sent as Unknown. We could still parse them here.
                pancurses::Input::Unknown(i) => Event::Unknown(i),
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
