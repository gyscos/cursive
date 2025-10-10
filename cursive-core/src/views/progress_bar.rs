use crate::align::HAlign;
use crate::style::{ColorStyle, ColorType, Effect, PaletteColor};
use crate::utils::Counter;
use crate::view::View;
use crate::{Printer, With};
use std::cmp;
use std::thread;

// pub type CbPromise = Option<Box<Fn(&mut Cursive) + Send>>;

/// Animated bar showing a progress value.
///
/// This bar has an internal counter, and adapts the length of the displayed
/// bar to the relative position of the counter between a minimum and maximum
/// values.
///
/// It also prints a customizable text in the center of the bar, which
/// defaults to the progression percentage.
///
/// The bar defaults to the current theme's highlight color,
/// but that can be customized.
///
/// # Example
///
/// ```
/// # use cursive_core::views::ProgressBar;
/// let bar = ProgressBar::new().with_task(|counter| {
///     // This closure is called in parallel.
///     for _ in 0..100 {
///         // Here we can communicate some
///         // advancement back to the bar.
///         counter.tick(1);
///     }
/// });
/// ```
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Counter,
    color: ColorType,
    // TODO: use a Promise instead?
    label_maker: Box<dyn Fn(usize, (usize, usize)) -> String + Send + Sync>,
}

fn make_percentage(value: usize, (min, max): (usize, usize)) -> String {
    if value < min {
        return String::from("0 %");
    }

    let (percentage, extra) = ratio(value - min, max - min, 100);
    let percentage = if extra > 4 {
        percentage + 1
    } else {
        percentage
    };
    format!("{percentage} %")
}

/// Returns length * value/max
///
/// Constraint: `value` from 0 to `max` should, as much as possible, produce equal-sized segments
/// from 0 to length.
///
/// Returns a tuple with:
/// * The integer part of the division
/// * A value between 0 and 8 (exclusive) corresponding to the remainder.
fn ratio(value: usize, max: usize, length: usize) -> (usize, usize) {
    let integer = length * value / max;
    let fraction = length * value - max * integer;

    let fraction = fraction * 8 / max;

    (integer, fraction)
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
            color: PaletteColor::Highlight.into(),
            label_maker: Box::new(make_percentage),
        }
    }

    /// Sets the value to follow.
    ///
    /// Use this to manually control the progress to display
    /// by directly modifying the value pointed to by `value`.
    #[must_use]
    pub fn with_value(self, value: Counter) -> Self {
        self.with(|s| s.set_counter(value))
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
    #[must_use]
    pub fn with_task<F: FnOnce(Counter) + Send + 'static>(self, task: F) -> Self {
        self.with(|s| s.start(task))
    }

    /// Sets the label generator.
    ///
    /// The given function will be called with `(value, (min, max))`.
    /// Its output will be used as the label to print inside the progress bar.
    ///
    /// The default one shows a percentage progress:
    ///
    /// ```
    /// fn make_progress(value: usize, (min, max): (usize, usize)) -> String {
    ///     let percent = 101 * (value - min) / (1 + max - min);
    ///     format!("{} %", percent)
    /// }
    /// ```
    #[must_use]
    pub fn with_label<F: Fn(usize, (usize, usize)) -> String + 'static + Send + Sync>(
        self,
        label_maker: F,
    ) -> Self {
        self.with(|s| s.set_label(label_maker))
    }

    /// Sets the label generator.
    ///
    /// The given function will be called with `(value, (min, max))`.
    /// Its output will be used as the label to print inside the progress bar.
    #[crate::callback_helpers]
    pub fn set_label<F: Fn(usize, (usize, usize)) -> String + 'static + Send + Sync>(
        &mut self,
        label_maker: F,
    ) {
        self.label_maker = Box::new(label_maker);
    }

    /// Sets the minimum value.
    ///
    /// When `value` equals `min`, the bar is at the minimum level.
    ///
    /// If `self.min > max`, `self.min` is set to `max`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn min(self, min: usize) -> Self {
        self.with(|s| s.set_min(min))
    }

    /// Sets the minimum value.
    ///
    /// When `value` equals `min`, the bar is at the minimum level.
    ///
    /// If `self.min > max`, `self.min` is set to `max`.
    pub fn set_min(&mut self, min: usize) {
        self.min = min;
        self.max = cmp::max(self.max, self.min);
    }

    /// Sets the maximum value.
    ///
    /// When `value` equals `max`, the bar is at the maximum level.
    ///
    /// If `min > self.max`, `self.max` is set to `min`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn max(self, max: usize) -> Self {
        self.with(|s| s.set_max(max))
    }

    /// Sets the maximum value.
    ///
    /// When `value` equals `max`, the bar is at the maximum level.
    ///
    /// If `min > self.max`, `self.max` is set to `min`.
    pub fn set_max(&mut self, max: usize) {
        self.max = max;
        self.min = cmp::min(self.min, self.max);
    }

    /// Sets the `min` and `max` range for the value.
    ///
    /// If `min > max`, swap the two values.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn range(self, min: usize, max: usize) -> Self {
        self.with(|s| s.set_range(min, max))
    }

    /// Sets the `min` and `max` range for the value.
    ///
    /// If `min > max`, swap the two values.
    pub fn set_range(&mut self, min: usize, max: usize) {
        if min > max {
            self.set_min(max);
            self.set_max(min);
        } else {
            self.set_min(min);
            self.set_max(max);
        }
    }

    /// Sets the current value.
    ///
    /// Value is clamped between `min` and `max`.
    pub fn set_value(&mut self, value: usize) {
        self.value.set(value);
    }

    /// Sets the value to follow.
    ///
    /// Use this to manually control the progress to display
    /// by directly modifying the value pointed to by `value`.
    pub fn set_counter(&mut self, value: Counter) {
        self.value = value;
    }

    /// Sets the color style.
    ///
    /// The default color is `PaletteColor::Highlight`.
    pub fn set_color<C>(&mut self, color: C)
    where
        C: Into<ColorType>,
    {
        self.color = color.into();
    }

    /// Sets the color style.
    ///
    /// Chainable variant of `set_color`.
    #[must_use]
    pub fn with_color<C>(self, color: C) -> Self
    where
        C: Into<ColorType>,
    {
        self.with(|s| s.set_color(color))
    }
}

fn sub_block(extra: usize) -> &'static str {
    match extra {
        0 => " ",
        1 => "▏",
        2 => "▎",
        3 => "▍",
        4 => "▌",
        5 => "▋",
        6 => "▊",
        7 => "▉",
        _ => "█",
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
        let (length, extra) = if value < self.min {
            (0, 0)
        } else {
            ratio(value - self.min, self.max - self.min, available)
        };

        let label = (self.label_maker)(value, (self.min, self.max));
        let offset = HAlign::Center.get_offset(label.len(), printer.size.x);

        let color_style = ColorStyle::new(PaletteColor::HighlightText, self.color);

        printer.with_color(color_style, |printer| {
            // TODO: Instead, write it with self.color and inherit_parent background?
            // Draw the right half of the label in reverse
            printer.with_effect(Effect::Reverse, |printer| {
                printer.print((length, 0), sub_block(extra));
                printer.print((offset, 0), &label);
            });
            let printer = &printer.cropped((length, 1));
            printer.print_hline((0, 0), length, " ");

            // Draw the left part in color_style (it may be cropped)
            printer.print((offset, 0), &label);
        });
    }
}

#[crate::blueprint(ProgressBar::new())]
struct Blueprint {
    min: Option<usize>,
    max: Option<usize>,
    value: Option<usize>,
    color: Option<ColorType>,
    label: Option<_>,
}
