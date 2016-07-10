use backend;
use event;
use theme;
use utf8;

use ncurses;

pub struct NcursesBackend;

impl backend::Backend for NcursesBackend {
    fn init() {
        ::std::env::set_var("ESCDELAY", "25");
        ncurses::setlocale(ncurses::LcCategory::all, "");
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr, true);
        ncurses::noecho();
        ncurses::cbreak();
        ncurses::start_color();
        ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        ncurses::wbkgd(ncurses::stdscr,
                       ncurses::COLOR_PAIR(theme::ColorStyle::Background.id()));
    }

    fn screen_size() -> (usize, usize) {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        ncurses::getmaxyx(ncurses::stdscr, &mut y, &mut x);
        (x as usize, y as usize)
    }

    fn has_colors() -> bool {
        ncurses::has_colors()
    }

    fn finish() {
        ncurses::endwin();
    }


    fn init_color_style(style: theme::ColorStyle, foreground: &theme::Color,
                        background: &theme::Color) {
        // TODO: build the color on the spot

        ncurses::init_pair(style.id(),
                           find_closest(foreground) as i16,
                           find_closest(background) as i16);
    }

    fn with_color<F: FnOnce()>(color: theme::ColorStyle, f: F) {
        let mut current_style: ncurses::attr_t = 0;
        let mut current_color: i16 = 0;
        ncurses::attr_get(&mut current_style, &mut current_color);

        let style = ncurses::COLOR_PAIR(color.id());
        ncurses::attron(style);
        f();
        // ncurses::attroff(style);
        ncurses::attron(current_style);
    }

    fn with_effect<F: FnOnce()>(effect: theme::Effect, f: F) {
        let style = match effect {
            theme::Effect::Reverse => ncurses::A_REVERSE(),
            theme::Effect::Simple => ncurses::A_NORMAL(),
        };
        ncurses::attron(style);
        f();
        ncurses::attroff(style);
    }

    fn clear() {
        ncurses::clear();
    }

    fn refresh() {
        ncurses::refresh();
    }

    fn print_at((x, y): (usize, usize), text: &str) {
        ncurses::mvaddstr(y as i32, x as i32, text);
    }

    fn poll_event() -> event::Event {
        let ch: i32 = ncurses::getch();

        // Is it a UTF-8 starting point?
        if 32 <= ch && ch < 0x100 && ch != 127 {
            event::Event::Char(utf8::read_char(ch as u8,
                                               || ncurses::getch() as u8)
                .unwrap())
        } else {
            event::Event::Key(parse_ncurses_char(ch))
        }
    }

    fn set_refresh_rate(fps: u32) {
        if fps == 0 {
            ncurses::timeout(-1);
        } else {
            ncurses::timeout(1000 / fps as i32);
        }
    }
}

