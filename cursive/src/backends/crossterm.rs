//! Backend using the pure-rust crossplatform crossterm library.
//!
//! Requires the `crossterm-backend` feature.
#![cfg(feature = "crossterm-backend")]
#![cfg_attr(feature = "doc-cfg", doc(cfg(feature = "crossterm-backend")))]

use std::{
    cell::{Cell, RefCell, RefMut},
    io::{BufWriter, Write},
    time::Duration,
};

#[cfg(unix)]
use std::fs::File;

pub use crossterm;

use crossterm::{
    cursor,
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode,
        KeyEvent as CKeyEvent, KeyEventKind, KeyModifiers, MouseButton as CMouseButton,
        MouseEvent as CMouseEvent, MouseEventKind,
    },
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use crate::{
    backend,
    event::{Event, Key, MouseButton, MouseEvent},
    theme, Vec2,
};

#[cfg(windows)]
type Stdout = std::io::Stdout;

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

fn translate_key(code: KeyCode) -> Option<Key> {
    Some(match code {
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
        // These should never occur.
        _ => return None,
    })
}

fn translate_event(event: CKeyEvent) -> Option<Event> {
    const CTRL_ALT: KeyModifiers =
        KeyModifiers::from_bits_truncate(KeyModifiers::CONTROL.bits() | KeyModifiers::ALT.bits());
    const CTRL_SHIFT: KeyModifiers =
        KeyModifiers::from_bits_truncate(KeyModifiers::CONTROL.bits() | KeyModifiers::SHIFT.bits());
    const ALT_SHIFT: KeyModifiers =
        KeyModifiers::from_bits_truncate(KeyModifiers::ALT.bits() | KeyModifiers::SHIFT.bits());

    if event.kind == KeyEventKind::Press {
        Some(match event {
            // Handle Char + modifier.
            CKeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char(c),
                ..
            } => Event::CtrlChar(c),
            CKeyEvent {
                modifiers: KeyModifiers::ALT,
                code: KeyCode::Char(c),
                ..
            } => Event::AltChar(c),
            CKeyEvent {
                modifiers: KeyModifiers::SHIFT,
                code: KeyCode::Char(c),
                ..
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
                ..
            } => Event::CtrlAlt(translate_key(code)?),
            CKeyEvent {
                modifiers: CTRL_SHIFT,
                code,
                ..
            } => Event::CtrlShift(translate_key(code)?),
            CKeyEvent {
                modifiers: ALT_SHIFT,
                code,
                ..
            } => Event::AltShift(translate_key(code)?),

            // Handle key + single modifier
            CKeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => Event::Ctrl(translate_key(code)?),
            CKeyEvent {
                modifiers: KeyModifiers::ALT,
                code,
                ..
            } => Event::Alt(translate_key(code)?),
            CKeyEvent {
                modifiers: KeyModifiers::SHIFT,
                code,
                ..
            } => Event::Shift(translate_key(code)?),

            // All other keys.
            CKeyEvent { code, .. } => Event::Key(translate_key(code)?),
        })
    } else {
        None
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
            debug_assert!(
                r <= 5,
                "Red color fragment (r = {r}) is out of bound. Make sure r ≤ 5."
            );
            debug_assert!(
                g <= 5,
                "Green color fragment (g = {g}) is out of bound. Make sure g ≤ 5."
            );
            debug_assert!(
                b <= 5,
                "Blue color fragment (b = {b}) is out of bound. Make sure b ≤ 5."
            );

            Color::AnsiValue(16 + 36 * r + 6 * g + b)
        }
        theme::Color::TerminalDefault => Color::Reset,
    }
}

impl Backend {
    /// Creates a new crossterm backend.
    pub fn init() -> Result<Box<dyn backend::Backend>, std::io::Error>
    where
        Self: Sized,
    {
        #[cfg(unix)]
        let stdout = std::fs::File::create("/dev/tty")?;

        #[cfg(windows)]
        let stdout = std::io::stdout();

        Self::init_with_stdout(stdout)
    }

    fn init_with_stdout(mut stdout: Stdout) -> Result<Box<dyn backend::Backend>, std::io::Error>
    where
        Self: Sized,
    {
        enable_raw_mode()?;

        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            cursor::Hide
        )?;

