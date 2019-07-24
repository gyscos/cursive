//! Backend using the pure-rust termion library.
//!
//! Requires the `termion-backend` feature.
#![cfg(feature = "termion")]

use termion;

use self::termion::color as tcolor;
use self::termion::event::Event as TEvent;
use self::termion::event::Key as TKey;
use self::termion::event::MouseButton as TMouseButton;
use self::termion::event::MouseEvent as TMouseEvent;
use self::termion::input::{MouseTerminal, TermRead};
use self::termion::raw::{IntoRawMode, RawTerminal};
use self::termion::screen::AlternateScreen;
use self::termion::style as tstyle;
use crossbeam_channel::{self, select, Receiver};

use crate::backend;
use crate::event::{Event, Key, MouseButton, MouseEvent};
use crate::theme;
use crate::vec::Vec2;

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Backend using termion
pub struct Backend {
    terminal:
        RefCell<AlternateScreen<MouseTerminal<RawTerminal<BufWriter<File>>>>>,
    current_style: Cell<theme::ColorPair>,

    // Inner state required to parse input
    last_button: Option<MouseButton>,

    input_receiver: Receiver<TEvent>,
    resize_receiver: Receiver<()>,
}

impl Backend {
    /// Creates a new termion-based backend.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>> {
        // Use a ~8MB buffer
        // Should be enough for a single screen most of the time.
        let terminal =
            RefCell::new(AlternateScreen::from(MouseTerminal::from(
                BufWriter::with_capacity(8_000_000, File::create("/dev/tty")?)
                    .into_raw_mode()?,
            )));

        write!(terminal.borrow_mut(), "{}", termion::cursor::Hide)?;

        let (input_sender, input_receiver) = crossbeam_channel::unbounded();
        let (resize_sender, resize_receiver) = crossbeam_channel::bounded(0);

        let running = Arc::new(AtomicBool::new(true));

        #[cfg(unix)]
        backend::resize::start_resize_thread(
            resize_sender,
            Arc::clone(&running),
        );

        // We want nonblocking input, but termion is blocking by default
        // Read input from a separate thread
        thread::spawn(move || {
            let input = std::fs::File::open("/dev/tty").unwrap();
            let mut events = input.events();

            // Take all the events we can
            while let Some(Ok(event)) = events.next() {
                // If we can't send, it means the receiving side closed,
                // so just stop.
                if input_sender.send(event).is_err() {
                    break;
                }
            }

            running.store(false, Ordering::Relaxed);
        });

        let c = Backend {
            terminal,
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),

