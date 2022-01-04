//! Backend using the pure-rust termion library.
//!
//! Requires the `termion-backend` feature.
#![cfg(feature = "termion")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "termion-backend")))]

pub use termion;

use crossbeam_channel::{self, Receiver};
use termion::color as tcolor;
use termion::event::Event as TEvent;
use termion::event::Key as TKey;
use termion::event::MouseButton as TMouseButton;
use termion::event::MouseEvent as TMouseEvent;
use termion::input::{Events, MouseTerminal, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::style as tstyle;

use crate::backend;
use crate::backends;
use crate::event::{Event, Key, MouseButton, MouseEvent};
use crate::theme;
use crate::Vec2;

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Backend using termion
pub struct Backend {
    // Do we want to make this generic on the writer?
    terminal:
        RefCell<AlternateScreen<MouseTerminal<RawTerminal<BufWriter<File>>>>>,
    current_style: Cell<theme::ColorPair>,

    // Inner state required to parse input
    last_button: Option<MouseButton>,

    events: Events<File>,
    resize_receiver: Receiver<()>,
    running: Arc<AtomicBool>,
}

/// Set the given file to be read in non-blocking mode. That is, attempting a
/// read on the given file may return 0 bytes.
///
/// Copied from private function at https://docs.rs/nonblock/0.1.0/nonblock/.
///
/// The MIT License (MIT)
///
/// Copyright (c) 2016 Anthony Nowell
///
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
///
/// The above copyright notice and this permission notice shall be included in all
/// copies or substantial portions of the Software.
///
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// SOFTWARE.
#[cfg(unix)]
fn set_blocking(file: &File, blocking: bool) -> std::io::Result<()> {
    use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
    use std::os::unix::io::AsRawFd;

    let fd = file.as_raw_fd();
    let flags = unsafe { fcntl(fd, F_GETFL, 0) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let flags = if blocking {
        flags & !O_NONBLOCK
    } else {
        flags | O_NONBLOCK
    };
    let res = unsafe { fcntl(fd, F_SETFL, flags) };
    if res != 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

impl Backend {
    /// Creates a new termion-based backend.
    ///
    /// Uses `/dev/tty` for input and output.
    pub fn init() -> std::io::Result<Box<dyn backend::Backend>> {
        Self::init_with_files(
            File::open("/dev/tty")?,
            File::create("/dev/tty")?,
        )
    }

    /// Creates a new termion-based backend.
    ///
    /// Uses `stdin` and `stdout` for input/output.
    pub fn init_stdio() -> std::io::Result<Box<dyn backend::Backend>> {
        Self::init_with_files(
            File::open("/dev/stdin")?,
            File::create("/dev/stdout")?,
        )
    }

    /// Creates a new termion-based backend using the given input and output files.
    pub fn init_with_files(
        input_file: File,
        output_file: File,
    ) -> std::io::Result<Box<dyn backend::Backend>> {
        #[cfg(unix)]
        set_blocking(&input_file, false)?;

        // Use a ~8MB buffer
        // Should be enough for a single screen most of the time.
        let terminal =
            RefCell::new(AlternateScreen::from(MouseTerminal::from(
                BufWriter::with_capacity(8_000_000, output_file)
                    .into_raw_mode()?,
            )));

        write!(terminal.borrow_mut(), "{}", termion::cursor::Hide)?;

        let (resize_sender, resize_receiver) = crossbeam_channel::bounded(0);
        let running = Arc::new(AtomicBool::new(true));
        #[cfg(unix)]
        backends::resize::start_resize_thread(
            resize_sender,
            Arc::clone(&running),
        );

        let c = Backend {
            terminal,
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),

            last_button: None,
            events: input_file.events(),
            resize_receiver,
            running,
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

impl Drop for Backend {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

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
}

impl backend::Backend for Backend {
    fn name(&self) -> &str {
        "termion"
    }

    fn set_title(&mut self, title: String) {
        write!(self.terminal.get_mut(), "\x1B]0;{}\x07", title).unwrap();
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
            theme::Effect::Dim => self.write(tstyle::Faint),
            theme::Effect::Bold => self.write(tstyle::Bold),
            theme::Effect::Blink => self.write(tstyle::Blink),
            theme::Effect::Italic => self.write(tstyle::Italic),
            theme::Effect::Strikethrough => self.write(tstyle::CrossedOut),
            theme::Effect::Underline => self.write(tstyle::Underline),
        }
    }

    fn unset_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.write(tstyle::NoInvert),
            theme::Effect::Dim | theme::Effect::Bold => {
                self.write(tstyle::NoFaint)
            }
            theme::Effect::Blink => self.write(tstyle::NoBlink),
            theme::Effect::Italic => self.write(tstyle::NoItalic),
            theme::Effect::Strikethrough => self.write(tstyle::NoCrossedOut),
            theme::Effect::Underline => self.write(tstyle::NoUnderline),
        }
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> Vec2 {
        // TODO: termion::terminal_size currently requires stdout.
        // When available, we should try to use self.terminal or something instead.
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
        if let Some(Ok(event)) = self.events.next() {
            Some(self.map_key(event))
        } else if let Ok(()) = self.resize_receiver.try_recv() {
            Some(Event::WindowResize)
        } else {
            None
        }
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
