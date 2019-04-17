//! Backend using the pure-rust crossplatform crossterm library.
//!
//! Requires the `crossterm-backend` feature.

#![cfg(feature = "crossterm")]

use crate::vec::Vec2;
use crate::{backend, theme};
use crossterm::{
    cursor, input, terminal, AlternateScreen, AsyncReader, Attribute,
    ClearType, Color, Colored, InputEvent as CInputEvent,
    KeyEvent as CKeyEvent, MouseButton as CMouseButton,
    MouseEvent as CMouseEvent, Terminal, TerminalCursor,
};

use crate::event::{Event, Key, MouseButton, MouseEvent};
use std::cell::{Cell, RefCell};
use std::io::{self, Stdout, Write};

/// Backend using crossterm
pub struct Backend {
    current_style: Cell<theme::ColorPair>,
    last_button: Option<MouseButton>,
    // reader to read user input async.
    async_reader: AsyncReader,
    _alternate_screen: AlternateScreen,
    stdout: RefCell<Stdout>,
    cursor: TerminalCursor,
    terminal: Terminal,
}

impl Backend {
    /// Creates a new crossterm backend.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>>
    where
        Self: Sized,
    {
        let _alternate_screen = AlternateScreen::to_alternate(true)?;

        let input = input();
        let async_reader = input.read_async();
        input.enable_mouse_mode().unwrap();

        cursor().hide()?;

        Ok(Box::new(Backend {
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),
            last_button: None,
            async_reader,
            _alternate_screen,
            stdout: RefCell::new(io::stdout()),
            terminal: terminal(),
            cursor: cursor(),
        }))
    }

    fn apply_colors(&self, colors: theme::ColorPair) {
        with_color(colors.front, |c| self.write(Colored::Fg(*c)));
        with_color(colors.back, |c| self.write(Colored::Bg(*c)));
    }

    fn write<T>(&self, content: T)
    where
        T: std::fmt::Display,
    {
        write!(self.stdout.borrow_mut(), "{}", format!("{}", content)).unwrap()
    }

    fn map_key(&mut self, event: CInputEvent) -> Event {
        match event {
            CInputEvent::Keyboard(key_event) => match key_event {
                CKeyEvent::Esc => Event::Key(Key::Esc),
                CKeyEvent::Backspace => Event::Key(Key::Backspace),
                CKeyEvent::Left => Event::Key(Key::Left),
                CKeyEvent::Right => Event::Key(Key::Right),
                CKeyEvent::Up => Event::Key(Key::Up),
                CKeyEvent::Down => Event::Key(Key::Down),
                CKeyEvent::Home => Event::Key(Key::Home),
                CKeyEvent::End => Event::Key(Key::End),
                CKeyEvent::PageUp => Event::Key(Key::PageUp),
                CKeyEvent::PageDown => Event::Key(Key::PageDown),
                CKeyEvent::Delete => Event::Key(Key::Del),
                CKeyEvent::Insert => Event::Key(Key::Ins),
                CKeyEvent::F(n) => Event::Key(Key::from_f(n)),
                CKeyEvent::Char('\n') => Event::Key(Key::Enter),
                CKeyEvent::Char('\t') => Event::Key(Key::Tab),
                CKeyEvent::Char(c) => Event::Char(c),
                CKeyEvent::Ctrl('c') => Event::Exit,
                CKeyEvent::Ctrl(c) => Event::CtrlChar(c),
                CKeyEvent::Alt(c) => Event::AltChar(c),
                _ => Event::Unknown(vec![]),
            },
            CInputEvent::Mouse(mouse_event) => match mouse_event {
                CMouseEvent::Press(btn, x, y) => {
                    let position = (x - 1, y - 1).into();

                    let event = match btn {
                        CMouseButton::Left => {
                            MouseEvent::Press(MouseButton::Left)
                        }
                        CMouseButton::Middle => {
                            MouseEvent::Press(MouseButton::Middle)
                        }
                        CMouseButton::Right => {
                            MouseEvent::Press(MouseButton::Right)
                        }
                        CMouseButton::WheelUp => MouseEvent::WheelUp,
                        CMouseButton::WheelDown => MouseEvent::WheelDown,
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
                CMouseEvent::Release(x, y) if self.last_button.is_some() => {
                    let event = MouseEvent::Release(self.last_button.unwrap());
                    let position = (x - 1, y - 1).into();

                    Event::Mouse {
                        event,
                        position,
                        offset: Vec2::zero(),
                    }
                }
                CMouseEvent::Hold(x, y) if self.last_button.is_some() => {
                    let event = MouseEvent::Hold(self.last_button.unwrap());
                    let position = (x - 1, y - 1).into();

                    Event::Mouse {
                        event,
                        position,
                        offset: Vec2::zero(),
                    }
                }
                _ => {
                    log::warn!(
                        "Unknown mouse button event {:?}!",
                        mouse_event
                    );
                    Event::Unknown(vec![])
                }
            },
            _ => {
                log::warn!("Unknown mouse event {:?}!", event);
                Event::Unknown(vec![])
            }
        }
    }
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "crossterm"
    }

    fn poll_event(&mut self) -> Option<Event> {
        self.async_reader.next().map(|event| self.map_key(event))
    }

    fn finish(&mut self) {
        self.cursor.goto(1, 1).unwrap();
        self.terminal.clear(ClearType::All).unwrap();
        self.write(Attribute::Reset);
        input().disable_mouse_mode().unwrap();
        cursor().show().unwrap();
    }

    fn refresh(&mut self) {
        self.stdout.borrow_mut().flush().unwrap();
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> Vec2 {
        let size = self.terminal.terminal_size();
        Vec2::from(size) + (1, 1)
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        self.cursor.goto(pos.x as u16, pos.y as u16).unwrap();
        self.write(text);
    }

    fn print_at_rep(&self, pos: Vec2, repetitions: usize, text: &str) {
        if repetitions > 0 {
            let mut out = self.stdout.borrow_mut();

            self.cursor.goto(pos.x as u16, pos.y as u16).unwrap();

            // as I (Timon) wrote this I figured out that calling `write_str` for unix was flushing the stdout.
            // Current work aground is writing bytes instead of a string to the terminal.
            out.write_all(text.as_bytes()).unwrap();

            let mut dupes_left = repetitions - 1;
            while dupes_left > 0 {
                out.write_all(text.as_bytes()).unwrap();
                dupes_left -= 1;
            }
        }
    }

    fn clear(&self, color: theme::Color) {
        self.apply_colors(theme::ColorPair {
            front: color,
            back: color,
        });

        self.terminal.clear(ClearType::All).unwrap();
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
            theme::Effect::Reverse => self.write(Attribute::Reverse),
            theme::Effect::Bold => self.write(Attribute::Bold),
            theme::Effect::Italic => self.write(Attribute::Italic),
            theme::Effect::Underline => self.write(Attribute::Underlined),
        }
    }

    fn unset_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.write(Attribute::Reverse),
            theme::Effect::Bold => self.write(Attribute::NoBold),
            theme::Effect::Italic => self.write(Attribute::NoItalic),
            theme::Effect::Underline => self.write(Attribute::Underlined),
        }
    }
}

