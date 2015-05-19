use std::cmp::max;

use ncurses;

use ::{Cursive,Margins};
use event::EventResult;
use view::{View,ViewPath,SizeRequest,DimensionRequest};
use view::{Button,SizedView};
use vec::Vec2;
use printer::Printer;

enum Focus {
    Content,
    Button(usize),
    Nothing,
}

pub struct Dialog {
    content: Box<View>,

    buttons: Vec<SizedView<Button>>,

    padding: Margins,
    borders: Margins,

    focus: Focus,
}

impl Dialog {
    pub fn new<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: Box::new(view),
            buttons: Vec::new(),
            focus: Focus::Nothing,
            padding: Margins::new(1,1,0,0),
            borders: Margins::new(1,1,1,1),
        }
    }

    pub fn button<'a, F>(mut self, label: &'a str, cb: F) -> Self
        where F: Fn(&mut Cursive, &ViewPath) + 'static
    {
        self.buttons.push(SizedView::new(Button::new(label, cb)));

        self
    }

}

fn offset_request(request: DimensionRequest, offset: i32) -> DimensionRequest {
    match request {
        DimensionRequest::Fixed(w) => DimensionRequest::Fixed((w as i32 + offset) as u32),
        DimensionRequest::AtMost(w) => DimensionRequest::AtMost((w as i32 + offset) as u32),
        DimensionRequest::Unknown => DimensionRequest::Unknown,
    }
}

impl View for Dialog {
    fn draw(&self, printer: &Printer) {

        let mut height = 0;
        let mut x = 0;
        for button in self.buttons.iter().rev() {
            // button.draw(&printer.sub_printer(), 
            let size = button.size;
            let offset = printer.size - self.borders.bot_right() - self.padding.bot_right() - size - Vec2::new(x, 0);
            button.draw(&printer.sub_printer(offset, size));
            x += size.x + 1;
            height = max(height, size.y+1);
        }

        let inner_size = printer.size
            - Vec2::new(0, height)
            - self.borders.combined()
            - self.padding.combined();

        self.content.draw(&printer.sub_printer(self.borders.top_left() + self.padding.top_left(), inner_size));

        printer.print(Vec2::new(0,0), "+");
        printer.print(Vec2::new(printer.size.x-1, 0), "+");
        printer.print(Vec2::new(0, printer.size.y-1), "+");
        printer.print(Vec2::new(printer.size.x-1, printer.size.y-1), "+");
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
                    _ => EventResult::Ignored,
                },
                res => res,
            },
            Focus::Nothing => EventResult::Ignored,
        }
    }
}