/// Returns the Key enum corresponding to the given ncurses event.
fn parse_ncurses_char(ch: i32) -> event::Key {

    match ch {
        // Values under 256 are chars and control values
        //
        // Tab is '\t'
        9 => event::Key::Tab,
        // Treat '\n' and the numpad Enter the same
        10 |
        ncurses::KEY_ENTER => event::Key::Enter,
        // This is the escape key when pressed by itself.
        // When used for control sequences, it should have been caught earlier.
        27 => event::Key::Esc,
        // `Backspace` sends 127, but Ctrl-H sends `Backspace`
        127 |
        ncurses::KEY_BACKSPACE => event::Key::Backspace,

        410 => event::Key::Resize,

        // Values 512 and above are probably extensions
        // Those keys don't seem to be documented...
        519 => event::Key::AltDel,
        520 => event::Key::AltShiftDel,
        521 => event::Key::CtrlDel,
        522 => event::Key::CtrlShiftDel,
        // 523: CtrlAltDel?
        //
        // 524?
        525 => event::Key::AltDown,
        526 => event::Key::AltShiftDown,
        527 => event::Key::CtrlDown,
        528 => event::Key::CtrlShiftDown,
        529 => event::Key::CtrlAltDown,

        530 => event::Key::AltEnd,
        531 => event::Key::AltShiftEnd,
        532 => event::Key::CtrlEnd,
        533 => event::Key::CtrlShiftEnd,
        534 => event::Key::CtrlAltEnd,

        535 => event::Key::AltHome,
        536 => event::Key::AltShiftHome,
        537 => event::Key::CtrlHome,
        538 => event::Key::CtrlShiftHome,
        539 => event::Key::CtrlAltHome,

        540 => event::Key::AltIns,
        // 541: AltShiftIns?
        542 => event::Key::CtrlIns,
        // 543: CtrlShiftIns?
        544 => event::Key::CtrlAltIns,

        545 => event::Key::AltLeft,
        546 => event::Key::AltShiftLeft,
        547 => event::Key::CtrlLeft,
        548 => event::Key::CtrlShiftLeft,
        549 => event::Key::CtrlAltLeft,

        550 => event::Key::AltPageDown,
        551 => event::Key::AltShiftPageDown,
        552 => event::Key::CtrlPageDown,
        553 => event::Key::CtrlShiftPageDown,
        554 => event::Key::CtrlAltPageDown,

        555 => event::Key::AltPageUp,
        556 => event::Key::AltShiftPageUp,
        557 => event::Key::CtrlPageUp,
        558 => event::Key::CtrlShiftPageUp,
        559 => event::Key::CtrlAltPageUp,

        560 => event::Key::AltRight,
        561 => event::Key::AltShiftRight,
        562 => event::Key::CtrlRight,
        563 => event::Key::CtrlShiftRight,
        564 => event::Key::CtrlAltRight,
        // 565?
        566 => event::Key::AltUp,
        567 => event::Key::AltShiftUp,
        568 => event::Key::CtrlUp,
        569 => event::Key::CtrlShiftUp,
        570 => event::Key::CtrlAltUp,

        ncurses::KEY_B2 => event::Key::NumpadCenter,
        ncurses::KEY_DC => event::Key::Del,
        ncurses::KEY_IC => event::Key::Ins,
        ncurses::KEY_BTAB => event::Key::ShiftTab,
        ncurses::KEY_SLEFT => event::Key::ShiftLeft,
        ncurses::KEY_SRIGHT => event::Key::ShiftRight,
        ncurses::KEY_LEFT => event::Key::Left,
        ncurses::KEY_RIGHT => event::Key::Right,
        ncurses::KEY_UP => event::Key::Up,
        ncurses::KEY_DOWN => event::Key::Down,
        ncurses::KEY_SR => event::Key::ShiftUp,
        ncurses::KEY_SF => event::Key::ShiftDown,
        ncurses::KEY_PPAGE => event::Key::PageUp,
        ncurses::KEY_NPAGE => event::Key::PageDown,
        ncurses::KEY_HOME => event::Key::Home,
        ncurses::KEY_END => event::Key::End,
        ncurses::KEY_SHOME => event::Key::ShiftHome,
        ncurses::KEY_SEND => event::Key::ShiftEnd,
        ncurses::KEY_SDC => event::Key::ShiftDel,
        ncurses::KEY_SNEXT => event::Key::ShiftPageDown,
        ncurses::KEY_SPREVIOUS => event::Key::ShiftPageUp,
        // All Fn keys use the same enum with associated number
        f @ ncurses::KEY_F1...ncurses::KEY_F12 => {
            event::Key::F((f - ncurses::KEY_F0) as u8)
        }
        f @ 277...288 => event::Key::ShiftF((f - 277) as u8),
        f @ 289...300 => event::Key::CtrlF((f - 289) as u8),
        f @ 301...312 => event::Key::CtrlShiftF((f - 300) as u8),
        f @ 313...324 => event::Key::AltF((f - 313) as u8),
        // Shift and Ctrl F{1-4} need escape sequences...
        //
        // TODO: shift and ctrl Fn keys
        // Avoids 8-10 (H,I,J), they are used by other commands.
        c @ 1...7 | c @ 11...25 => {
            event::Key::CtrlChar((b'a' + (c - 1) as u8) as char)
        }
        _ => event::Key::Unknown(ch),
    }
}

fn find_closest(color: &theme::Color) -> u8 {
    match *color {
        theme::Color::Black => 0,
        theme::Color::Red => 1,
        theme::Color::Green => 2,
        theme::Color::Yellow => 3,
        theme::Color::Blue => 4,
        theme::Color::Magenta => 5,
        theme::Color::Cyan => 6,
        theme::Color::White => 7,
        theme::Color::Rgb(r, g, b) => {
            let r = 6 * r as u16 / 256;
            let g = 6 * g as u16 / 256;
            let b = 6 * b as u16 / 256;
            (16 + 36 * r + 6 * g + b) as u8
        }
        theme::Color::RgbLowRes(r, g, b) => (16 + 36 * r + 6 * g + b) as u8,
    }
}