fn with_color<F, R>(clr: theme::Color, f: F) -> R
where
    F: FnOnce(&Color) -> R,
{
    match clr {
        theme::Color::Dark(theme::BaseColor::Black) => f(&Color::Black),
        theme::Color::Dark(theme::BaseColor::Red) => f(&Color::DarkRed),
        theme::Color::Dark(theme::BaseColor::Green) => f(&Color::DarkGreen),
        theme::Color::Dark(theme::BaseColor::Yellow) => f(&Color::DarkYellow),
        theme::Color::Dark(theme::BaseColor::Blue) => f(&Color::DarkBlue),
        theme::Color::Dark(theme::BaseColor::Magenta) => {
            f(&Color::DarkMagenta)
        }
        theme::Color::Dark(theme::BaseColor::Cyan) => f(&Color::DarkCyan),
        theme::Color::Dark(theme::BaseColor::White) => f(&Color::Grey),

        theme::Color::Light(theme::BaseColor::Black) => f(&Color::Grey),
        theme::Color::Light(theme::BaseColor::Red) => f(&Color::Red),
        theme::Color::Light(theme::BaseColor::Green) => f(&Color::Green),
        theme::Color::Light(theme::BaseColor::Yellow) => f(&Color::Yellow),
        theme::Color::Light(theme::BaseColor::Blue) => f(&Color::Blue),
        theme::Color::Light(theme::BaseColor::Magenta) => f(&Color::Magenta),
        theme::Color::Light(theme::BaseColor::Cyan) => f(&Color::Cyan),
        theme::Color::Light(theme::BaseColor::White) => f(&Color::White),

        theme::Color::Rgb(r, g, b) => f(&Color::Rgb { r, g, b }),
        theme::Color::RgbLowRes(r, g, b) => {
            debug_assert!(r <= 5,
                          "Red color fragment (r = {}) is out of bound. Make sure r ≤ 5.",
                          r);
            debug_assert!(g <= 5,
                          "Green color fragment (g = {}) is out of bound. Make sure g ≤ 5.",
                          g);
            debug_assert!(b <= 5,
                          "Blue color fragment (b = {}) is out of bound. Make sure b ≤ 5.",
                          b);

            f(&Color::AnsiValue(16 + 36 * r + 6 * g + b))
        }

        theme::Color::TerminalDefault => {
            unimplemented!(
                "I have to take a look at how reset has to work out"
            );
        }
    }
}
