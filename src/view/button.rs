use std::rc::Rc;

use theme::ColorStyle;
use Cursive;
use vec::Vec2;
use view::{SizeRequest, View};
use event::*;
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
        where F: Fn(&mut Cursive) + 'static
    {
        Button {
            label: label.to_string(),
            callback: Rc::new(Box::new(cb)),
        }
    }
}

impl View for Button {
    fn draw(&mut self, printer: &Printer) {
        let style = if !printer.focused {
            ColorStyle::Primary
        } else {
            ColorStyle::Highlight
        };
        let x = printer.size.x - 1;

        printer.with_color(style, |printer| {
            printer.print((1, 0), &self.label);
            printer.print((0, 0), "<");
            printer.print((x, 0), ">");
        });
    }

    fn get_min_size(&self, _: SizeRequest) -> Vec2 {
        // Meh. Fixed size we are.
        Vec2::new(2 + self.label.chars().count(), 1)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            // 10 is the ascii code for '\n', that is the return key
            Event::Key(Key::Enter) => EventResult::Consumed(Some(self.callback.clone())),
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self) -> bool {
        true
    }
}
