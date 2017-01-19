extern crate termion;

extern crate chan_signal;

use ::backend;
use chan;
use ::event::{Event, Key};
use self::termion::color as tcolor;
use self::termion::event::Key as TKey;
use self::termion::input::TermRead;
use self::termion::raw::IntoRawMode;
use self::termion::style as tstyle;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::thread;
use std::time;

use ::theme;

pub struct Concrete {
    terminal: termion::raw::RawTerminal<::std::io::Stdout>,
    current_style: Cell<theme::ColorStyle>,
    colors: BTreeMap<i16, (Box<tcolor::Color>, Box<tcolor::Color>)>,

    input: chan::Receiver<Event>,
    resize: chan::Receiver<chan_signal::Signal>,
    timeout: Option<u32>,
}

trait Effectable {
    fn on(&self);
    fn off(&self);
}

struct ColorRef<'a>(&'a tcolor::Color);

impl<'a> tcolor::Color for ColorRef<'a> {
    #[inline]
    fn write_fg(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.write_fg(f)
    }

    #[inline]
    fn write_bg(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.write_bg(f)
    }
}

impl Effectable for theme::Effect {
    fn on(&self) {
        match *self {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => print!("{}", tstyle::Invert),
        }
    }

    fn off(&self) {
        match *self {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => print!("{}", tstyle::NoInvert),
        }
    }
}

fn apply_colors(fg: &tcolor::Color, bg: &tcolor::Color) {
    print!("{}{}", tcolor::Fg(ColorRef(fg)), tcolor::Bg(ColorRef(bg)));
}

impl Concrete {
    fn apply_colorstyle(&self, color_style: theme::ColorStyle) {
        let (ref fg, ref bg) = self.colors[&color_style.id()];
        apply_colors(&**fg, &**bg);
    }
}

impl backend::Backend for Concrete {
    fn init() -> Self {
        print!("{}", termion::cursor::Hide);

        let resize = chan_signal::notify(&[chan_signal::Signal::WINCH]);

        let terminal = ::std::io::stdout().into_raw_mode().unwrap();
        let (sender, receiver) = chan::async();

        thread::spawn(move || for key in ::std::io::stdin().keys() {
            if let Ok(key) = key {
                sender.send(map_key(key))
            }
        });

        let backend = Concrete {
            terminal: terminal,
            current_style: Cell::new(theme::ColorStyle::Background),
            colors: BTreeMap::new(),
            input: receiver,
            resize: resize,
            timeout: None,
        };

        backend
    }

    fn finish(&mut self) {
        print!("{}{}", termion::cursor::Show, termion::cursor::Goto(1, 1));
        print!("{}[49m{}[39m{}",
               27 as char,
               27 as char,
               termion::clear::All);
    }

    fn init_color_style(&mut self, style: theme::ColorStyle,
                        foreground: &theme::Color, background: &theme::Color) {
        // Step 1: convert foreground and background into proper termion Color
        self.colors.insert(style.id(),
                           (colour_to_termion_colour(foreground),
                            colour_to_termion_colour(background)));
    }

    fn with_color<F: FnOnce()>(&self, color: theme::ColorStyle, f: F) {
        let current_style = self.current_style.get();

        self.apply_colorstyle(color);

        self.current_style.set(color);
        f();
        self.current_style.set(current_style);

        self.apply_colorstyle(current_style);
    }

    fn with_effect<F: FnOnce()>(&self, effect: theme::Effect, f: F) {
        effect.on();
        f();
        effect.off();
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> (usize, usize) {
        let (x, y) = termion::terminal_size().unwrap_or((1, 1));
        (x as usize, y as usize)
    }

    fn clear(&self) {
        self.apply_colorstyle(theme::ColorStyle::Background);
        print!("{}", termion::clear::All);
    }

    fn refresh(&mut self) {
        self.terminal.flush().unwrap();
    }

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        print!("{}{}",
               termion::cursor::Goto(1 + x as u16, 1 + y as u16),
               text);
    }

