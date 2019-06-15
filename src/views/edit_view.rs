use crate::direction::Direction;
use crate::event::{Callback, Event, EventResult, Key, MouseEvent};
use crate::rect::Rect;
use crate::theme::{ColorStyle, Effect};
use crate::utils::lines::simple::{simple_prefix, simple_suffix};
use crate::vec::Vec2;
use crate::view::View;
use crate::{Cursive, Printer, With};
use std::cell::RefCell;
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Closure type for callbacks when the content is modified.
///
/// Arguments are the `Cursive`, current content of the input and cursor
/// position
pub type OnEdit = dyn Fn(&mut Cursive, &str, usize);

/// Closure type for callbacks when Enter is pressed.
///
/// Arguments are the `Cursive` and the content of the input.
pub type OnSubmit = dyn Fn(&mut Cursive, &str);

/// Input box where the user can enter and edit text.
///
/// # Examples
///
/// From the [edit example][1].
///
/// [1]: https://github.com/gyscos/Cursive/blob/master/examples/edit.rs
///
/// ```rust
/// # use cursive::Cursive;
/// # use cursive::traits::*;
/// # use cursive::views::{Dialog, EditView, TextView};
/// # fn main() {
/// let mut siv = Cursive::dummy();
///
/// // Create a dialog with an edit text and a button.
/// // The user can either hit the <Ok> button,
/// // or press Enter on the edit text.
/// siv.add_layer(
///     Dialog::new()
///         .title("Enter your name")
///         .padding((1, 1, 1, 0))
///         .content(
///             EditView::new()
///                 .on_submit(show_popup)
///                 .with_id("name")
///                 .fixed_width(20),
///         )
///         .button("Ok", |s| {
///             let name = s.call_on_id(
///                 "name",
///                 |view: &mut EditView| view.get_content(),
///             ).unwrap();
///             show_popup(s, &name);
///         }),
/// );
///
/// fn show_popup(s: &mut Cursive, name: &str) {
///     if name.is_empty() {
///         s.add_layer(Dialog::info("Please enter a name!"));
///     } else {
///         let content = format!("Hello {}!", name);
///         s.pop_layer();
///         s.add_layer(Dialog::around(TextView::new(content))
///             .button("Quit", |s| s.quit()));
///     }
/// }
///
/// # }
/// ```
pub struct EditView {
    /// Current content.
    content: Rc<String>,

    /// Cursor position in the content, in bytes.
    cursor: usize,

    /// Number of bytes to skip at the beginning of the content.
    ///
    /// (When the content is too long for the display, we hide part of it)
    offset: usize,

    /// Optional limit to the content width.
    ///
    /// Input will be rejected if it would make the content exceed this width.
    max_content_width: Option<usize>,

    /// Last display length, to know the possible offset range
    last_length: usize,

    /// Callback when the content is modified.
    ///
    /// Will be called with the current content and the cursor position.
    on_edit: Option<Rc<OnEdit>>,

    /// Callback when <Enter> is pressed.
    on_submit: Option<Rc<OnSubmit>>,

    /// When `true`, only print `*` instead of the true content.
    secret: bool,

    /// Character to fill empty space
    filler: String,

    enabled: bool,

    style: ColorStyle,
}

new_default!(EditView);

impl EditView {
    /// Creates a new, empty edit view.
    pub fn new() -> Self {
        EditView {
            content: Rc::new(String::new()),
            cursor: 0,
            offset: 0,
            last_length: 0, // scrollable: false,
            on_edit: None,
            on_submit: None,
            max_content_width: None,
            secret: false,
            filler: "_".to_string(),
            enabled: true,
            style: ColorStyle::secondary(),
        }
    }

    /// Sets a maximum width for the content.
    ///
    /// Input will be rejected if it would make the content exceed this width.
    ///
    /// Giving `None` means no maximum width is applied.
    pub fn set_max_content_width(&mut self, width: Option<usize>) {
        self.max_content_width = width;
    }

    /// Sets a maximum width for the content.
    ///
    /// Input will be rejected if it would make the content exceed this width.
    ///
    /// Chainable variant.
    pub fn max_content_width(self, width: usize) -> Self {
        self.with(|s| s.set_max_content_width(Some(width)))
    }

    /// If `secret` is `true`, the content won't be displayed in clear.
    ///
    /// Only `*` will be shown.
    pub fn set_secret(&mut self, secret: bool) {
        self.secret = secret;
    }

