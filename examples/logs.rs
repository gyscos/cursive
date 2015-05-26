extern crate cursive;

use std::sync::mpsc;
use std::thread;

use cursive::Cursive;
use cursive::printer::Printer;
use cursive::view::{View,FullView};

fn main() {
    // As usual, create the Cursive root
    let mut siv = Cursive::new();

    // We want to refresh the page even when no input is given.
    siv.set_fps(10);
    siv.add_global_callback('q' as i32, |s| s.quit());

    // A channel will communicate data from our running task to the UI.
    let (tx,rx) = mpsc::channel();

    // Generate data in a separate thread.
    thread::spawn(|| { generate_logs(tx); });

    // And sets the view to read from the other end of the channel.
    // (We use FullView to force fullscreen because
    // we have no min_size for the BufferView).
    siv.add_layer(FullView::new(BufferView::new(200, rx)));

    siv.run();
}

// We will only simulate log generation here.
// In real life, this may come from a running task, a separate process, ...
fn generate_logs(tx: mpsc::Sender<String>) {
    let mut i = 1;
    loop {
        let line = format!("Interesting log line {}", i);
        i += 1;
        // The send will fail when the other side is dropped.
        // (When the application ends).
        match tx.send(line) {
            Err(_) => return,
            Ok(_) => (),
        }
        thread::sleep_ms(30);
    }
}

// Let's define a buffer view, that shows the last lines from a stream.
struct BufferView {
    // We will emulate a ring buffer
    buffer: Vec<String>,
    // Current position in the buffer
    pos: usize,
    // Receiving end of the stream
    rx: mpsc::Receiver<String>,
}

impl BufferView {
    // Creates a new view with the given buffer size
    fn new(size: usize, rx: mpsc::Receiver<String>) -> Self {
        BufferView {
            rx: rx,
            buffer: (0..size).map(|_| String::new()).collect(),
            pos: 0,
        }
    }

    // Reads available data from the stream into the buffer
    fn update(&mut self) {
        let mut i = self.pos;
        while let Ok(line) = self.rx.try_recv() {
            self.buffer[i] = line;
            i = (i+1) % self.buffer.len();
        }
        self.pos = i;
    }

    // Chain together the two parts of the buffer to appear as a circular one.
    // The signature is quite ugly, but basically we return an iterator:
    // a Chain of two slice iterators.
    fn ring<'a>(&'a self) -> std::iter::Chain<std::slice::Iter<'a,String>, std::slice::Iter<'a,String>> {
        // The main buffer is "circular" starting at self.pos
        // So we chain the two parts as one
        self.buffer[self.pos..].iter().chain(self.buffer[..self.pos].iter())
    }
}

impl View for BufferView {
    fn draw(&mut self, printer: &Printer, _: bool) {
        // Before drawing, we'll want to update the buffer
        self.update();

        // If the buffer is large enough, we'll discard the beginning and keep the end.
        // If the buffer is too small, only print a part of it with an offset.
        let (discard,offset) = if self.buffer.len() > printer.size.y as usize {
            (self.buffer.len() - printer.size.y as usize, 0)
        } else {
            (0, printer.size.y - self.buffer.len())
        };

        for (i, line) in self.ring().skip(discard).enumerate() {
            printer.print((0,offset + i), line);
        }
    }
}
