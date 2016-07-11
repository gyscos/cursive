use std::cmp::max;
use std::any::Any;

use Cursive;
use align::*;
use event::*;
use theme::ColorStyle;
use view::{Selector, TextView, View};
use view::{Button, SizedView};
use vec::{ToVec4, Vec2, Vec4};
use printer::Printer;

use unicode_width::UnicodeWidthStr;

#[derive(PartialEq)]
enum Focus {
    Content,
    Button(usize),
}

/// Popup-like view with a main content, and optional buttons under it.
///
/// # Examples
///
/// ```
/// # use cursive::view::{Dialog,TextView};
/// let dialog = Dialog::new(TextView::new("Hello!"))
///                     .button("Ok", |s| s.quit());
/// ```
pub struct Dialog {
    title: String,
    content: Box<View>,

    buttons: Vec<SizedView<Button>>,

    padding: Vec4,
    borders: Vec4,

    focus: Focus,

    align: Align,
}

impl Dialog {
    /// Creates a new Dialog with the given content.
    pub fn new<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: Box::new(view),
            buttons: Vec::new(),
            title: String::new(),
            focus: Focus::Content,
            padding: Vec4::new(1, 1, 0, 0),
            borders: Vec4::new(1, 1, 1, 1),
            align: Align::top_right(),
        }
    }

    pub fn info(text: &str) -> Self {
        Self::new(TextView::new(text)).dismiss_button("Ok")
    }

    /// Adds a button to the dialog with the given label and callback.
    ///
    /// Consumes and returns self for easy chaining.
    pub fn button<F>(mut self, label: &str, cb: F) -> Self
        where F: Fn(&mut Cursive) + 'static
    {
        self.buttons.push(SizedView::new(Button::new(label, cb)));

        self
    }

    /// Sets the horizontal alignment for the buttons, if any.
    /// Only works if the buttons are as a row at the bottom of the dialog.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Sets the vertical alignment for the buttons, if any.
    /// Only works if the buttons are as a column to the right of the dialog.
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Shortcut method to add a button that will dismiss the dialog.
    pub fn dismiss_button(self, label: &str) -> Self {
        self.button(label, |s| s.screen_mut().pop_layer())
    }

    /// Sets the title of the dialog.
    /// If not empty, it will be visible at the top.
    pub fn title(mut self, label: &str) -> Self {
        self.title = label.to_string();
        self
    }

    /// Sets the padding in the dialog (around content and buttons).
    pub fn padding<T: ToVec4>(mut self, padding: T) -> Self {
        self.padding = padding.to_vec4();

        self
    }
}

