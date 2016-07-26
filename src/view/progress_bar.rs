use std::sync::Arc;

use std::sync::atomic::{AtomicUsize, Ordering};

use {Cursive, Printer};
use vec::Vec2;
use align::HAlign;
use direction::Orientation;
use theme::{ColorStyle, Effect};
use view::View;

pub type CbPromise = Option<Box<Fn(&mut Cursive) + Send>>;

/// Display progress.
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Arc<AtomicUsize>,
    // TODO: use a Promise instead?
    label_maker: Box<Fn(usize, (usize, usize)) -> String>,
}

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
    pub fn with_value(mut self, value: Arc<AtomicUsize>) -> Self {
        self.value = value;
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

    fn get_min_size(&mut self, size: Vec2) -> Vec2 {
        size.with_axis(Orientation::Vertical, 1)
    }
}
