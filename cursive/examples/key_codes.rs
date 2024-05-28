use cursive::event::{Event, EventResult};
use cursive::traits::*;
use cursive::Printer;

// This example define a custom view that prints any event it receives.
// This is a handy way to check the input received by cursive.

fn main() {
    let mut siv = cursive::default();
    siv.add_layer(KeyCodeView::new(10).full_width().fixed_height(10));

    siv.run();
}

// Our view will have a small history of the last events.
struct KeyCodeView {
    history: Vec<String>,
    size: usize,
}

impl KeyCodeView {
    fn new(size: usize) -> Self {
        KeyCodeView {
            history: Vec::new(),
            size,
        }
    }
}

// Let's implement the `View` trait.
// `View` contains many methods, but only a few are required.
impl View for KeyCodeView {
    fn draw(&self, printer: &Printer) {
        // We simply draw every event from the history.
        for (y, line) in self.history.iter().enumerate() {
            printer.print((0, y), line);
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // Each line will be a debug-format of the event.
        let line = format!("{event:?}");
        self.history.push(line);

        // Keep a fixed-sized history.
        while self.history.len() > self.size {
            self.history.remove(0);
        }

        // No need to return any callback.
        EventResult::Consumed(None)
    }
}
