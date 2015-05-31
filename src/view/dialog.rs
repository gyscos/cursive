use std::cmp::max;
use std::any::Any;

use color;
use ::{Cursive};
use event::*;
use view::{View,SizeRequest,DimensionRequest,Selector};
use view::{Button,SizedView};
use vec::{Vec2,Vec4,ToVec4};
use printer::Printer;

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
/// let dialog = Dialog::new(TextView::new("Hello!")).button("Ok", |s,_| s.quit());
/// ```
pub struct Dialog {
    title: String,
    content: Box<View>,

    buttons: Vec<SizedView<Button>>,

    padding: Vec4,
    borders: Vec4,

    focus: Focus,
}

impl Dialog {
    /// Creates a new Dialog with the given content.
    pub fn new<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: Box::new(view),
            buttons: Vec::new(),
            title: String::new(),
            focus: Focus::Content,
            padding: Vec4::new(1,1,0,0),
            borders: Vec4::new(1,1,1,1),
        }
    }

    /// Adds a button to the dialog with the given label and callback.
    ///
    /// Consumes and returns self for easy chaining.
    pub fn button<'a, F>(mut self, label: &'a str, cb: F) -> Self
        where F: Fn(&mut Cursive) + 'static
    {
        self.buttons.push(SizedView::new(Button::new(label, cb)));

        self
    }

    /// Shortcut method to add a button that will dismiss the dialog.
    pub fn dismiss_button<'a>(self, label: &'a str) -> Self {
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

        // This will be the height used by the buttons.
        let mut height = 0;
        // Current horizontal position of the next button we'll draw.
        let mut x = 0;
        for (i,button) in self.buttons.iter_mut().enumerate().rev() {
            let size = button.size;
            let offset = printer.size - self.borders.bot_right() - self.padding.bot_right() - size - Vec2::new(x, 0);
            // Add some special effect to the focused button
            button.draw(&printer.sub_printer(offset, size, self.focus == Focus::Button(i)));
            // Keep 1 blank between two buttons
            x += size.x + 1;
            // Also keep 1 blank above the buttons
            height = max(height, size.y+1);
        }

        // What do we have left?
        let inner_size = printer.size
            - Vec2::new(0, height)
            - self.borders.combined()
            - self.padding.combined();

        self.content.draw(&printer.sub_printer(self.borders.top_left() + self.padding.top_left(), inner_size, self.focus == Focus::Content));

        printer.print_box(Vec2::new(0,0), printer.size);

        if self.title.len() > 0 {
            let len = self.title.chars().count();
            let x = (printer.size.x - len) / 2;
            printer.print((x-2,0), "┤ ");
            printer.print((x+len,0), " ├");

            printer.with_color(color::TITLE_PRIMARY, |p| p.print((x,0), &self.title));
        }

    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        // Padding and borders are not available for kids.
        let content_req = req.reduced(self.padding.combined() + self.borders.combined());
        let content_size = self.content.get_min_size(content_req);

        let mut buttons_size = Vec2::new(0,0);
        for button in self.buttons.iter() {
            let s = button.view.get_min_size(req);
            buttons_size.x += s.x + 1;
            buttons_size.y = max(buttons_size.y, s.y + 1);
        }

        // On the Y axis, we add buttons and content.
        // On the X axis, we take the max.
        let mut inner_size = Vec2::new(max(content_size.x, buttons_size.x),
                                   content_size.y + buttons_size.y)
                        + self.padding.combined() + self.borders.combined();

        if self.title.len() > 0 {
            // If we have a title, we have to fit it too!
            inner_size.x = max(inner_size.x, self.title.chars().count() + 6);
        }

        inner_size
    }

    fn layout(&mut self, mut size: Vec2) {
        // Padding and borders are taken, sorry.
        size = size - (self.borders.combined() + self.padding.combined());
        let req = SizeRequest {
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };

        // Buttons are kings, we give them everything they want.
        let mut buttons_height = 0;
        for button in self.buttons.iter_mut().rev() {
            let size = button.get_min_size(req);
            buttons_height = max(buttons_height, size.y+1);
            button.layout(size);
        }

        // Poor content will have to make do with what's left.
        self.content.layout(size - Vec2::new(0, buttons_height));
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.focus {
            // If we are on the content, we can only go down.
            Focus::Content => match self.content.on_event(event) {
                EventResult::Ignored if !self.buttons.is_empty() => match event {
                    Event::KeyEvent(Key::Down) => {
                        // Default to leftmost button when going down.
                        self.focus = Focus::Button(0);
                        EventResult::Consumed(None)
                    },
                    Event::KeyEvent(Key::Tab) | Event::KeyEvent(Key::ShiftTab) => {
                        self.focus = Focus::Button(0);
                        EventResult::Consumed(None)
                    }
                    _ => EventResult::Ignored,
                },
                res => res,
            },
            // If we are on a button, we have more choice
            Focus::Button(i) => match self.buttons[i].on_event(event) {
                EventResult::Ignored => match event {
                    // Up goes back to the content
                    Event::KeyEvent(Key::Up) => {
                        if self.content.take_focus() {
                            self.focus = Focus::Content;
                            EventResult::Consumed(None)
                        } else {
                            EventResult::Ignored
                        }
                    },
                    Event::KeyEvent(Key::Tab) | Event::KeyEvent(Key::ShiftTab) => {
                        if self.content.take_focus() {
                            self.focus = Focus::Content;
                            EventResult::Consumed(None)
                        } else {
                            EventResult::Ignored
                        }
                    },
                    // Left and Right move to other buttons
                    Event::KeyEvent(Key::Right) if i+1 < self.buttons.len() => {
                        self.focus = Focus::Button(i+1);
                        EventResult::Consumed(None)
                    },
                    Event::KeyEvent(Key::Right) if i > 0 => {
                        self.focus = Focus::Button(i-1);
                        EventResult::Consumed(None)
                    },
                    _ => EventResult::Ignored,
                },
                res => res,
            },
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
