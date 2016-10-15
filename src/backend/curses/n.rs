extern crate ncurses;

use backend;
use event::{Event, Key};

use self::super::find_closest;
use theme::{Color, ColorStyle, Effect};
use utf8;

pub struct Concrete;

impl backend::Backend for Concrete {
    fn init() -> Self {
        // The delay is the time ncurses wait after pressing ESC
        // to see if it's an escape sequence.
        // Default delay is way too long. 25 is imperceptible yet works fine.
        ::std::env::set_var("ESCDELAY", "25");
        ncurses::setlocale(ncurses::LcCategory::all, "");
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr(), true);
        ncurses::noecho();
        ncurses::cbreak();
        ncurses::start_color();
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        ncurses::wbkgd(ncurses::stdscr(),
                       ncurses::COLOR_PAIR(ColorStyle::Background.id()));

        Concrete
    }

    fn screen_size(&self) -> (usize, usize) {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut y, &mut x);
        (x as usize, y as usize)
    }

    fn has_colors(&self) -> bool {
        ncurses::has_colors()
    }

    fn finish(&mut self) {
        ncurses::endwin();
    }


    fn init_color_style(&mut self, style: ColorStyle, foreground: &Color,
                        background: &Color) {
        // TODO: build the color on the spot

        ncurses::init_pair(style.id(),
                           find_closest(foreground) as i16,
                           find_closest(background) as i16);
    }

    fn with_color<F: FnOnce()>(&self, color: ColorStyle, f: F) {
        let mut current_style: ncurses::attr_t = 0;
        let mut current_color: i16 = 0;
        ncurses::attr_get(&mut current_style, &mut current_color);

        let style = ncurses::COLOR_PAIR(color.id());
        ncurses::attron(style);
        f();
        // ncurses::attroff(style);
        ncurses::attron(current_style);
    }

    fn with_effect<F: FnOnce()>(&self, effect: Effect, f: F) {
        let style = match effect {
            Effect::Reverse => ncurses::A_REVERSE(),
            Effect::Simple => ncurses::A_NORMAL(),
        };
        ncurses::attron(style);
        f();
        ncurses::attroff(style);
    }

    fn clear(&self) {
        ncurses::clear();
    }

    fn refresh(&mut self) {
        ncurses::refresh();
    }

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        ncurses::mvaddstr(y as i32, x as i32, text);
    }

    fn poll_event(&self) -> Event {
        let ch: i32 = ncurses::getch();

        // Is it a UTF-8 starting point?
        if 32 <= ch && ch <= 255 && ch != 127 {
            Event::Char(utf8::read_char(ch as u8,
                                        || Some(ncurses::getch() as u8))
                .unwrap())
        } else {
            parse_ncurses_char(ch)
        }
    }

    fn set_refresh_rate(&mut self, fps: u32) {
        if fps == 0 {
            ncurses::timeout(-1);
        } else {
            ncurses::timeout(1000 / fps as i32);
        }
    }
}

/// Returns the Key enum corresponding to the given ncurses event.
fn parse_ncurses_char(ch: i32) -> Event {
    match ch {
        // Value sent by ncurses when nothing happens
        -1 => Event::Refresh,

        // Values under 256 are chars and control values
        //
        // Tab is '\t'
        9 => Event::Key(Key::Tab),
        // Treat '\n' and the numpad Enter the same
        10 |
        ncurses::KEY_ENTER => Event::Key(Key::Enter),
        // This is the escape key when pressed by itself.
        // When used for control sequences, it should have been caught earlier.
        27 => Event::Key(Key::Esc),
        // `Backspace` sends 127, but Ctrl-H sends `Backspace`
        127 |
        ncurses::KEY_BACKSPACE => Event::Key(Key::Backspace),

        410 => Event::WindowResize,

        // Values 512 and above are probably extensions
        // Those keys don't seem to be documented...
        519 => Event::Alt(Key::Del),
        520 => Event::AltShift(Key::Del),
        521 => Event::Ctrl(Key::Del),
        522 => Event::CtrlShift(Key::Del),
        // 523: CtrlAltDel?
        //
        // 524?
        525 => Event::Alt(Key::Down),
        526 => Event::AltShift(Key::Down),
        527 => Event::Ctrl(Key::Down),
        528 => Event::CtrlShift(Key::Down),
        529 => Event::CtrlAlt(Key::Down),

        530 => Event::Alt(Key::End),
        531 => Event::AltShift(Key::End),
        532 => Event::Ctrl(Key::End),
        533 => Event::CtrlShift(Key::End),
        534 => Event::CtrlAlt(Key::End),

        535 => Event::Alt(Key::Home),
        536 => Event::AltShift(Key::Home),
        537 => Event::Ctrl(Key::Home),
        538 => Event::CtrlShift(Key::Home),
        539 => Event::CtrlAlt(Key::Home),

        540 => Event::Alt(Key::Ins),
        // 541: AltShiftIns?
        542 => Event::Ctrl(Key::Ins),
        // 543: CtrlShiftIns?
        544 => Event::CtrlAlt(Key::Ins),

        545 => Event::Alt(Key::Left),
        546 => Event::AltShift(Key::Left),
        547 => Event::Ctrl(Key::Left),
        548 => Event::CtrlShift(Key::Left),
        549 => Event::CtrlAlt(Key::Left),

        550 => Event::Alt(Key::PageDown),
        551 => Event::AltShift(Key::PageDown),
        552 => Event::Ctrl(Key::PageDown),
        553 => Event::CtrlShift(Key::PageDown),
        554 => Event::CtrlAlt(Key::PageDown),

        555 => Event::Alt(Key::PageUp),
        556 => Event::AltShift(Key::PageUp),
        557 => Event::Ctrl(Key::PageUp),
        558 => Event::CtrlShift(Key::PageUp),
        559 => Event::CtrlAlt(Key::PageUp),

        560 => Event::Alt(Key::Right),
        561 => Event::AltShift(Key::Right),
        562 => Event::Ctrl(Key::Right),
        563 => Event::CtrlShift(Key::Right),
        564 => Event::CtrlAlt(Key::Right),
        // 565?
        566 => Event::Alt(Key::Up),
        567 => Event::AltShift(Key::Up),
        568 => Event::Ctrl(Key::Up),
        569 => Event::CtrlShift(Key::Up),
        570 => Event::CtrlAlt(Key::Up),

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
        // All Fn keys use the same enum with associated number
        f @ ncurses::KEY_F1...ncurses::KEY_F12 => {
            Event::Key(Key::from_f((f - ncurses::KEY_F0) as u8))
        }
        f @ 277...288 => Event::Shift(Key::from_f((f - 276) as u8)),
        f @ 289...300 => Event::Ctrl(Key::from_f((f - 288) as u8)),
        f @ 301...312 => Event::CtrlShift(Key::from_f((f - 300) as u8)),
        f @ 313...324 => Event::Alt(Key::from_f((f - 312) as u8)),
        // Values 8-10 (H,I,J) are used by other commands,
        // so we probably won't receive them. Meh~
        c @ 1...25 => Event::CtrlChar((b'a' + (c - 1) as u8) as char),
        _ => Event::Unknown(ch),
    }
}
