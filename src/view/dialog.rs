use std::cmp::max;

use ncurses;

use ::{Cursive,Margins};
use event::EventResult;
use view::{View,ViewPath,SizeRequest,DimensionRequest};
use view::{Button,SizedView};
use vec::Vec2;
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
    content: Box<View>,

    buttons: Vec<SizedView<Button>>,

    padding: Margins,
    borders: Margins,

    focus: Focus,
}

impl Dialog {
    /// Creates a new Dialog with the given content.
    pub fn new<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: Box::new(view),
            buttons: Vec::new(),
            focus: Focus::Content,
            padding: Margins::new(1,1,0,0),
            borders: Margins::new(1,1,1,1),
        }
    }

    /// Adds a button to the dialog with the given label and callback.
    ///
    /// Consumes and returns self for easy chaining.
    pub fn button<'a, F>(mut self, label: &'a str, cb: F) -> Self
        where F: Fn(&mut Cursive, &ViewPath) + 'static
    {
        self.buttons.push(SizedView::new(Button::new(label, cb)));

        self
    }

}

impl View for Dialog {
    fn draw(&self, printer: &Printer, focused: bool) {

        let mut height = 0;
        let mut x = 0;
        for (i,button) in self.buttons.iter().enumerate().rev() {
            let size = button.size;
            let offset = printer.size - self.borders.bot_right() - self.padding.bot_right() - size - Vec2::new(x, 0);
            // Add some special effect to the focused button
            button.draw(&printer.sub_printer(offset, size), focused && (self.focus == Focus::Button(i)));
            x += size.x + 1;
            height = max(height, size.y+1);
        }

        let inner_size = printer.size
            - Vec2::new(0, height)
            - self.borders.combined()
            - self.padding.combined();

        self.content.draw(&printer.sub_printer(self.borders.top_left() + self.padding.top_left(), inner_size), focused && self.focus == Focus::Content);

        printer.print_box(Vec2::new(0,0), printer.size, '+', '-', '|');

    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        let content_req = req.reduced(self.padding.combined() + self.borders.combined());
        let content_size = self.content.get_min_size(content_req);

        let mut buttons_size = Vec2::new(0,0);
        for button in self.buttons.iter() {
            let s = button.view.get_min_size(req);
            buttons_size.x += s.x + 1;
            buttons_size.y = max(buttons_size.y, s.y + 1);
        }

        let inner_size = Vec2::new(
            max(content_size.x, buttons_size.x),
            content_size.y + buttons_size.y);

        inner_size + self.padding.combined() + self.borders.combined()
    }

    fn layout(&mut self, mut size: Vec2) {
        // First layout the buttons
        size = size - (self.borders.combined() + self.padding.combined());
        let req = SizeRequest {
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };

        let mut buttons_height = 0;
        for button in self.buttons.iter_mut().rev() {
            let size = button.get_min_size(req);
            buttons_height = max(buttons_height, size.y+1);
            button.layout(size);
        }

        self.content.layout(size - Vec2::new(0, buttons_height));
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match self.focus {
            Focus::Content => match self.content.on_key_event(ch) {
                EventResult::Ignored if !self.buttons.is_empty() => match ch {
                    ncurses::KEY_DOWN => {
                        self.focus = Focus::Button(0);
                        EventResult::Consumed(None, ViewPath::new())
                    },
                    _ => EventResult::Ignored,
                },
                res => res,
            },
            Focus::Button(i) => match self.buttons[i].on_key_event(ch) {
                EventResult::Ignored => match ch {
                    ncurses::KEY_UP => {
                        if self.content.take_focus() {
                            self.focus = Focus::Content;
                            EventResult::consume()
                        } else {
                            EventResult::Ignored
                        }
                    },
                    ncurses::KEY_RIGHT if i+1 < self.buttons.len() => {
                        self.focus = Focus::Button(i+1);
                        EventResult::consume()
                    },
                    ncurses::KEY_LEFT if i > 0 => {
                        self.focus = Focus::Button(i-1);
                        EventResult::consume()
                    },
                    _ => EventResult::Ignored,
                },
                res => res,
            },
        }
    }

    fn take_focus(&mut self) -> bool {
        if !self.buttons.is_empty() {
            self.focus = Focus::Button(0);
            true
        } else {
            self.focus = Focus::Content;
            self.content.take_focus()
        }
    }
}