        Ok(Box::new(Backend {
            current_style: Cell::new(theme::ColorPair::from_256colors(0, 0)),
            stdout: RefCell::new(BufWriter::new(stdout)),
        }))
    }

    /// Create a new crossterm backend with provided output file. Unix only
    #[cfg(unix)]
    pub fn init_with_stdout_file(outfile: File) -> Result<Box<dyn backend::Backend>, std::io::Error>
    where
        Self: Sized,
    {
        Self::init_with_stdout(outfile)
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
        f(&mut self.stdout_mut());
    }

    fn set_attr(&self, attr: Attribute) {
        self.with_stdout(|stdout| queue!(stdout, SetAttribute(attr)).unwrap());
    }

    fn map_key(&mut self, event: CEvent) -> Option<Event> {
        Some(match event {
            CEvent::Key(key_event) => translate_event(key_event)?,
            CEvent::Mouse(CMouseEvent {
                kind,
                column,
                row,
                modifiers: _,
            }) => {
                let position = (column, row).into();
                let event = match kind {
                    MouseEventKind::Down(button) => MouseEvent::Press(translate_button(button)),
                    MouseEventKind::Up(button) => MouseEvent::Release(translate_button(button)),
                    MouseEventKind::Drag(button) => MouseEvent::Hold(translate_button(button)),
                    MouseEventKind::Moved => {
                        return None;
                    }
                    MouseEventKind::ScrollDown => MouseEvent::WheelDown,
                    MouseEventKind::ScrollUp => MouseEvent::WheelUp,
                    MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {
                        // TODO: Currently unsupported.
                        return None;
                    }
                };

                Event::Mouse {
                    event,
                    position,
                    offset: Vec2::zero(),
                }
            }
            CEvent::Resize(_, _) => Event::WindowResize,
            CEvent::Paste(_) => {
                unreachable!("Did not enable bracketed paste.")
            }
            CEvent::FocusGained | CEvent::FocusLost => return None,
        })
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        // We have to execute the show cursor command at the `stdout`.
        self.with_stdout(|stdout| {
            execute!(
                stdout,
                SetForegroundColor(Color::Reset),
                SetBackgroundColor(Color::Reset),
                LeaveAlternateScreen,
                DisableMouseCapture,
                cursor::Show,
                cursor::MoveTo(0, 0),
                terminal::Clear(terminal::ClearType::All)
            )
            .expect("Can not disable mouse capture or show cursor.")
        });

        disable_raw_mode().unwrap();
    }
}

impl backend::Backend for Backend {
    fn is_persistent(&self) -> bool {
        true
    }

    fn poll_event(&mut self) -> Option<Event> {
        match poll(Duration::from_millis(1)) {
            Ok(true) => match read() {
                Ok(event) => match self.map_key(event) {
                    Some(event) => Some(event),
                    None => self.poll_event(),
                },
                Err(e) => panic!("{e:?}"),
            },
            _ => None,
        }
    }

    fn set_title(&mut self, title: String) {
        self.with_stdout(|stdout| execute!(stdout, terminal::SetTitle(title)).unwrap());
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

    fn move_to(&self, pos: Vec2) {
        self.with_stdout(|stdout| {
            queue!(stdout, cursor::MoveTo(pos.x as u16, pos.y as u16)).unwrap()
        });
    }

    fn print(&self, text: &str) {
        self.with_stdout(|stdout| queue!(stdout, Print(text)).unwrap());
    }

    fn clear(&self, color: theme::Color) {
        self.apply_colors(theme::ColorPair {
            front: color,
            back: color,
        });

        self.with_stdout(|stdout| queue!(stdout, Clear(ClearType::All)).unwrap());
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
            theme::Effect::Strikethrough => self.set_attr(Attribute::CrossedOut),
            theme::Effect::Underline => self.set_attr(Attribute::Underlined),
        }
    }

    fn unset_effect(&self, effect: theme::Effect) {
        match effect {
            theme::Effect::Simple => (),
            theme::Effect::Reverse => self.set_attr(Attribute::NoReverse),
            theme::Effect::Dim | theme::Effect::Bold => self.set_attr(Attribute::NormalIntensity),
            theme::Effect::Blink => self.set_attr(Attribute::NoBlink),
            theme::Effect::Italic => self.set_attr(Attribute::NoItalic),
            theme::Effect::Strikethrough => self.set_attr(Attribute::NotCrossedOut),
            theme::Effect::Underline => self.set_attr(Attribute::NoUnderline),
        }
    }

    fn name(&self) -> &str {
        "crossterm"
    }
}
