use crate::utils::Counter;
use std::io::{self, Read};

/// Wrapper around a `Read` that reports the progress made.
///
/// Used to monitor a file downloading or other slow IO task
/// in a progress bar.
///
/// # Examples
///
/// ```rust,no_run
/// use std::io::Read;
/// use cursive::utils::{Counter, ProgressReader};
///
/// // Read a file and report the progress
/// let file = std::fs::File::open("large_file").unwrap();
/// let counter = Counter::new(0);
/// let mut reader = ProgressReader::new(counter.clone(), file);
///
/// std::thread::spawn(move || {
///     // Left as an exercise: use an AtomicBool for a stop condition!
///     loop {
///         let progress = counter.get();
///         println!("Read {} bytes so far", progress);
///     }
/// });
///
/// // As we read data, the counter will be updated and the control thread
/// // will monitor the progress.
/// let mut buffer = Vec::new();
/// reader.read_to_end(&mut buffer).unwrap();
/// ```
#[derive(Clone, Debug)]
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
        ProgressReader { reader, counter }
    }

    /// Unwraps this `ProgressReader`, returning the reader and counter.
    pub fn deconstruct(self) -> (R, Counter) {
        (self.reader, self.counter)
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.reader.read(buf)?;
        self.counter.tick(result);
        Ok(result)
    }
}
