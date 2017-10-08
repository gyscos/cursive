extern crate termion;

extern crate chan_signal;

use self::termion::color as tcolor;
use self::termion::event::Event as TEvent;
use self::termion::event::Key as TKey;
use self::termion::input::TermRead;
use self::termion::raw::IntoRawMode;
use self::termion::screen::AlternateScreen;
use self::termion::style as tstyle;
use backend;
use chan;
use event::{Event, Key};
use std::cell::Cell;
use std::io::Write;
use std::thread;

use theme;

pub struct Concrete {
    terminal: AlternateScreen<termion::raw::RawTerminal<::std::io::Stdout>>,
    current_style: Cell<theme::ColorPair>,
    input: chan::Receiver<Event>,
    resize: chan::Receiver<chan_signal::Signal>,
    timeout: Option<u32>,
}

trait Effectable {
    fn on(&self);
    fn off(&self);
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

impl Concrete {
    fn apply_colors(&self, colors: theme::ColorPair) {
        with_color(&colors.front, |c| print!("{}", tcolor::Fg(c)));
        with_color(&colors.back, |c| print!("{}", tcolor::Bg(c)));
    }
}

impl backend::Backend for Concrete {
    fn init() -> Self {
        print!("{}", termion::cursor::Hide);

        let resize = chan_signal::notify(&[chan_signal::Signal::WINCH]);

        let terminal = AlternateScreen::from(::std::io::stdout()
                                                 .into_raw_mode()
                                                 .unwrap());
        let (sender, receiver) = chan::async();

        thread::spawn(move || for key in ::std::io::stdin().events() {
                          if let Ok(key) = key {
                              sender.send(map_key(key))
                          }
                      });

        let backend = Concrete {
            terminal: terminal,
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),
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

    fn with_color<F: FnOnce()>(&self, color: theme::ColorPair, f: F) {
        let current_style = self.current_style.get();

        if current_style != color {
            self.apply_colors(color);
            self.current_style.set(color);
        }

        f();

        if current_style != color {
            self.current_style.set(current_style);
            self.apply_colors(current_style);
        }
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

    fn clear(&self, color: theme::Color) {
        self.apply_colors(theme::ColorPair {
            front: color,
            back: color,
        });
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

    fn poll_event(&mut self) -> Event {
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

fn map_key(event: TEvent) -> Event {
    match event {
        TEvent::Unsupported(bytes) => Event::Unknown(bytes),
        TEvent::Key(TKey::Esc) => Event::Key(Key::Esc),
        TEvent::Key(TKey::Backspace) => Event::Key(Key::Backspace),
        TEvent::Key(TKey::Left) => Event::Key(Key::Left),
        TEvent::Key(TKey::Right) => Event::Key(Key::Right),
        TEvent::Key(TKey::Up) => Event::Key(Key::Up),
        TEvent::Key(TKey::Down) => Event::Key(Key::Down),
        TEvent::Key(TKey::Home) => Event::Key(Key::Home),
        TEvent::Key(TKey::End) => Event::Key(Key::End),
        TEvent::Key(TKey::PageUp) => Event::Key(Key::PageUp),
        TEvent::Key(TKey::PageDown) => Event::Key(Key::PageDown),
        TEvent::Key(TKey::Delete) => Event::Key(Key::Del),
        TEvent::Key(TKey::Insert) => Event::Key(Key::Ins),
        TEvent::Key(TKey::F(i)) if i < 12 => Event::Key(Key::from_f(i)),
        TEvent::Key(TKey::F(j)) => Event::Unknown(vec![j]),
        TEvent::Key(TKey::Char('\n')) => Event::Key(Key::Enter),
        TEvent::Key(TKey::Char('\t')) => Event::Key(Key::Tab),
        TEvent::Key(TKey::Char(c)) => Event::Char(c),
        TEvent::Key(TKey::Ctrl('c')) => Event::Exit,
        TEvent::Key(TKey::Ctrl(c)) => Event::CtrlChar(c),
        TEvent::Key(TKey::Alt(c)) => Event::AltChar(c),
        _ => Event::Unknown(vec![]),
    }

}

fn with_color<F, R>(clr: &theme::Color, f: F) -> R
    where F: FnOnce(&tcolor::Color) -> R
{

    match *clr {
          theme::Color::TerminalDefault => f(&tcolor::Reset),
          theme::Color::Dark(theme::BaseColor::Black) => f(&tcolor::Black),
          theme::Color::Dark(theme::BaseColor::Red) => f(&tcolor::Red),
          theme::Color::Dark(theme::BaseColor::Green) => f(&tcolor::Green),
          theme::Color::Dark(theme::BaseColor::Yellow) => f(&tcolor::Yellow),
          theme::Color::Dark(theme::BaseColor::Blue) => f(&tcolor::Blue),
          theme::Color::Dark(theme::BaseColor::Magenta) => f(&tcolor::Magenta),
          theme::Color::Dark(theme::BaseColor::Cyan) => f(&tcolor::Cyan),
          theme::Color::Dark(theme::BaseColor::White) => f(&tcolor::White),

          theme::Color::Light(theme::BaseColor::Black) => {
              f(&tcolor::LightBlack)
          }
          theme::Color::Light(theme::BaseColor::Red) => f(&tcolor::LightRed),
          theme::Color::Light(theme::BaseColor::Green) => {
              f(&tcolor::LightGreen)
          }
          theme::Color::Light(theme::BaseColor::Yellow) => {
              f(&tcolor::LightYellow)
          }
          theme::Color::Light(theme::BaseColor::Blue) => f(&tcolor::LightBlue),
          theme::Color::Light(theme::BaseColor::Magenta) => {
              f(&tcolor::LightMagenta)
          }
          theme::Color::Light(theme::BaseColor::Cyan) => f(&tcolor::LightCyan),
          theme::Color::Light(theme::BaseColor::White) => {
              f(&tcolor::LightWhite)
          }

          theme::Color::Rgb(r, g, b) => f(&tcolor::Rgb(r, g, b)),
          theme::Color::RgbLowRes(r, g, b) => {
              f(&tcolor::AnsiValue::rgb(r, g, b))
          }

      }
}
