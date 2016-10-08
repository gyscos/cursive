

use ::backend;
use ::event::{Event, Key};
use std::io::Write;
use termion;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use ::theme::{BaseColor, Color, ColorStyle, Effect};

pub struct TermionBackend {
    terminal: termion::raw::RawTerminal<::std::io::Stdout>,
}

impl backend::Backend for TermionBackend {
    fn init() -> Self {
        print!("{}", termion::cursor::Hide);
        Self::clear();


        TermionBackend {
            terminal: ::std::io::stdout().into_raw_mode().unwrap(),
        }
    }

    fn finish(&mut self) {
        // Maybe we should clear everything?
        print!("{}{}", termion::cursor::Show, termion::cursor::Goto(1, 1));
        Self::clear();
    }

    fn init_color_style(&mut self, style: ColorStyle, foreground: &Color,
                        background: &Color) {
        // Do we _need_ to save the color?
    }

    fn with_color<F: FnOnce()>(&self, color: ColorStyle, f: F) {
        // TODO: actually use colors
        // TODO: careful! need to remember the previous state
        //                and apply it back
        f();
    }

    fn with_effect<F: FnOnce()>(&self, effect: Effect, f: F) {
        // TODO: actually use effects
        // TODO: careful! need to remember the previous state
        //                and apply it back
        f();
    }

    fn has_colors(&self) -> bool {
        // TODO: color support detection?
        true
    }

    fn screen_size(&self) -> (usize, usize) {
        let (x, y) = termion::terminal_size().unwrap_or((1, 1));
        (x as usize, y as usize)
    }

    fn clear() {
        print!("{}", termion::clear::All);
    }

    fn refresh(&mut self) {
        // Not sure termion needs a refresh phase
        self.terminal.flush().unwrap();
    }

    fn print_at(&self, (x, y): (usize, usize), text: &str) {
        // TODO: terminals are 1-based. Should we add 1 here?
        print!("{}{}", termion::cursor::Goto(x as u16, y as u16), text);
    }

    fn set_refresh_rate(&mut self, fps: u32) {
        // TODO: handle async refresh, when no input is entered.
        // Could be done with a timeout on the event polling,
        // if it was supportedd.
    }

    fn poll_event(&self) -> Event {
        ::std::io::stdin().keys().next();
        Event::Key(Key::Enter)
    }
}