    fn set_refresh_rate(&mut self, fps: u32) {
        self.timeout = Some(1000 / fps as u32);
    }

    fn poll_event(&self) -> Event {
        let input = &self.input;
        let resize = &self.resize;

        if let Some(timeout) = self.timeout {
            let timeout = chan::after_ms(timeout);
            chan_select!{
                timeout.recv() => return Event::Refresh,
                resize.recv() => return Event::WindowResize,
                input.recv() -> input => return input.unwrap(),
            }
        } else {
            chan_select!{
                resize.recv() => return Event::WindowResize,
                input.recv() -> input => return input.unwrap(),
            }
        }
    }
}

fn map_key(key: TKey) -> Event {
    match key {
        TKey::Esc => Event::Key(Key::Esc),
        TKey::Backspace => Event::Key(Key::Backspace),
        TKey::Left => Event::Key(Key::Left),
        TKey::Right => Event::Key(Key::Right),
        TKey::Up => Event::Key(Key::Up),
        TKey::Down => Event::Key(Key::Down),
        TKey::Home => Event::Key(Key::Home),
        TKey::End => Event::Key(Key::End),
        TKey::PageUp => Event::Key(Key::PageUp),
        TKey::PageDown => Event::Key(Key::PageDown),
        TKey::Delete => Event::Key(Key::Del),
        TKey::Insert => Event::Key(Key::Ins),
        TKey::F(i) if i < 12 => Event::Key(Key::from_f(i)),
        TKey::F(j) => Event::Unknown(-(j as i32)),
        TKey::Char('\n') => Event::Key(Key::Enter),
        TKey::Char('\t') => Event::Key(Key::Tab),
        TKey::Char(c) => Event::Char(c),
        TKey::Ctrl('c') => Event::Exit,
        TKey::Ctrl(c) => Event::CtrlChar(c),
        TKey::Alt(c) => Event::AltChar(c),
        _ => Event::Unknown(-1),
    }

}

fn colour_to_termion_colour(clr: &theme::Color) -> Box<tcolor::Color> {
    match *clr {
        theme::Color::Dark(theme::BaseColor::Black) => Box::new(tcolor::Black),
        theme::Color::Dark(theme::BaseColor::Red) => Box::new(tcolor::Red),
        theme::Color::Dark(theme::BaseColor::Green) => Box::new(tcolor::Green),
        theme::Color::Dark(theme::BaseColor::Yellow) => {
            Box::new(tcolor::Yellow)
        }
        theme::Color::Dark(theme::BaseColor::Blue) => Box::new(tcolor::Blue),
        theme::Color::Dark(theme::BaseColor::Magenta) => {
            Box::new(tcolor::Magenta)
        }
        theme::Color::Dark(theme::BaseColor::Cyan) => Box::new(tcolor::Cyan),
        theme::Color::Dark(theme::BaseColor::White) => Box::new(tcolor::White),

        theme::Color::Light(theme::BaseColor::Black) => {
            Box::new(tcolor::LightBlack)
        }
        theme::Color::Light(theme::BaseColor::Red) => {
            Box::new(tcolor::LightRed)
        }
        theme::Color::Light(theme::BaseColor::Green) => {
            Box::new(tcolor::LightGreen)
        }
        theme::Color::Light(theme::BaseColor::Yellow) => {
            Box::new(tcolor::LightYellow)
        }
        theme::Color::Light(theme::BaseColor::Blue) => {
            Box::new(tcolor::LightBlue)
        }
        theme::Color::Light(theme::BaseColor::Magenta) => {
            Box::new(tcolor::LightMagenta)
        }
        theme::Color::Light(theme::BaseColor::Cyan) => {
            Box::new(tcolor::LightCyan)
        }
        theme::Color::Light(theme::BaseColor::White) => {
            Box::new(tcolor::LightWhite)
        }

        theme::Color::Rgb(r, g, b) => Box::new(tcolor::Rgb(r, g, b)),
        theme::Color::RgbLowRes(r, g, b) => {
            Box::new(tcolor::AnsiValue::rgb(r, g, b))
        }
    }
}
