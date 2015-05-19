use std::rc::Rc;

use ncurses;

use ::Cursive;
use vec::Vec2;
use view::{View,ViewPath,SizeRequest};
use event::{Callback,EventResult};
use printer::Printer;

/// Simple text label with a callback when ENTER is pressed.
pub struct Button {
    label: String,
    callback: Rc<Callback>,
}

impl Button {
    pub fn new<F>(label: &str, cb: F) -> Self
        where F: Fn(&mut Cursive, &ViewPath) + 'static
    {
        Button {
            label: label.to_string(),
            callback: Rc::new(Box::new(cb)),
        }
    }
}

impl View for Button {

    fn draw(&self, printer: &Printer) {
        printer.print((1u32,0u32), &self.label);
        printer.print((0u32,0u32), "<");
        printer.print((printer.size.x-1,0), ">");
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        Vec2::new(2 + self.label.len() as u32, 1)
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match ch {
            // 10 is the ascii code for '\n', that is the return key
            10 => EventResult::callback(self.callback.clone()),
            _ => EventResult::Ignored,
        }
    }
}
