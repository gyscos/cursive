use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use theme::{ColorStyle, Effect};
use vec::Vec2;
use view::{IdView, View};
use event::*;
use printer::Printer;


/// Input box where the user can enter and edit text.
pub struct EditView {
    /// Current content.
    content: String,
    /// Cursor position in the content, in bytes.
    cursor: usize,
    /// Minimum layout length asked to the parent.
    min_length: usize,

    /// Number of bytes to skip at the beginning of the content.
    ///
    /// (When the content is too long for the display, we hide part of it)
    offset: usize,
    /// Last display length, to know the possible offset range
    last_length: usize, /* scrollable: bool,
                         * TODO: add a max text length? */
}

impl Default for EditView {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn set_content(&mut self, content: &str) {
        self.offset = 0;
        self.content = content.to_string();
    }

    /// Get the current text.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Sets the current content to the given value.
    ///
    /// Convenient chainable method.
    pub fn content(mut self, content: &str) -> Self {
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

impl View for EditView {
    fn draw(&mut self, printer: &Printer) {
        assert!(printer.size.x == self.last_length);

        let width = self.content.width();
        printer.with_color(ColorStyle::Secondary, |printer| {
            printer.with_effect(Effect::Reverse, |printer| {
                if width < self.last_length {
                    // No problem, everything fits.
                    printer.print((0, 0), &self.content);
                    printer.print_hline((width, 0),
                                        printer.size.x - width,
                                        "_");
                } else {
                    let content = &self.content[self.offset..];
                    let display_bytes = content.graphemes(true)
                        .scan(0, |w, g| {
                            *w += g.width();
                            if *w > self.last_length {
                                None
                            } else {
                                Some(g)
                            }
                        })
                        .map(|g| g.len())
                        .fold(0, |a, b| a + b);

                    let content = &content[..display_bytes];

                    printer.print((0, 0), content);
                    let width = content.width();

                    if width < self.last_length {
                        printer.print_hline((width, 0),
                                            self.last_length - width,
                                            "_");
                    }
                }
            });

            // Now print cursor
            if printer.focused {
                let c = if self.cursor == self.content.len() {
                    "_"
                } else {
                    // Get the char from the string... Is it so hard?
                    self.content[self.cursor..]
                        .graphemes(true)
                        .next()
                        .expect(&format!("Found no char at cursor {} in {}",
                                         self.cursor,
                                         self.content))
                };
                let offset = self.content[self.offset..self.cursor].width();
                printer.print((offset, 0), c);
            }
        });
    }

    fn layout(&mut self, size: Vec2) {
        self.last_length = size.x;
    }

    fn get_min_size(&mut self, _: Vec2) -> Vec2 {
        Vec2::new(self.min_length, 1)
    }

    fn take_focus(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: Event) -> EventResult {

        match event {
            Event::Char(ch) => {
                // Find the byte index of the char at self.cursor

                self.content.insert(self.cursor, ch);
                // TODO: handle wide (CJK) chars
                self.cursor += ch.len_utf8();
            }
            Event::Key(key) => {
                match key {
                    Key::Home => self.cursor = 0,
                    Key::End => self.cursor = self.content.len(),
                    Key::Left if self.cursor > 0 => {
                        let len = self.content[..self.cursor]
                            .graphemes(true)
                            .last()
                            .unwrap()
                            .len();
                        self.cursor -= len;
                    }
                    Key::Right if self.cursor < self.content.len() => {
                        let len = self.content[self.cursor..]
                            .graphemes(true)
                            .next()
                            .unwrap()
                            .len();
                        self.cursor += len;
                    }
                    Key::Backspace if self.cursor > 0 => {
                        let len = self.content[..self.cursor]
                            .graphemes(true)
                            .last()
                            .unwrap()
                            .len();
                        self.cursor -= len;
                        self.content.remove(self.cursor);
                    }
                    Key::Del if self.cursor < self.content.len() => {
                        self.content.remove(self.cursor);
                    }
                    _ => return EventResult::Ignored,
                }
            }
        }

        // Keep cursor in [offset, offset+last_length] by changing offset
        // So keep offset in [last_length-cursor,cursor]
        // Also call this on resize,
        // but right now it is an event like any other
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else {
            // So we're against the right wall.
            // Let's find how much space will be taken by the selection
            // (either a char, or _)
            let c_len = self.content[self.cursor..]
                .graphemes(true)
                .map(|g| g.width())
                .next()
                .unwrap_or(1);
            // Now, we have to fit self.content[..self.cursor]
            // into self.last_length - c_len.
            let available = self.last_length - c_len;
            // Look at the content before the cursor (we will print its tail).
            // From the end, count the length until we reach `available`.
            // Then sum the byte lengths.
            let tail_bytes =
                tail_bytes(&self.content[self.offset..self.cursor], available);
            self.offset = self.cursor - tail_bytes;
            assert!(self.cursor >= self.offset);

        }

        // If we have too much space
        if self.content[self.offset..].width() < self.last_length {
            let tail_bytes = tail_bytes(&self.content, self.last_length - 1);
            self.offset = self.content.len() - tail_bytes;
        }

        EventResult::Consumed(None)
    }
}

// Return the number of bytes, from the end of text,
// which constitute the longest tail that fits in the given width.
fn tail_bytes(text: &str, width: usize) -> usize {
    text.graphemes(true)
        .rev()
        .scan(0, |w, g| {
            *w += g.width();
            if *w > width {
                None
            } else {
                Some(g)
            }
        })
        .map(|g| g.len())
        .fold(0, |a, b| a + b)
}
