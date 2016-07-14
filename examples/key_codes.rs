extern crate cursive;

use cursive::Cursive;

use cursive::view::{View, BoxView};
use cursive::Printer;
use cursive::event::{EventResult, Event};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(BoxView::fixed_size((30, 10), KeyCodeView::new(10)));

    siv.run();
}

struct KeyCodeView {
    history: Vec<String>,
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
    fn draw(&mut self, printer: &Printer) {
        for (y, line) in self.history.iter().enumerate() {
            printer.print((0, y), &line);
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let line = format!("{:?}", event);
        self.history.push(line);

        while self.history.len() > self.size {
            self.history.remove(0);
        }

        EventResult::Consumed(None)
    }
}
