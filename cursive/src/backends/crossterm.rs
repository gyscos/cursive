//! Backend using the pure-rust crossplatform crossterm library.
//!
//! Requires the `crossterm-backend` feature.
#![cfg(feature = "crossterm")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]

use std::{
    cell::{Cell, RefCell, RefMut},
    io::{self, BufWriter, Write},
    time::Duration,
};

pub use crossterm;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent,
        KeyCode, KeyEvent as CKeyEvent, KeyModifiers,
        MouseButton as CMouseButton, MouseEvent as CMouseEvent,
        MouseEventKind,
    },
    execute, queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};

use crate::{
    backend,
    event::{Event, Key, MouseButton, MouseEvent},
    theme, Vec2,
};

#[cfg(windows)]
type Stdout = io::Stdout;

#[cfg(unix)]
type Stdout = std::fs::File;

/// Backend using crossterm
pub struct Backend {
    current_style: Cell<theme::ColorPair>,

    stdout: RefCell<BufWriter<Stdout>>,
}

fn translate_button(button: CMouseButton) -> MouseButton {
    match button {
        CMouseButton::Left => MouseButton::Left,
        CMouseButton::Right => MouseButton::Right,
        CMouseButton::Middle => MouseButton::Middle,
    }
}

fn translate_key(code: KeyCode) -> Key {
    match code {
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Delete => Key::Del,
        KeyCode::Insert => Key::Ins,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::F(n) => Key::from_f(n),
        KeyCode::BackTab => Key::Tab, /* not supported */
        KeyCode::Char(_) => Key::Tab, /* is handled at `Event` level, use tab as default */
        KeyCode::Null => Key::Tab, /* is handled at `Event` level, use tab as default */
    }
}

fn translate_event(event: CKeyEvent) -> Event {
    const CTRL_ALT: KeyModifiers = KeyModifiers::from_bits_truncate(
        KeyModifiers::CONTROL.bits() | KeyModifiers::ALT.bits(),
    );
    const CTRL_SHIFT: KeyModifiers = KeyModifiers::from_bits_truncate(
        KeyModifiers::CONTROL.bits() | KeyModifiers::SHIFT.bits(),
    );
    const ALT_SHIFT: KeyModifiers = KeyModifiers::from_bits_truncate(
        KeyModifiers::ALT.bits() | KeyModifiers::SHIFT.bits(),
    );

    match event {
        // Handle Char + modifier.
        CKeyEvent {
            modifiers: KeyModifiers::CONTROL,
            code: KeyCode::Char(c),
        } => Event::CtrlChar(c),
        CKeyEvent {
            modifiers: KeyModifiers::ALT,
            code: KeyCode::Char(c),
        } => Event::AltChar(c),
        CKeyEvent {
            modifiers: KeyModifiers::SHIFT,
            code: KeyCode::Char(c),
        } => Event::Char(c),
        CKeyEvent {
            code: KeyCode::Char(c),
            ..
        } => Event::Char(c),
        // From now on, assume the key is never a `Char`.

        // Explicitly handle 'backtab' since crossterm does not sent SHIFT alongside the back tab key.
        CKeyEvent {
            code: KeyCode::BackTab,
            ..
        } => Event::Shift(Key::Tab),

        // Handle key + multiple modifiers
        CKeyEvent {
            modifiers: CTRL_ALT,
            code,
        } => Event::CtrlAlt(translate_key(code)),
        CKeyEvent {
            modifiers: CTRL_SHIFT,
            code,
        } => Event::CtrlShift(translate_key(code)),
        CKeyEvent {
            modifiers: ALT_SHIFT,
            code,
        } => Event::AltShift(translate_key(code)),

        // Handle key + single modifier
        CKeyEvent {
            modifiers: KeyModifiers::CONTROL,
            code,
        } => Event::Ctrl(translate_key(code)),
        CKeyEvent {
            modifiers: KeyModifiers::ALT,
            code,
        } => Event::Alt(translate_key(code)),
        CKeyEvent {
            modifiers: KeyModifiers::SHIFT,
            code,
        } => Event::Shift(translate_key(code)),

        // All other keys.
        CKeyEvent { code, .. } => Event::Key(translate_key(code)),
    }
}

fn translate_color(base_color: theme::Color) -> Color {
    match base_color {
        theme::Color::Dark(theme::BaseColor::Black) => Color::Black,
        theme::Color::Dark(theme::BaseColor::Red) => Color::DarkRed,
        theme::Color::Dark(theme::BaseColor::Green) => Color::DarkGreen,
        theme::Color::Dark(theme::BaseColor::Yellow) => Color::DarkYellow,
        theme::Color::Dark(theme::BaseColor::Blue) => Color::DarkBlue,
        theme::Color::Dark(theme::BaseColor::Magenta) => Color::DarkMagenta,
        theme::Color::Dark(theme::BaseColor::Cyan) => Color::DarkCyan,
        theme::Color::Dark(theme::BaseColor::White) => Color::Grey,
        theme::Color::Light(theme::BaseColor::Black) => Color::DarkGrey,
        theme::Color::Light(theme::BaseColor::Red) => Color::Red,
        theme::Color::Light(theme::BaseColor::Green) => Color::Green,
        theme::Color::Light(theme::BaseColor::Yellow) => Color::Yellow,
        theme::Color::Light(theme::BaseColor::Blue) => Color::Blue,
        theme::Color::Light(theme::BaseColor::Magenta) => Color::Magenta,
        theme::Color::Light(theme::BaseColor::Cyan) => Color::Cyan,
        theme::Color::Light(theme::BaseColor::White) => Color::White,
        theme::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
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

            Color::AnsiValue(16 + 36 * r + 6 * g + b)
        }
        theme::Color::TerminalDefault => Color::Reset,
    }
}