impl View for Dialog {
    fn draw(&mut self, printer: &Printer) {

        // This will be the buttons_height used by the buttons.
        let mut buttons_height = 0;
        // Current horizontal position of the next button we'll draw.

        // Sum of the sizes + len-1 for margins
        let width = if self.buttons.is_empty() {
            0
        } else {
            self.buttons
                .iter()
                .map(|button| button.size.x)
                .fold(0, |a, b| a + b) + self.buttons.len() - 1
        };
        let overhead = self.padding + self.borders;
        let mut offset = overhead.left +
                         self.align
            .h
            .get_offset(width, printer.size.x - overhead.horizontal());
        let y = printer.size.y - self.padding.bottom - self.borders.bottom - 1;

        for (i, button) in self.buttons.iter_mut().enumerate() {
            let size = button.size;
            // Add some special effect to the focused button
            button.draw(&printer.sub_printer(Vec2::new(offset, y),
                                             size,
                                             self.focus == Focus::Button(i)));
            // Keep 1 blank between two buttons
            offset += size.x + 1;
            // Also keep 1 blank above the buttons
            buttons_height = max(buttons_height, size.y + 1);
        }

        // What do we have left?
        let inner_size = printer.size - Vec2::new(0, buttons_height) -
                         self.borders.combined() -
                         self.padding.combined();

        self.content
            .draw(&printer.sub_printer(self.borders.top_left() +
                                       self.padding.top_left(),
                                       inner_size,
                                       self.focus == Focus::Content));

        printer.print_box(Vec2::new(0, 0), printer.size);

        if !self.title.is_empty() {
            let len = self.title.width();
            let x = (printer.size.x - len) / 2;
            printer.print((x - 2, 0), "┤ ");
            printer.print((x + len, 0), " ├");

            printer.with_color(ColorStyle::TitlePrimary,
                               |p| p.print((x, 0), &self.title));
        }

    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        // Padding and borders are not available for kids.
        let nomans_land = self.padding.combined() + self.borders.combined();

        // Buttons are not flexible, so their size doesn't depend on ours.
        let mut buttons_size = Vec2::new(0, 0);
        if !self.buttons.is_empty() {
            buttons_size.x += self.buttons.len() - 1;
        }
        for button in &mut self.buttons {
            let s = button.view.get_min_size(req);
            buttons_size.x += s.x;
            buttons_size.y = max(buttons_size.y, s.y + 1);
        }

        // We also remove one row for the buttons.
        let content_req = req - (nomans_land + Vec2::new(0, buttons_size.y));
        let content_size = self.content.get_min_size(content_req);

        // On the Y axis, we add buttons and content.
        // On the X axis, we take the max.
        let mut inner_size = Vec2::new(max(content_size.x, buttons_size.x),
                                       content_size.y + buttons_size.y) +
                             self.padding.combined() +
                             self.borders.combined();

        if !self.title.is_empty() {
            // If we have a title, we have to fit it too!
            inner_size.x = max(inner_size.x, self.title.width() + 6);
        }

        inner_size
    }

    fn layout(&mut self, mut size: Vec2) {
        // Padding and borders are taken, sorry.
        // TODO: handle border-less themes?
        size = size - (self.borders.combined() + self.padding.combined());

        // Buttons are kings, we give them everything they want.
        let mut buttons_height = 0;
        for button in self.buttons.iter_mut().rev() {
            let size = button.get_min_size(size);
            buttons_height = max(buttons_height, size.y + 1);
            button.layout(size);
        }

        // Poor content will have to make do with what's left.
        self.content.layout(size - Vec2::new(0, buttons_height));
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.focus {
            // If we are on the content, we can only go down.
            Focus::Content => {
                match self.content.on_event(event) {
                    EventResult::Ignored if !self.buttons.is_empty() => {
                        match event {
                            Event::Key(Key::Down) |
                            Event::Key(Key::Tab) |
                            Event::Shift(Key::Tab) => {
                                // Default to leftmost button when going down.
                                self.focus = Focus::Button(0);
                                EventResult::Consumed(None)
                            }
                            _ => EventResult::Ignored,
                        }
                    }
                    res => res,
                }
            }
            // If we are on a button, we have more choice
            Focus::Button(i) => {
                match self.buttons[i].on_event(event) {
                    EventResult::Ignored => {
                        match event {
                            // Up goes back to the content
                            Event::Key(Key::Up) |
                            Event::Key(Key::Tab) |
                            Event::Shift(Key::Tab) => {
                                if self.content.take_focus() {
                                    self.focus = Focus::Content;
                                    EventResult::Consumed(None)
                                } else {
                                    EventResult::Ignored
                                }
                            }
                            // Left and Right move to other buttons
                            Event::Key(Key::Right) if i + 1 <
                                                      self.buttons
                                .len() => {
                                self.focus = Focus::Button(i + 1);
                                EventResult::Consumed(None)
                            }
                            Event::Key(Key::Left) if i > 0 => {
                                self.focus = Focus::Button(i - 1);
                                EventResult::Consumed(None)
                            }
                            _ => EventResult::Ignored,
                        }
                    }
                    res => res,
                }
            }
        }
    }

    fn take_focus(&mut self) -> bool {
        // TODO: add a direction to the focus. Meanwhile, takes button first.
        if self.content.take_focus() {
            self.focus = Focus::Content;
            true
        } else if !self.buttons.is_empty() {
            self.focus = Focus::Button(0);
            true
        } else {
            false
        }
    }

    fn find(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.content.find(selector)
    }
}
