use std::io::{self, Read};
use views::Counter;

/// Wrapper around a `Read` that reports the progress made.
///
/// Used to monitor a file downloading or other slow IO task
/// in a progress bar.
pub struct ProgressReader<R: Read> {
    reader: R,
    counter: Counter,
}

impl<R: Read> ProgressReader<R> {
    /// Creates a new `ProgressReader` around `reader`.
    ///
    /// `counter` will be updated with the number of bytes read.
    ///
    /// You should make sure the progress bar knows how
    /// many bytes should be received.
    pub fn new(counter: Counter, reader: R) -> Self {
        ProgressReader {
            reader: reader,
            counter: counter,
        }
    }

    /// Unwraps this `ProgressReader`, returning the reader and counter.
    pub fn deconstruct(self) -> (R, Counter) {
        (self.reader, self.counter)
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = try!(self.reader.read(buf));
        self.counter.tick(result);
        Ok(result)
    }
}
