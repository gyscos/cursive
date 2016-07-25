use std::sync::{Arc, Mutex};

use std::sync::atomic::{AtomicUsize, Ordering};

use {Cursive, Printer};
use event::*;
use theme::ColorStyle;
use view::View;

/// Display progress.
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Arc<AtomicUsize>,
    // TODO: use a Promise instead?
    callback: Option<Arc<Mutex<Option<Box<Fn(&mut Cursive) + Send>>>>>,
}

impl ProgressBar {
    /// Creates a new progress bar.
    ///
    /// Default values:
    ///
    /// * `min`: 0
    /// * `max`: 100
    /// * `value`: 0
    pub fn new() -> Self {
        ProgressBar {
            min: 0,
            max: 100,
            value: Arc::new(AtomicUsize::new(0)),
            callback: None,
        }
    }

    /// Sets the value to follow.
    pub fn with_value(mut self, value: Arc<AtomicUsize>) -> Self {
        self.value = value;
        self
    }

    /// Sets the callback to follow.
    ///
    /// Whenever `callback` is set, it will be called on the next event loop.
    pub fn with_callback(mut self,
                         callback: Arc<Mutex<Option<Box<Fn(&mut Cursive) + Send>>>>)
                         -> Self {
        self.callback = Some(callback);
        self
    }

    /// Sets the minimum value.
    ///
    /// When `value` equals `min`, the bar is at the minimum level.
    pub fn min(mut self, min: usize) -> Self {
        self.min = min;
        self
    }

    /// Sets the maximum value.
    ///
    /// When `value` equals `max`, the bar is at the maximum level.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Sets the `min` and `max` range for the value.
    pub fn range(self, min: usize, max: usize) -> Self {
        self.min(min).max(max)
    }

    /// Sets the current value.
    ///
    /// Value is clamped between `min` and `max`.
    pub fn set_value(&mut self, value: usize) {
        self.value.store(value, Ordering::Relaxed);
    }
}

impl View for ProgressBar {
    fn draw(&self, printer: &Printer) {
        // Now, the bar itself...
        let available = printer.size.x;

        let value = self.value.load(Ordering::Relaxed);
        let length = (available * (value - self.min)) / (self.max - self.min);
        printer.with_color(ColorStyle::Highlight, |printer| {
            printer.print_hline((0, 0), length, " ");
        });
    }

    fn on_event(&mut self, _: Event) -> EventResult {
        if let Some(ref cb) = self.callback {
            if let Some(cb) = cb.lock().unwrap().take() {
                return EventResult::Consumed(Some(cb.into()));
            }
        }

        EventResult::Ignored
    }
}
