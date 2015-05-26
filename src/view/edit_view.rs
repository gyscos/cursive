use ncurses;

use color;
use vec::Vec2;
use view::{View,SizeRequest};
use event::EventResult;
use printer::Printer;

/// Displays an editable text.
pub struct EditView {
    content: String,
    cursor: usize,
    multiline: bool,
    min_length: usize,
}

impl EditView {
    /// Creates a new, empty edit view.
    pub fn new() -> Self {
        EditView {
            content: String::new(),
            cursor: 0,
            multiline: false,
            min_length: 1,
        }
    }

    /// Replace the entire content of the view with the given one.
    pub fn set_content<'a>(&mut self, content: &'a str) {
        self.content = content.to_string();
    }

    /// Get the current text.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Sets the current content to the given value. Convenient chainable method.
    pub fn content<'a>(mut self, content: &'a str) -> Self {
        self.set_content(content);
        self
    }

    /// Sets the minimum length for this view.
    /// (This applies to the layout, not the content.)
    pub fn min_length(mut self, min_length: usize) -> Self {
        self.min_length = min_length;

        self
    }
}

fn read_char(ch: i32) -> Option<char> {
    // Printable ascii range: 32-126
    if ch >= ' ' as i32 && ch <= '~' as i32 {
        Some(ch as u8 as char)
    } else {
        None
    }
}

impl View for EditView {
    fn draw(&mut self, printer: &Printer, focused: bool) {
        let style = if focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE };
        printer.with_style(style, |printer| {
            printer.print((0,0), &self.content);
        });
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        Vec2::new(self.min_length, 1)
    }

    fn take_focus(&mut self) -> bool {
        true
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {

        if let Some(ch) = read_char(ch) {
            self.content.insert(self.cursor, ch);
            self.cursor += 1;
            return EventResult::Consumed(None);
        }

        match ch {
            ncurses::KEY_HOME => self.cursor = 0,
            ncurses::KEY_END => self.cursor = self.content.len(),
            ncurses::KEY_LEFT if self.cursor > 0 => self.cursor -= 1,
            ncurses::KEY_RIGHT if self.cursor < self.content.len() => self.cursor += 1,
            127 if self.cursor > 0 => { self.cursor -= 1; self.content.remove(self.cursor); },
            330 if self.cursor < self.content.len() => { self.content.remove(self.cursor); },
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }
}
