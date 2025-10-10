use crossbeam_channel::{unbounded, Receiver, Sender};
use cursive::traits::*;
use cursive::Vec2;
use cursive::{Cursive, Printer};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

// This example will print a stream of logs generated from a separate thread.
//
// We will use a custom view using a channel to receive data asynchronously.

fn main() {
    // As usual, create the Cursive root
    let mut siv = cursive::default();

    let cb_sink = siv.cb_sink().clone();

    // We want to refresh the page even when no input is given.
    siv.add_global_callback('q', |s| s.quit());

    // A channel will communicate data from our running task to the UI.
    let (tx, rx) = unbounded();

    // Generate data in a separate thread.
    thread::spawn(move || {
        generate_logs(&tx, cb_sink);
    });

    // And sets the view to read from the other end of the channel.
    siv.add_layer(BufferView::new(200, rx).full_screen());

    siv.run();
}

// We will only simulate log generation here.
// In real life, this may come from a running task, a separate process, ...
fn generate_logs(tx: &Sender<String>, cb_sink: cursive::CbSink) {
    let mut i = 1;
    loop {
        let line = format!("Interesting log line {i}");
        i += 1;
        // The send will fail when the other side is dropped.
        // (When the application ends).
        if tx.send(line).is_err() {
            return;
        }
        cb_sink.send(Box::new(Cursive::noop)).unwrap();
        thread::sleep(Duration::from_millis(30));
    }
}

// Let's define a buffer view, that shows the last lines from a stream.
struct BufferView {
    // We'll use a ring buffer
    buffer: VecDeque<String>,
    // Receiving end of the stream
    rx: Receiver<String>,
}

impl BufferView {
    // Creates a new view with the given buffer size
    fn new(size: usize, rx: Receiver<String>) -> Self {
        let mut buffer = VecDeque::new();
        buffer.resize(size, String::new());
        BufferView { buffer, rx }
    }

    // Reads available data from the stream into the buffer
    fn update(&mut self) {
        // Add each available line to the end of the buffer.
        while let Ok(line) = self.rx.try_recv() {
            self.buffer.push_back(line);
            self.buffer.pop_front();
        }
    }
}

impl View for BufferView {
    fn layout(&mut self, _: Vec2) {
        // Before drawing, we'll want to update the buffer
        self.update();
    }

    fn draw(&self, printer: &Printer) {
        // Print the end of the buffer
        for (i, line) in self.buffer.iter().rev().take(printer.size.y).enumerate() {
            printer.print((0, printer.size.y - 1 - i), line);
        }
    }
}
