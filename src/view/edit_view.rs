use ncurses;

use color;
use vec::Vec2;
use view::{View,SizeRequest};
use event::*;
use printer::Printer;

/// Displays an editable text.
pub struct EditView {
    content: String,
    cursor: usize,
    min_length: usize,
}

impl EditView {
    /// Creates a new, empty edit view.
    pub fn new() -> Self {
        EditView {
            content: String::new(),
            cursor: 0,
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

fn remove_char(s: &mut String, cursor: usize) {
    let i = match s.char_indices().nth(cursor) {
        Some((i,_)) => i,
        None => return,
    };
    s.remove(i);
}

impl View for EditView {
    fn draw(&mut self, printer: &Printer, focused: bool) {
        // let style = if focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE };
        let len = self.content.chars().count();
        printer.with_color(color::SECONDARY, |printer| {
            printer.with_style(ncurses::A_REVERSE(), |printer| {
                printer.print((0,0), &self.content);
                printer.print_hline((len,0), printer.size.x-len, '_' as u64);
            });

            // Now print cursor
            if focused {
                let c = if self.cursor == len {
                    '_'
                } else {
                    // Get the char from the string... Is it so hard?
                    self.content.chars().nth(self.cursor).expect(&format!("Found no char at cursor {} in {}", self.cursor, self.content))
                };
                printer.print_hline((self.cursor, 0), 1, c as u64);
            }
        });
    }

    fn get_min_size(&self, _: SizeRequest) -> Vec2 {
        Vec2::new(self.min_length, 1)
    }

    fn take_focus(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: Event) -> EventResult {

        match event {
            Event::CharEvent(ch) => {
                // Find the byte index of the char at self.cursor

                match self.content.char_indices().nth(self.cursor) {
                    None => self.content.push(ch),
                    Some((i,_)) => self.content.insert(i, ch),
                }
                self.cursor += 1;
                return EventResult::Consumed(None);
            },
            Event::KeyEvent(key) => match key {
                Key::Home => self.cursor = 0,
                Key::End => self.cursor = self.content.chars().count(),
                Key::Left if self.cursor > 0 => self.cursor -= 1,
                Key::Right if self.cursor < self.content.chars().count() => self.cursor += 1,
                Key::Backspace if self.cursor > 0 => { self.cursor -= 1; remove_char(&mut self.content, self.cursor); },
                Key::Del if self.cursor < self.content.chars().count() => { remove_char(&mut self.content, self.cursor); },
                _ => return EventResult::Ignored,

            },
        }

        EventResult::Consumed(None)
    }
}