    /// Hides the content of the view.
    ///
    /// Only `*` will be shown.
    pub fn secret(self) -> Self {
        self.with(|s| s.set_secret(true))
    }

    /// Sets the character to fill in blank space.
    ///
    /// Defaults to "_".
    pub fn set_filler<S: Into<String>>(&mut self, filler: S) {
        self.filler = filler.into();
    }

    /// Sets the character to fill in blank space.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::EditView;
    /// let edit = EditView::new().filler(" ");
    /// ```
    pub fn filler<S: Into<String>>(self, filler: S) -> Self {
        self.with(|s| s.set_filler(filler))
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

    /// Sets the style used for this view.
    ///
    /// When the view is enabled, the style will be reversed.
    ///
    /// Defaults to `ColorStyle::Secondary`.
    pub fn set_style(&mut self, style: ColorStyle) {
        self.style = style;
    }

    /// Sets the style used for this view.
    ///
    /// When the view is enabled, the style will be reversed.
    ///
    /// Chainable variant.
    pub fn style(self, style: ColorStyle) -> Self {
        self.with(|s| s.set_style(style))
    }

    /// Sets a mutable callback to be called whenever the content is modified.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    ///
    /// *Warning*: this callback cannot be called recursively. If you somehow
    /// trigger this callback again in the given closure, it will be ignored.
    ///
    /// If you don't need a mutable closure but want the possibility of
    /// recursive calls, see [`set_on_edit`](#method.set_on_edit).
    pub fn set_on_edit_mut<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Cursive, &str, usize) + 'static,
    {
        self.set_on_edit(immut3!(callback));
    }

