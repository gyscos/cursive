//! Backend using the pure-rust crossplatform crossterm library.
//!
//! Requires the `crossterm-backend` feature.

#![cfg(feature = "crossterm")]

use std::{
    cell::{Cell, RefCell, RefMut},
    fs::File,
    io::{self, BufWriter, Write},
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent,
        KeyCode, KeyEvent as CKeyEvent, KeyModifiers,
        MouseButton as CMouseButton, MouseEvent as CMouseEvent,
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
type Stdout = File;

/// Backend using crossterm
pub struct Backend {
    current_style: Cell<theme::ColorPair>,
    last_button: Option<MouseButton>,

    stdout: RefCell<BufWriter<Stdout>>,
}

impl From<CMouseButton> for MouseButton {
    fn from(button: CMouseButton) -> Self {
        match button {
            CMouseButton::Left => MouseButton::Left,
            CMouseButton::Right => MouseButton::Right,
            CMouseButton::Middle => MouseButton::Middle,
        }
    }
}

impl From<KeyCode> for Key {
    fn from(code: KeyCode) -> Self {
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
}

impl From<CKeyEvent> for Event {
    fn from(event: CKeyEvent) -> Self {
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
                code: KeyCode::Char('c'),
            } => Event::Exit,
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

            // Handle key + multiple modifiers
            CKeyEvent {
                modifiers: CTRL_ALT,
                code,
            } => Event::CtrlAlt(Key::from(code)),
            CKeyEvent {
                modifiers: CTRL_SHIFT,
                code,
            } => Event::CtrlShift(Key::from(code)),
            CKeyEvent {
                modifiers: ALT_SHIFT,
                code,
            } => Event::AltShift(Key::from(code)),

            // Handle key + single modifier
            CKeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
            } => Event::Ctrl(Key::from(code)),
            CKeyEvent {
                modifiers: KeyModifiers::ALT,
                code,
            } => Event::Alt(Key::from(code)),
            CKeyEvent {
                modifiers: KeyModifiers::SHIFT,
                code,
            } => Event::Shift(Key::from(code)),

            CKeyEvent {
                code: KeyCode::Char(c),
                ..
            } => Event::Char(c),

            // Explicitly handle 'backtab' since crossterm does not sent SHIFT alongside the back tab key.
            CKeyEvent {
                code: KeyCode::BackTab,
                ..
            } => Event::Shift(Key::Tab),

            // All other keys.
            CKeyEvent { code, .. } => Event::Key(Key::from(code)),
        }
    }
}

impl From<theme::Color> for Color {
    fn from(base_color: theme::Color) -> Self {
        match base_color {
            theme::Color::Dark(theme::BaseColor::Black) => Color::Black,
            theme::Color::Dark(theme::BaseColor::Red) => Color::DarkRed,
            theme::Color::Dark(theme::BaseColor::Green) => Color::DarkGreen,
            theme::Color::Dark(theme::BaseColor::Yellow) => Color::DarkYellow,
            theme::Color::Dark(theme::BaseColor::Blue) => Color::DarkBlue,
            theme::Color::Dark(theme::BaseColor::Magenta) => {
                Color::DarkMagenta
            }
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
}

impl Backend {
    /// Creates a new crossterm backend.
    pub fn init() -> Result<Box<dyn backend::Backend>, crossterm::ErrorKind>
    where
        Self: Sized,
    {
        enable_raw_mode()?;

        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            Hide
        )?;

        #[cfg(unix)]
        let stdout = RefCell::new(BufWriter::new(File::create("/dev/tty")?));
        #[cfg(windows)]
        let stdout = RefCell::new(BufWriter::new(io::stdout()));

        Ok(Box::new(Backend {
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),
            last_button: None,
            stdout,
        }))
    }

    fn apply_colors(&self, colors: theme::ColorPair) {
        queue!(
            self.stdout_mut(),
            SetForegroundColor(Color::from(colors.front)),
            SetBackgroundColor(Color::from(colors.back))
        )
        .unwrap();
    }

    fn stdout_mut(&self) -> RefMut<BufWriter<Stdout>> {
        self.stdout.borrow_mut()
    }

    fn set_attr(&self, attr: Attribute) {
        queue!(self.stdout_mut(), SetAttribute(attr)).unwrap();
    }

    fn map_key(&mut self, event: CEvent) -> Event {
        match event {
            CEvent::Key(key_event) => Event::from(key_event),
            CEvent::Mouse(mouse_event) => {
                let position;
                let event;

                match mouse_event {
                    CMouseEvent::Down(button, x, y, _) => {
                        let button = MouseButton::from(button);
                        self.last_button = Some(button);
                        event = MouseEvent::Press(button);
                        position = (x, y).into();
                    }
                    CMouseEvent::Up(_, x, y, _) => {
                        event = MouseEvent::Release(self.last_button.unwrap());
                        position = (x, y).into();
                    }
                    CMouseEvent::Drag(_, x, y, _) => {
                        event = MouseEvent::Hold(self.last_button.unwrap());
                        position = (x, y).into();
                    }
                    CMouseEvent::ScrollDown(x, y, _) => {
                        event = MouseEvent::WheelDown;
                        position = (x, y).into();
                    }
                    CMouseEvent::ScrollUp(x, y, _) => {
                        event = MouseEvent::WheelUp;
                        position = (x, y).into();
                    }
                };

                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            CEvent::Resize(_, _) => Event::WindowResize,
        }
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        match poll(Duration::from_millis(1)) {
            Ok(true) => match read() {
                Ok(event) => Some(self.map_key(event)),
                Err(e) => panic!("{:?}", e),
            },
            _ => None,
        }
    }

    fn finish(&mut self) {
        // We have to execute the show cursor command at the `stdout`.
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        )
        .expect("Can not disable mouse capture or show cursor.");

        disable_raw_mode().unwrap();
    }

    fn refresh(&mut self) {
        self.stdout_mut().flush().unwrap();
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
        queue!(
            self.stdout_mut(),
            MoveTo(pos.x as u16, pos.y as u16),
            Print(text)
        )
        .unwrap();
    }

    fn print_at_rep(&self, pos: Vec2, repetitions: usize, text: &str) {
        if repetitions > 0 {
            let mut out = self.stdout_mut();

            queue!(out, MoveTo(pos.x as u16, pos.y as u16)).unwrap();

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

        queue!(self.stdout_mut(), Clear(ClearType::All)).unwrap();
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
            theme::Effect::Bold => self.set_attr(Attribute::Bold),
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
            theme::Effect::Bold => self.set_attr(Attribute::NormalIntensity),
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
