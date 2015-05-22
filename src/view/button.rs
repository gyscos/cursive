use std::rc::Rc;

use color;
use ::Cursive;
use vec::Vec2;
use view::{View,ViewPath,SizeRequest};
use event::{Callback,EventResult};
use printer::Printer;

/// Simple text label with a callback when ENTER is pressed.
/// A button shows its content in a single line and has a fixed size.
pub struct Button {
    label: String,
    callback: Rc<Callback>,
}

impl Button {
    /// Creates a new button with the given content and callback.
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

    fn draw(&mut self, printer: &Printer, focused: bool) {
        let style = if !focused { color::PRIMARY } else { color::HIGHLIGHT };
        let x = printer.size.x - 1;

        let printer = printer.style(style);
        printer.print((1u32,0u32), &self.label);
        printer.print((0u32,0u32), "<");
        printer.print((x,0), ">");
    }

    fn get_min_size(&self, _: SizeRequest) -> Vec2 {
        // Meh. Fixed size we are.
        Vec2::new(2 + self.label.len() as u32, 1)
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match ch {
            // 10 is the ascii code for '\n', that is the return key
            10 => EventResult::callback(self.callback.clone()),
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self) -> bool {
        true
    }
}