            last_button: None,
            input_receiver,
            resize_receiver,
        };

        Ok(Box::new(c))
    }

    fn apply_colors(&self, colors: theme::ColorPair) {
        with_color(colors.front, |c| self.write(tcolor::Fg(c)));
        with_color(colors.back, |c| self.write(tcolor::Bg(c)));
    }

    fn map_key(&mut self, event: TEvent) -> Event {
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
            TEvent::Mouse(TMouseEvent::Press(btn, x, y)) => {
                let position = (x - 1, y - 1).into();

                let event = match btn {
                    TMouseButton::Left => MouseEvent::Press(MouseButton::Left),
                    TMouseButton::Middle => {
                        MouseEvent::Press(MouseButton::Middle)
                    }
                    TMouseButton::Right => {
                        MouseEvent::Press(MouseButton::Right)
                    }
                    TMouseButton::WheelUp => MouseEvent::WheelUp,
                    TMouseButton::WheelDown => MouseEvent::WheelDown,
                };

                if let MouseEvent::Press(btn) = event {
                    self.last_button = Some(btn);
                }

                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            TEvent::Mouse(TMouseEvent::Release(x, y))
                if self.last_button.is_some() =>
            {
                let event = MouseEvent::Release(self.last_button.unwrap());
                let position = (x - 1, y - 1).into();
                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            TEvent::Mouse(TMouseEvent::Hold(x, y))
                if self.last_button.is_some() =>
            {
                let event = MouseEvent::Hold(self.last_button.unwrap());
                let position = (x - 1, y - 1).into();
                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            _ => Event::Unknown(vec![]),
        }
    }

    fn write<T>(&self, content: T)
    where
        T: std::fmt::Display,
    {
        write!(self.terminal.borrow_mut(), "{}", content).unwrap();
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "termion"
    }

    fn finish(&mut self) {
        write!(
            self.terminal.get_mut(),
            "{}{}",
            termion::cursor::Show,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();

        write!(
            self.terminal.get_mut(),
            "{}[49m{}[39m{}",
            27 as char,
            27 as char,
            termion::clear::All
        )
        .unwrap();
    }

    fn set_color(&self, color: theme::ColorPair) -> theme::ColorPair {
        let current_style = self.current_style.get();

        if current_style != color {
            self.apply_colors(color);
            self.current_style.set(color);
        }

        current_style
    }

    fn set_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.write(tstyle::Invert),
            theme::Effect::Bold => self.write(tstyle::Bold),
            theme::Effect::Italic => self.write(tstyle::Italic),
            theme::Effect::Underline => self.write(tstyle::Underline),
        }
    }

    fn unset_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.write(tstyle::NoInvert),
            theme::Effect::Bold => self.write(tstyle::NoBold),
            theme::Effect::Italic => self.write(tstyle::NoItalic),
            theme::Effect::Underline => self.write(tstyle::NoUnderline),
        }
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> Vec2 {
        // TODO: termion::terminal_size currently requires stdout.
        // When available, we should try to use /dev/tty instead.
        let (x, y) = termion::terminal_size().unwrap_or((1, 1));
        (x, y).into()
    }

    fn clear(&self, color: theme::Color) {
        self.apply_colors(theme::ColorPair {
            front: color,
            back: color,
        });

        self.write(termion::clear::All);
    }

    fn refresh(&mut self) {
        self.terminal.get_mut().flush().unwrap();
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        write!(
            self.terminal.borrow_mut(),
            "{}{}",
            termion::cursor::Goto(1 + pos.x as u16, 1 + pos.y as u16),
            text
        )
        .unwrap();
    }

    fn print_at_rep(&self, pos: Vec2, repetitions: usize, text: &str) {
        if repetitions > 0 {
            let mut out = self.terminal.borrow_mut();
            write!(
                out,
                "{}{}",
                termion::cursor::Goto(1 + pos.x as u16, 1 + pos.y as u16),
                text
            )
            .unwrap();

            let mut dupes_left = repetitions - 1;
            while dupes_left > 0 {
                write!(out, "{}", text).unwrap();
                dupes_left -= 1;
            }
        }
    }

    fn poll_event(&mut self) -> Option<Event> {
        let event = select! {
            recv(self.input_receiver) -> event => event.ok(),
            recv(self.resize_receiver) -> _ => return Some(Event::WindowResize),
            default => return None,
        };
        event.map(|event| self.map_key(event))
    }
}

fn with_color<F, R>(clr: theme::Color, f: F) -> R
where
    F: FnOnce(&dyn tcolor::Color) -> R,
{
    match clr {
        theme::Color::TerminalDefault => f(&tcolor::Reset),
        theme::Color::Dark(theme::BaseColor::Black) => f(&tcolor::Black),
        theme::Color::Dark(theme::BaseColor::Red) => f(&tcolor::Red),
        theme::Color::Dark(theme::BaseColor::Green) => f(&tcolor::Green),
        theme::Color::Dark(theme::BaseColor::Yellow) => f(&tcolor::Yellow),
        theme::Color::Dark(theme::BaseColor::Blue) => f(&tcolor::Blue),
        theme::Color::Dark(theme::BaseColor::Magenta) => f(&tcolor::Magenta),
        theme::Color::Dark(theme::BaseColor::Cyan) => f(&tcolor::Cyan),
        theme::Color::Dark(theme::BaseColor::White) => f(&tcolor::White),

        theme::Color::Light(theme::BaseColor::Black) => f(&tcolor::LightBlack),
        theme::Color::Light(theme::BaseColor::Red) => f(&tcolor::LightRed),
        theme::Color::Light(theme::BaseColor::Green) => f(&tcolor::LightGreen),
        theme::Color::Light(theme::BaseColor::Yellow) => {
            f(&tcolor::LightYellow)
        }
        theme::Color::Light(theme::BaseColor::Blue) => f(&tcolor::LightBlue),
        theme::Color::Light(theme::BaseColor::Magenta) => {
            f(&tcolor::LightMagenta)
        }
        theme::Color::Light(theme::BaseColor::Cyan) => f(&tcolor::LightCyan),
        theme::Color::Light(theme::BaseColor::White) => f(&tcolor::LightWhite),

        theme::Color::Rgb(r, g, b) => f(&tcolor::Rgb(r, g, b)),
        theme::Color::RgbLowRes(r, g, b) => {
            f(&tcolor::AnsiValue::rgb(r, g, b))
        }
    }
}
