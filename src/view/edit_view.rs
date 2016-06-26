use ncurses;
use unicode_segmentation::UnicodeSegmentation;

use std::cmp::min;

use theme::ColorPair;
use vec::Vec2;
use view::{View, IdView, SizeRequest};
use event::*;
use printer::Printer;

/// Input box where the user can enter and edit text.
pub struct EditView {
    /// Current content
    content: String,
    /// Cursor position in the content
    cursor: usize,
    /// Minimum layout length asked to the parent
    min_length: usize,

    /// When the content is too long for the display, offset it
    offset: usize,
    /// Last display length, to know the possible offset range
    last_length: usize, /* scrollable: bool,
                         * TODO: add a max text length? */
}

impl EditView {
    /// Creates a new, empty edit view.
    pub fn new() -> Self {
        EditView {
            content: String::new(),
            cursor: 0,
            offset: 0,
            min_length: 1,
            last_length: 0, // scrollable: false,
        }
    }

    /// Replace the entire content of the view with the given one.
    pub fn set_content<'a>(&mut self, content: &'a str) {
        self.offset = 0;
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

    /// Wraps this view into an IdView with the given id.
    pub fn with_id(self, label: &str) -> IdView<Self> {
        IdView::new(label, self)
    }
}

fn remove_char(s: &mut String, cursor: usize) {
    let i = match s.char_indices().nth(cursor) {
        Some((i, _)) => i,
        None => return,
    };
    s.remove(i);
}

impl View for EditView {
    fn draw(&mut self, printer: &Printer) {
        // let style = if focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE };
        let len = self.content.chars().count();
        printer.with_color(ColorPair::Secondary, |printer| {
            printer.with_style(ncurses::A_REVERSE(), |printer| {
                if len < self.last_length {
                    printer.print((0, 0), &self.content);
                    printer.print_hline((len, 0), printer.size.x - len, "_");
                } else {
                    let visible_end = min(self.content.len(), self.offset + self.last_length);

                    let content = &self.content[self.offset..visible_end];
                    printer.print((0, 0), content);
                    if visible_end - self.offset < printer.size.x {
                        printer.print((printer.size.x - 1, 0), "_");
                    }
                }
            });

            // Now print cursor
            if printer.focused {
                let c = if self.cursor == len {
                    "_"
                } else {
                    // Get the char from the string... Is it so hard?
                    self.content
                        .graphemes(true)
                        .nth(self.cursor)
                        .expect(&format!("Found no char at cursor {} in {}",
                                         self.cursor,
                                         self.content))
                };
                printer.print_hline((self.cursor - self.offset, 0), 1, c);
            }
        });
    }

    fn layout(&mut self, size: Vec2) {
        self.last_length = size.x;
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
                    Some((i, _)) => self.content.insert(i, ch),
                }
                // TODO: handle wide (CJK) chars
                self.cursor += 1;
            }
            Event::KeyEvent(key) => {
                match key {
                    Key::Home => self.cursor = 0,
                    Key::End => self.cursor = self.content.chars().count(),
                    Key::Left if self.cursor > 0 => self.cursor -= 1,
                    Key::Right if self.cursor < self.content.chars().count() => self.cursor += 1,
                    Key::Backspace if self.cursor > 0 => {
                        self.cursor -= 1;
                        remove_char(&mut self.content, self.cursor);
                    }
                    Key::Del if self.cursor < self.content.chars().count() => {
                        remove_char(&mut self.content, self.cursor);
                    }
                    _ => return EventResult::Ignored,
                }
            }
        }

        // Keep cursor in [offset, offset+last_length] by changing offset
        // So keep offset in [last_length-cursor,cursor]
        // Also call this on resize, but right now it is an event like any other
        if self.cursor >= self.offset + self.last_length {
            self.offset = self.cursor - self.last_length + 1;
        } else if self.cursor < self.offset {
            self.offset = self.cursor;
        }
        if self.offset + self.last_length > self.content.len() + 1 {

            self.offset = if self.content.len() > self.last_length {
                self.content.len() - self.last_length + 1
            } else {
                0
            };
        }

        EventResult::Consumed(None)
    }
}