    /// Sets a callback to be called whenever the content is modified.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    ///
    /// This callback can safely trigger itself recursively if needed
    /// (for instance if you call `on_event` on this view from the callback).
    ///
    /// If you need a mutable closure and don't care about the recursive
    /// aspect, see [`set_on_edit_mut`](#method.set_on_edit_mut).
    pub fn set_on_edit<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, &str, usize) + 'static,
    {
        self.on_edit = Some(Rc::new(callback));
    }

    /// Sets a mutable callback to be called whenever the content is modified.
    ///
    /// Chainable variant. See [`set_on_edit_mut`](#method.set_on_edit_mut).
    pub fn on_edit_mut<F>(self, callback: F) -> Self
    where
        F: FnMut(&mut Cursive, &str, usize) + 'static,
    {
        self.with(|v| v.set_on_edit_mut(callback))
    }

    /// Sets a callback to be called whenever the content is modified.
    ///
    /// Chainable variant. See [`set_on_edit`](#method.set_on_edit).
    pub fn on_edit<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, &str, usize) + 'static,
    {
        self.with(|v| v.set_on_edit(callback))
    }

    /// Sets a mutable callback to be called when `<Enter>` is pressed.
    ///
    /// `callback` will be given the content of the view.
    ///
    /// *Warning*: this callback cannot be called recursively. If you somehow
    /// trigger this callback again in the given closure, it will be ignored.
    ///
    /// If you don't need a mutable closure but want the possibility of
    /// recursive calls, see [`set_on_submit`](#method.set_on_submit).
    pub fn set_on_submit_mut<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Cursive, &str) + 'static,
    {
        // TODO: don't duplicate all those methods.
        // Instead, have some generic function immutify()
        // or something that wraps a FnMut closure.
        let callback = RefCell::new(callback);
        self.set_on_submit(move |s, text| {
            if let Ok(mut f) = callback.try_borrow_mut() {
                (&mut *f)(s, text);
            }
        });
    }

    /// Sets a callback to be called when `<Enter>` is pressed.
    ///
    /// `callback` will be given the content of the view.
    ///
    /// This callback can safely trigger itself recursively if needed
    /// (for instance if you call `on_event` on this view from the callback).
    ///
    /// If you need a mutable closure and don't care about the recursive
    /// aspect, see [`set_on_submit_mut`](#method.set_on_submit_mut).
    pub fn set_on_submit<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, &str) + 'static,
    {
        self.on_submit = Some(Rc::new(callback));
    }

    /// Sets a mutable callback to be called when `<Enter>` is pressed.
    ///
    /// Chainable variant.
    pub fn on_submit_mut<F>(self, callback: F) -> Self
    where
        F: FnMut(&mut Cursive, &str) + 'static,
    {
        self.with(|v| v.set_on_submit_mut(callback))
    }

    /// Sets a callback to be called when `<Enter>` is pressed.
    ///
    /// Chainable variant.
    pub fn on_submit<F>(self, callback: F) -> Self
    where
        F: Fn(&mut Cursive, &str) + 'static,
    {
        self.with(|v| v.set_on_submit(callback))
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
    ///
    /// Returns a callback in response to content change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn set_content<S: Into<String>>(&mut self, content: S) -> Callback {
        let content = content.into();
        let len = content.len();

        self.content = Rc::new(content);
        self.offset = 0;
        self.set_cursor(len);

        self.make_edit_cb().unwrap_or_else(Callback::dummy)
    }

    /// Get the current text.
    pub fn get_content(&self) -> Rc<String> {
        Rc::clone(&self.content)
    }

    /// Sets the current content to the given value.
    ///
    /// Convenient chainable method.
    ///
    /// Does not run the `on_edit` callback.
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.set_content(content);
        self
    }

    /// Sets the cursor position.
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;

        self.keep_cursor_in_view();
    }

    /// Insert `ch` at the current cursor position.
    ///
    /// Returns a callback in response to content change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn insert(&mut self, ch: char) -> Callback {
        // First, make sure we can actually insert anything.
        if let Some(width) = self.max_content_width {
            // XXX: we assume here that the widths are linearly additive.
            // Is that true? What about weird combined unicode thingies?
            // Also, say the user copy+paste some content, do we want to
            // stop halfway through a possibly split grapheme?
            if ch.width().unwrap_or(0) + self.content.width() > width {
                // ABORT
                return Callback::dummy();
            }
        }

        // `make_mut` applies copy-on-write
        // It means it'll just return a ref if no one else has a ref,
        // and it will clone it into `self.content` otherwise.

        Rc::make_mut(&mut self.content).insert(self.cursor, ch);
        self.cursor += ch.len_utf8();

        self.keep_cursor_in_view();

        self.make_edit_cb().unwrap_or_else(Callback::dummy)
    }

    /// Remove the character at the current cursor position.
    ///
    /// Returns a callback in response to content change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn remove(&mut self, len: usize) -> Callback {
        let start = self.cursor;
        let end = self.cursor + len;
        for _ in Rc::make_mut(&mut self.content).drain(start..end) {}

        self.keep_cursor_in_view();

        self.make_edit_cb().unwrap_or_else(Callback::dummy)
    }

    fn make_edit_cb(&self) -> Option<Callback> {
        self.on_edit.clone().map(|cb| {
            // Get a new Rc on the content
            let content = Rc::clone(&self.content);
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, cursor);
            })
        })
    }

    fn keep_cursor_in_view(&mut self) {
        // keep cursor in [offset, offset+last_length] by changing offset
        // so keep offset in [last_length-cursor,cursor]
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
                .map(UnicodeWidthStr::width)
                .next()
                .unwrap_or(1);

            // Now, we have to fit self.content[..self.cursor]
            // into self.last_length - c_len.
            let available = match self.last_length.checked_sub(c_len) {
                Some(s) => s,
                // Weird - no available space?
                None => return,
            };
            // Look at the content before the cursor (we will print its tail).
            // From the end, count the length until we reach `available`.
            // Then sum the byte lengths.
            let suffix_length = simple_suffix(
                &self.content[self.offset..self.cursor],
                available,
            )
            .length;

            assert!(suffix_length <= self.cursor);
            self.offset = self.cursor - suffix_length;
            // Make sure the cursor is in view
            assert!(self.cursor >= self.offset);
        }

        // If we have too much space
        if self.content[self.offset..].width() < self.last_length {
            assert!(self.last_length >= 1);
            let suffix_length =
                simple_suffix(&self.content, self.last_length - 1).length;

            assert!(self.content.len() >= suffix_length);
            self.offset = self.content.len() - suffix_length;
        }
    }
}

/// Returns a `&str` with `length` characters `*`.
///
/// Only works for small `length` (1 or 2).
/// Best used for single character replacement.
fn make_small_stars(length: usize) -> &'static str {
    // TODO: be able to use any character as hidden mode?
    &"****"[..length]
}

