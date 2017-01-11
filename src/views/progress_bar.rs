

use Printer;
use align::HAlign;
use std::cmp;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use std::thread;
use theme::{ColorStyle, Effect};
use view::View;

// pub type CbPromise = Option<Box<Fn(&mut Cursive) + Send>>;

/// Atomic counter used by `ProgressBar`.
#[derive(Clone)]
pub struct Counter(pub Arc<AtomicUsize>);

impl Counter {
    /// Creates a new `Counter` starting with the given value.
    pub fn new(value: usize) -> Self {
        Counter(Arc::new(AtomicUsize::new(value)))
    }

    /// Retrieves the current progress value.
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    /// Sets the current progress value.
    pub fn set(&self, value: usize) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// Increase the current progress by `ticks`.
    pub fn tick(&self, ticks: usize) {
        self.0.fetch_add(ticks, Ordering::Relaxed);
    }
}

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
/// # use cursive::views::ProgressBar;
/// let bar = ProgressBar::new()
///                       .with_task(|counter| {
///                           // This closure is called in parallel.
///                           for _ in 0..100 {
///                               // Here we can communicate some
///                               // advancement back to the bar.
///                               counter.tick(1);
///                           }
///                       });
/// ```
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Counter,
    // TODO: use a Promise instead?
    label_maker: Box<Fn(usize, (usize, usize)) -> String>,
}


fn make_percentage(value: usize, (min, max): (usize, usize)) -> String {
    if value < min {
        // ?? Negative progress?
        let percent = 101 * (min - value) / (1 + max - min);
        format!("-{} %", percent)
    } else {
        let percent = 101 * (value - min) / (1 + max - min);
        format!("{} %", percent)
    }
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
            value: Counter::new(0),
            label_maker: Box::new(make_percentage),
        }
    }

    /// Sets the value to follow.
    ///
    /// Use this to manually control the progress to display
    /// by directly modifying the value pointed to by `value`.
    pub fn with_value(mut self, value: Counter) -> Self {
        self.value = value;
        self
    }

    /// Starts a function in a separate thread, and monitor the progress.
    ///
    /// `f` will be given a `Counter` to increment the bar's progress.
    ///
    /// This does not reset the value, so it can be called several times
    /// to advance the progress in multiple sessions.
    pub fn start<F: FnOnce(Counter) + Send + 'static>(&mut self, f: F) {
        let counter: Counter = self.value.clone();

        thread::spawn(move || {
            f(counter);
        });
    }

    /// Starts a function in a separate thread, and monitor the progress.
    ///
    /// Chainable variant.
    pub fn with_task<F: FnOnce(Counter) + Send + 'static>(mut self, task: F)
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
    ///
    /// If `self.min > max`, `self.min` is set to `max`.
    pub fn min(mut self, min: usize) -> Self {
        self.min = min;
        self.max = cmp::max(self.max, self.min);

        self
    }

    /// Sets the maximum value.
    ///
    /// When `value` equals `max`, the bar is at the maximum level.
    ///
    /// If `min > self.max`, `self.max` is set to `min`.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self.min = cmp::min(self.min, self.max);

        self
    }

    /// Sets the `min` and `max` range for the value.
    ///
    /// If `min > max`, swap the two values.
    pub fn range(self, min: usize, max: usize) -> Self {
        if min > max {
            self.min(max).max(min)
        } else {
            self.min(min).max(max)
        }
    }

    /// Sets the current value.
    ///
    /// Value is clamped between `min` and `max`.
    pub fn set_value(&mut self, value: usize) {
        self.value.set(value);
    }
}

impl View for ProgressBar {
    fn draw(&self, printer: &Printer) {
        // Now, the bar itself...
        let available = printer.size.x;

        let value = self.value.get();

        // If we're under the minimum, don't draw anything.
        // If we're over the maximum, we'll try to draw more, but the printer
        // will crop us anyway, so it's not a big deal.
        let length = if value < self.min {
            0
        } else {
            ((1 + available) * (value - self.min)) / (1 + self.max - self.min)
        };

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
