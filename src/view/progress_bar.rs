use std::sync::Arc;

use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

use {Cursive, Printer};
use align::HAlign;
use theme::{ColorStyle, Effect};
use view::View;

pub type CbPromise = Option<Box<Fn(&mut Cursive) + Send>>;

/// Animated bar showing a progress value.
///
/// This bar has an internal counter, and adapts the length of the displayed
/// bar to the relative position of the counter between a minimum and maximum
/// values.
///
/// It also prints a customizable text in the center of the bar, which
/// defaults to the progression percentage.
///
/// # Example
///
/// ```
/// # use cursive::prelude::*;
/// let bar = ProgressBar::new()
///                       .with_task(|ticker| {
///                           // This closure is called in parallel.
///                           for _ in 0..100 {
///                               // Here we can communicate some
///                               // advancement back to the bar.
///                               ticker(1);
///                           }
///                       });
/// ```
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Arc<AtomicUsize>,
    // TODO: use a Promise instead?
    label_maker: Box<Fn(usize, (usize, usize)) -> String>,
}

pub type Ticker = Box<Fn(usize) + Send>;

fn make_percentage(value: usize, (min, max): (usize, usize)) -> String {
    let percent = 101 * (value - min) / (1 + max - min);
    format!("{} %", percent)
}

new_default!(ProgressBar);

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
            label_maker: Box::new(make_percentage),
        }
    }

    /// Sets the value to follow.
    ///
    /// Use this to manually control the progress to display
    /// by directly modifying the value pointed to by `value`.
    pub fn with_value(mut self, value: Arc<AtomicUsize>) -> Self {
        self.value = value;
        self
    }

    /// Starts a function in a separate thread, and monitor the progress.
    ///
    /// `f` will be given a `Ticker` to increment the bar's progress.
    ///
    /// This does not reset the value, so it can be called several times
    /// to advance the progress in multiple sessions.
    pub fn start<F: FnOnce(Ticker) + Send + 'static>(&mut self, f: F) {
        let value = self.value.clone();
        let ticker: Ticker = Box::new(move |ticks| {
            value.fetch_add(ticks, Ordering::Relaxed);
        });

        thread::spawn(move || {
            f(ticker);
        });
    }

    /// Starts a function in a separate thread, and monitor the progress.
    ///
    /// Chainable variant.
    pub fn with_task<F: FnOnce(Ticker) + Send + 'static>(mut self, task: F)
                                                         -> Self {
        self.start(task);
        self
    }

    /// Sets the label generator.
    ///
    /// The given function will be called with `(value, (min, max))`.
    /// Its output will be used as the label to print inside the progress bar.
    ///
    /// The default one shows a percentage progress:
    ///
    /// ```
    /// fn make_percentage(value: usize, (min, max): (usize, usize)) -> String {
    ///     let percent = 101 * (value - min) / (1 + max - min);
    ///     format!("{} %", percent)
    /// }
    /// ```
    pub fn with_label<F: Fn(usize, (usize, usize)) -> String + 'static>
        (mut self, label_maker: F)
         -> Self {
        self.label_maker = Box::new(label_maker);
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
        let length = ((1 + available) * (value - self.min)) /
                     (1 + self.max - self.min);

        let label = (self.label_maker)(value, (self.min, self.max));
        let offset = HAlign::Center.get_offset(label.len(), printer.size.x);

        printer.with_color(ColorStyle::Highlight, |printer| {
            printer.with_effect(Effect::Reverse, |printer| {
                printer.print((offset, 0), &label);
            });
            let printer = &printer.sub_printer((0, 0), (length, 1), true);
            printer.print_hline((0, 0), length, " ");
            printer.print((offset, 0), &label);
        });
    }
}