impl Backend {
    /// Creates a new crossterm backend.
    pub fn init() -> Result<Box<dyn backend::Backend>, crossterm::ErrorKind>
    where
        Self: Sized,
    {
        enable_raw_mode()?;

        // TODO: Use the stdout we define down there
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            Hide
        )?;

        #[cfg(unix)]
        let stdout =
            RefCell::new(BufWriter::new(std::fs::File::create("/dev/tty")?));

        #[cfg(windows)]
        let stdout = RefCell::new(BufWriter::new(io::stdout()));

        Ok(Box::new(Backend {
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),
            stdout,
        }))
    }

    fn apply_colors(&self, colors: theme::ColorPair) {
        self.with_stdout(|stdout| {
            queue!(
                stdout,
                SetForegroundColor(translate_color(colors.front)),
                SetBackgroundColor(translate_color(colors.back))
            )
            .unwrap()
        });
    }

    fn stdout_mut(&self) -> RefMut<BufWriter<Stdout>> {
        self.stdout.borrow_mut()
    }

    fn with_stdout(&self, f: impl FnOnce(&mut BufWriter<Stdout>)) {
        f(&mut *self.stdout_mut());
    }

    fn set_attr(&self, attr: Attribute) {
        self.with_stdout(|stdout| queue!(stdout, SetAttribute(attr)).unwrap());
    }

    fn map_key(&mut self, event: CEvent) -> Option<Event> {
        Some(match event {
            CEvent::Key(key_event) => translate_event(key_event),
            CEvent::Mouse(CMouseEvent {
                kind,
                column,
                row,
                modifiers: _,
            }) => {
                let position = (column, row).into();
                let event = match kind {
                    MouseEventKind::Down(button) => {
                        MouseEvent::Press(translate_button(button))
                    }
                    MouseEventKind::Up(button) => {
                        MouseEvent::Release(translate_button(button))
                    }
                    MouseEventKind::Drag(button) => {
                        MouseEvent::Hold(translate_button(button))
                    }
                    MouseEventKind::Moved => {
                        return None;
                    }
                    MouseEventKind::ScrollDown => MouseEvent::WheelDown,
                    MouseEventKind::ScrollUp => MouseEvent::WheelUp,
                };

                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            CEvent::Resize(_, _) => Event::WindowResize,
        })
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        // We have to execute the show cursor command at the `stdout`.
        self.with_stdout(|stdout| {
            execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, Show)
                .expect("Can not disable mouse capture or show cursor.")
        });

        disable_raw_mode().unwrap();
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        match poll(Duration::from_millis(1)) {
            Ok(true) => match read() {
                Ok(event) => match self.map_key(event) {
                    Some(event) => Some(event),
                    None => return self.poll_event(),
                },
                Err(e) => panic!("{:?}", e),
            },
            _ => None,
        }
    }

    fn set_title(&mut self, title: String) {
        self.with_stdout(|stdout| {
            execute!(stdout, terminal::SetTitle(title)).unwrap()
        });
    }

    fn refresh(&mut self) {
        self.with_stdout(|stdout| stdout.flush().unwrap());
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> Vec2 {
        let size = terminal::size().unwrap_or((1, 1));
        Vec2::from(size)
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        self.with_stdout(|stdout| {
            queue!(stdout, MoveTo(pos.x as u16, pos.y as u16), Print(text))
                .unwrap()
        });
    }

    fn print_at_rep(&self, pos: Vec2, repetitions: usize, text: &str) {
        if repetitions > 0 {
            self.with_stdout(|out| {
                queue!(out, MoveTo(pos.x as u16, pos.y as u16)).unwrap();

                out.write_all(text.as_bytes()).unwrap();

                let mut dupes_left = repetitions - 1;
                while dupes_left > 0 {
                    out.write_all(text.as_bytes()).unwrap();
                    dupes_left -= 1;
                }
            });
        }
    }

    fn clear(&self, color: theme::Color) {
        self.apply_colors(theme::ColorPair {
            front: color,
            back: color,
        });

        self.with_stdout(|stdout| {
            queue!(stdout, Clear(ClearType::All)).unwrap()
        });
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
            theme::Effect::Reverse => self.set_attr(Attribute::Reverse),
            theme::Effect::Dim => self.set_attr(Attribute::Dim),
            theme::Effect::Bold => self.set_attr(Attribute::Bold),
            theme::Effect::Blink => self.set_attr(Attribute::SlowBlink),
            theme::Effect::Italic => self.set_attr(Attribute::Italic),
            theme::Effect::Strikethrough => {
                self.set_attr(Attribute::CrossedOut)
            }
            theme::Effect::Underline => self.set_attr(Attribute::Underlined),
        }
    }

    fn unset_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.set_attr(Attribute::NoReverse),
            theme::Effect::Dim | theme::Effect::Bold => {
                self.set_attr(Attribute::NormalIntensity)
            }
            theme::Effect::Blink => self.set_attr(Attribute::NoBlink),
            theme::Effect::Italic => self.set_attr(Attribute::NoItalic),
            theme::Effect::Strikethrough => {
                self.set_attr(Attribute::NotCrossedOut)
            }
            theme::Effect::Underline => self.set_attr(Attribute::NoUnderline),
        }
    }

    fn name(&self) -> &str {
        "crossterm"
    }
}
