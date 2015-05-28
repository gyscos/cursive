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
    fn draw(&mut self, printer: &Printer, _: bool) {
        for (y,line) in self.history.iter().enumerate() {
            printer.print((0,y), &line);
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let line = match event {
            Event::CharEvent(c) => format!("Char: {}", c),
            Event::KeyEvent(key) => format!("Key: {}", key),
        }
        self.history.push(line);

        while self.history.len() > self.size {
            self.history.remove(0);
        }

        EventResult::Consumed(None)
    }
}

