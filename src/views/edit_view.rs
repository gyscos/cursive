use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use std::rc::Rc;

use Cursive;
use With;
use direction::Direction;
use theme::{ColorStyle, Effect};
use vec::Vec2;
use view::View;
use event::*;
use Printer;


/// Input box where the user can enter and edit text.
///
/// # Examples
///
/// From the [edit example].
///
/// [edit example]: https://github.com/gyscos/Cursive/blob/master/examples/edit.rs
///
/// ```
/// # extern crate cursive;
/// # use cursive::prelude::*;
/// # fn main() {
/// let mut siv = Cursive::new();
///
/// // Create a dialog with an edit text and a button.
/// siv.add_layer(Dialog::new(EditView::new().min_length(20).with_id("edit"))
///                   .padding((1, 1, 1, 0))
///                   .title("Enter your name")
///                   .button("Ok", |s| {
///                       // When the button is clicked,
///                       // read the text and print it in a new dialog.
///                       let name = s.find_id::<EditView>("edit")
///                                   .unwrap()
///                                   .get_content()
///                                   .to_string();
///                       if name.is_empty() {
///                           s.add_layer(Dialog::new(TextView::new("Please enter a name!"))
///                                           .dismiss_button("Ok"));
///                       } else {
///                           let content = format!("Hello {}!", name);
///                           s.pop_layer();
///                           s.add_layer(Dialog::new(TextView::new(&content))
///                                           .button("Quit", |s| s.quit()));
///                       }
///                   }));
/// # }
/// ```
pub struct EditView {
    /// Current content.
    content: Rc<String>,
    /// Cursor position in the content, in bytes.
    cursor: usize,
    /// Minimum layout length asked to the parent.
    min_length: usize,

    /// Number of bytes to skip at the beginning of the content.
    ///
    /// (When the content is too long for the display, we hide part of it)
    offset: usize,
    /// Last display length, to know the possible offset range
    last_length: usize,

    /// Callback when the content is modified.
    ///
    /// Will be called with the current content and the cursor position
    on_edit: Option<Rc<Fn(&mut Cursive, &str, usize)>>,

    /// Callback when <Enter> is pressed.
    on_submit: Option<Rc<Fn(&mut Cursive, &str)>>,

    enabled: bool,
}

new_default!(EditView);

impl EditView {
    /// Creates a new, empty edit view.
    pub fn new() -> Self {
        EditView {
            content: Rc::new(String::new()),
            cursor: 0,
            offset: 0,
            min_length: 1,
            last_length: 0, // scrollable: false,
            on_edit: None,
            on_submit: None,
            enabled: true,
        }
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Sets a callback to be called whenever the content is modified.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    pub fn on_edit<F: Fn(&mut Cursive, &str, usize) + 'static>(mut self, callback: F) -> Self {
        self.on_edit = Some(Rc::new(callback));
        self
    }

    /// Sets a callback to be called when `<Enter>` is pressed.
    ///
    /// `callback` will be given the content of the view.
    pub fn on_submit<F: Fn(&mut Cursive, &str) + 'static>(mut self, callback: F) -> Self {
        self.on_submit = Some(Rc::new(callback));
        self
    }

    /// Enable or disable this view.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Replace the entire content of the view with the given one.
    pub fn set_content(&mut self, content: &str) {
        self.offset = 0;
        self.content = Rc::new(content.to_string());
    }

    /// Get the current text.
    pub fn get_content(&self) -> &str {
        &&self.content
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

    /// Insert `ch` at the current cursor position.
    pub fn insert(&mut self, ch: char) {
        // `make_mut` applies copy-on-write
        // It means it'll just return a ref if no one else has a ref,
        // and it will clone it into `self.content` otherwise.
        Rc::make_mut(&mut self.content).insert(self.cursor, ch);
    }

    /// Remove the character at the current cursor position.
    pub fn remove(&mut self, len: usize) {
        let start = self.cursor;
        let end = self.cursor + len;
        Rc::make_mut(&mut self.content).drain(start..end).collect::<Vec<_>>();
    }
}

impl View for EditView {
    fn draw(&self, printer: &Printer) {
        assert!(printer.size.x == self.last_length,
                "Was promised {}, received {}",
                self.last_length,
                printer.size.x);

        let width = self.content.width();
        printer.with_color(ColorStyle::Secondary, |printer| {
            let effect = if self.enabled {
                Effect::Reverse
            } else {
                Effect::Simple
            };
            printer.with_effect(effect, |printer| {
                if width < self.last_length {
                    // No problem, everything fits.
                    printer.print((0, 0), self.get_content());
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
                                         &self.content))
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

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn on_event(&mut self, event: Event) -> EventResult {

        match event {
            Event::Char(ch) => {
                // Find the byte index of the char at self.cursor

                self.insert(ch);
                self.cursor += ch.len_utf8();
            }
            // TODO: handle ctrl-key?
            Event::Key(Key::Home) => self.cursor = 0,
            Event::Key(Key::End) => self.cursor = self.content.len(),
            Event::Key(Key::Left) if self.cursor > 0 => {
                let len = self.content[..self.cursor]
                    .graphemes(true)
                    .last()
                    .unwrap()
                    .len();
                self.cursor -= len;
            }
            Event::Key(Key::Right) if self.cursor < self.content.len() => {
                let len = self.content[self.cursor..]
                    .graphemes(true)
                    .next()
                    .unwrap()
                    .len();
                self.cursor += len;
            }
            Event::Key(Key::Backspace) if self.cursor > 0 => {
                let len = self.content[..self.cursor]
                    .graphemes(true)
                    .last()
                    .unwrap()
                    .len();
                self.cursor -= len;
                self.remove(len);
            }
            Event::Key(Key::Del) if self.cursor < self.content.len() => {
                let len = self.content[self.cursor..]
                    .graphemes(true)
                    .next()
                    .unwrap()
                    .len();
                self.remove(len);
            }
            Event::Key(Key::Enter) if self.on_submit.is_some() => {
                let cb = self.on_submit.clone().unwrap();
                let content = self.content.clone();
                return EventResult::with_cb(move |s| {
                    cb(s, &content);
                });
            }
            _ => return EventResult::Ignored,
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

        let cb = self.on_edit.clone().map(|cb| {

            // Get a new Rc on it
            let content = self.content.clone();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, cursor);
            })
        });
        EventResult::Consumed(cb)
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
