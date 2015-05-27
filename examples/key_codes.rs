extern crate cursive;

use cursive::Cursive;

use cursive::view::{View,BoxView};
use cursive::printer::Printer;
use cursive::event::EventResult;

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(BoxView::new((10,4), KeyCodeView::new(4)));

    siv.run();
}

struct KeyCodeView {
    history: Vec<i32>,
    size: usize,
}

impl KeyCodeView {
    fn new(size: usize) -> Self {
        KeyCodeView {
            history: Vec::new(),
            size: size,
        }
    }
}

impl View for KeyCodeView {
    fn draw(&mut self, printer: &Printer, _: bool) {
        for (y,n) in self.history.iter().enumerate() {
            printer.print((0,y), &format!("{}", n));
        }
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        self.history.push(ch);

        while self.history.len() > self.size {
            self.history.remove(0);
        }

        EventResult::Consumed(None)
    }
}