impl View for EditView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        assert_eq!(
            printer.size.x, self.last_length,
            "Was promised {}, received {}",
            self.last_length, printer.size.x
        );

        let width = self.content.width();
        printer.with_color(self.style, |printer| {
            let effect = if self.enabled && printer.enabled {
                Effect::Reverse
            } else {
                Effect::Simple
            };
            printer.with_effect(effect, |printer| {
                if width < self.last_length {
                    // No problem, everything fits.
                    assert!(printer.size.x >= width);
                    if self.secret {
                        printer.print_hline((0, 0), width, "*");
                    } else {
                        printer.print((0, 0), &self.content);
                    }
                    let filler_len =
                        (printer.size.x - width) / self.filler.width();
                    printer.print_hline(
                        (width, 0),
                        filler_len,
                        self.filler.as_str(),
                    );
                } else {
                    let content = &self.content[self.offset..];
                    let display_bytes = content
                        .graphemes(true)
                        .scan(0, |w, g| {
                            *w += g.width();
                            if *w > self.last_length {
                                None
                            } else {
                                Some(g)
                            }
                        })
                        .map(str::len)
                        .sum();

                    let content = &content[..display_bytes];
                    let width = content.width();

                    if self.secret {
                        printer.print_hline((0, 0), width, "*");
                    } else {
                        printer.print((0, 0), content);
                    }

                    if width < self.last_length {
                        let filler_len =
                            (self.last_length - width) / self.filler.width();
                        printer.print_hline(
                            (width, 0),
                            filler_len,
                            self.filler.as_str(),
                        );
                    }
                }
            });

            // Now print cursor
            if printer.focused {
                let c: &str = if self.cursor == self.content.len() {
                    &self.filler
                } else {
                    // Get the char from the string... Is it so hard?
                    let selected = self.content[self.cursor..]
                        .graphemes(true)
                        .next()
                        .unwrap_or_else(|| {
                            panic!(
                                "Found no char at cursor {} in {}",
                                self.cursor, &self.content
                            )
                        });
                    if self.secret {
                        make_small_stars(selected.width())
                    } else {
                        selected
                    }
                };
                let offset = self.content[self.offset..self.cursor].width();
                printer.print((offset, 0), c);
            }
        });
    }

    fn layout(&mut self, size: Vec2) {
        self.last_length = size.x;
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Char(ch) => {
                return EventResult::Consumed(Some(self.insert(ch)));
            }
            // TODO: handle ctrl-key?
            Event::Key(Key::Home) => self.set_cursor(0),
            Event::Key(Key::End) => {
                // When possible, NLL to the rescue!
                let len = self.content.len();
                self.set_cursor(len);
            }
            Event::Key(Key::Left) if self.cursor > 0 => {
                let len = self.content[..self.cursor]
                    .graphemes(true)
                    .last()
                    .unwrap()
                    .len();
                let cursor = self.cursor - len;
                self.set_cursor(cursor);
            }
            Event::Key(Key::Right) if self.cursor < self.content.len() => {
                let len = self.content[self.cursor..]
                    .graphemes(true)
                    .next()
                    .unwrap()
                    .len();
                let cursor = self.cursor + len;
                self.set_cursor(cursor);
            }
            Event::Key(Key::Backspace) if self.cursor > 0 => {
                let len = self.content[..self.cursor]
                    .graphemes(true)
                    .last()
                    .unwrap()
                    .len();
                self.cursor -= len;
                return EventResult::Consumed(Some(self.remove(len)));
            }
            Event::Key(Key::Del) if self.cursor < self.content.len() => {
                let len = self.content[self.cursor..]
                    .graphemes(true)
                    .next()
                    .unwrap()
                    .len();
                return EventResult::Consumed(Some(self.remove(len)));
            }
            Event::Key(Key::Enter) if self.on_submit.is_some() => {
                let cb = self.on_submit.clone().unwrap();
                let content = Rc::clone(&self.content);
                return EventResult::with_cb(move |s| {
                    cb(s, &content);
                });
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if position.fits_in_rect(offset, (self.last_length, 1)) => {
                if let Some(position) = position.checked_sub(offset) {
                    self.cursor = self.offset
                        + simple_prefix(
                            &self.content[self.offset..],
                            position.x,
                        )
                        .length;
                }
            }
            _ => return EventResult::Ignored,
        }

        // self.keep_cursor_in_view();

        EventResult::Consumed(self.make_edit_cb())
    }

    fn important_area(&self, _: Vec2) -> Rect {
        let char_width = if self.cursor >= self.content.len() {
            // Show a space if we're at the end of the content
            1
        } else {
            // Otherwise look at the selected character.
            self.content[self.cursor..]
                .graphemes(true)
                .next()
                .unwrap()
                .width()
        };

        let x = self.content[..self.cursor].width();

        Rect::from_size((x, 0), (char_width, 1))
    }
}
