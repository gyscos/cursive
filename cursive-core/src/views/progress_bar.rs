use crate::align::HAlign;
use crate::style::chart::{AXIS, Fill};
use crate::style::{ColorType, PaletteColor};
use crate::utils::Counter;
use crate::view::View;
use crate::views::bar::{Bar, BarStyle, LabelPosition};
use crate::{Printer, With};
use std::cmp;
use std::thread;
use unicode_width::UnicodeWidthStr;

// pub type CbPromise = Option<Box<Fn(&mut Cursive) + Send>>;

/// Animated bar showing a progress value.
///
/// This bar has an internal counter, and adapts the length of the displayed
/// bar to the relative position of the counter between a minimum and maximum
/// values.
///
/// It also prints a customizable text, which defaults to the progression
/// percentage. By default it is in the center of the bar.
///
/// The bar defaults to the current theme's highlight color,
/// but that can be customized with [`color`](Self::set_color) or with a
/// [`Fill`].
///
/// # Styles
///
/// The bar is a solid colored bar by default. Use [`style`](Self::set_style)
/// for other looks. With [`BarStyle::Line`] and [`BarStyle::Block`], the label
/// is drawn to the right of the bar by default, and the unfilled part shows a
/// [track](Self::set_track).
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
///
/// A thin bar, green and then red as it fills up:
///
/// ```
/// use cursive_core::style::BaseColor;
/// use cursive_core::style::chart::Fill;
/// use cursive_core::views::{BarStyle, ProgressBar};
///
/// let bar = ProgressBar::new().style(BarStyle::Line).fill(Fill::steps([
///     (0.0, BaseColor::Green.dark()),
///     (0.8, BaseColor::Red.dark()),
/// ]));
/// ```
pub struct ProgressBar {
    min: usize,
    max: usize,
    value: Counter,
    fill: Fill,
    track: ColorType,
    style: BarStyle,
    // `None` for the default of the style.
    label_position: Option<LabelPosition>,
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
/// If `max` is 0, the result is a full bar.
///
/// Constraint: `value` from 0 to `max` should, as much as possible, produce equal-sized segments
/// from 0 to length.
///
/// Returns a tuple with:
/// * The integer part of the division
/// * A value between 0 and 8 (exclusive) corresponding to the remainder.
fn ratio(value: usize, max: usize, length: usize) -> (usize, usize) {
    if max == 0 {
        return (length, 0);
    }
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
            fill: Fill::solid(PaletteColor::Highlight),
            track: ColorType::from(&AXIS),
            style: BarStyle::Solid,
            label_position: None,
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

    /// Sets the color of the filled part of the bar.
    ///
    /// This is the same as setting a solid [`Fill`]: the last call to this or
    /// to [`set_fill`](Self::set_fill) wins.
    ///
    /// The default color is `PaletteColor::Highlight`.
    pub fn set_color<C>(&mut self, color: C)
    where
        C: Into<ColorType>,
    {
        self.fill = Fill::solid(color);
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

    /// Sets how the bar is drawn.
    ///
    /// The default is [`BarStyle::Solid`].
    ///
    /// If the label position was not set, it depends on the style.
    pub fn set_style(&mut self, style: BarStyle) {
        self.style = style;
    }

    /// Sets how the bar is drawn.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn style(self, style: BarStyle) -> Self {
        self.with(|s| s.set_style(style))
    }

    /// Sets where the label is drawn.
    ///
    /// By default it is [`LabelPosition::Inside`] for [`BarStyle::Solid`], and
    /// [`LabelPosition::Right`] for the other styles. Calling this overrides
    /// the default, even if the style is changed afterwards.
    pub fn set_label_position(&mut self, position: LabelPosition) {
        self.label_position = Some(position);
    }

    /// Sets where the label is drawn.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn label_position(self, position: LabelPosition) -> Self {
        self.with(|s| s.set_label_position(position))
    }

    /// Sets the color of the unfilled part of the bar.
    ///
    /// This is used by [`BarStyle::Line`] and [`BarStyle::Block`]. A solid bar
    /// ignores it.
    ///
    /// The default is the [chart axis color](AXIS).
    pub fn set_track<C>(&mut self, track: C)
    where
        C: Into<ColorType>,
    {
        self.track = track.into();
    }

    /// Sets the color of the unfilled part of the bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn track<C>(self, track: C) -> Self
    where
        C: Into<ColorType>,
    {
        self.with(|s| s.set_track(track))
    }

    /// Sets the fill used to color the filled part of the bar.
    ///
    /// The fill sees the whole bar as its area, and the completion ratio
    /// (from 0 to 1) as its value: [`Fill::steps`] colors the bar according to
    /// how complete it is, and [`Fill::horizontal`] gives a gradient along
    /// the bar.
    ///
    /// The last call to this or to [`set_color`](Self::set_color) wins.
    pub fn set_fill(&mut self, fill: impl Into<Fill>) {
        self.fill = fill.into();
    }

    /// Sets the fill used to color the filled part of the bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn fill(self, fill: impl Into<Fill>) -> Self {
        self.with(|s| s.set_fill(fill))
    }

    fn effective_label_position(&self) -> LabelPosition {
        self.label_position.unwrap_or(match self.style {
            BarStyle::Solid => LabelPosition::Inside,
            BarStyle::Line | BarStyle::Block => LabelPosition::Right,
        })
    }
}

impl View for ProgressBar {
    fn layout_key(&self) -> u64 {
        // Always the same size.
        crate::view::layout_key_seed::<Self>()
    }

    fn needs_relayout(&self) -> bool {
        // Same reason: nothing here can change the size.
        false
    }

    fn draw(&self, printer: &Printer) {
        let width = printer.size.x;
        if width == 0 || printer.size.y == 0 {
            return;
        }

        let value = self.value.get();
        let label = (self.label_maker)(value, (self.min, self.max));
        let position = self.effective_label_position();

        // With a label on the right, the bar gets what is left. The room reserved for the label
        // does not depend on the current value, or the bar would change size as the label does.
        let label_width = match position {
            LabelPosition::Right => [self.min, self.max, value]
                .into_iter()
                .map(|value| (self.label_maker)(value, (self.min, self.max)).width())
                .max()
                .unwrap_or(0)
                .min(width),
            _ => 0,
        };
        let bar_width = match position {
            LabelPosition::Right => width.saturating_sub(label_width + 1),
            _ => width,
        };

        // If we're under the minimum, don't draw anything.
        // If we're over the maximum, we'll try to draw more, but the printer
        // will crop us anyway, so it's not a big deal.
        let (length, extra) = if value < self.min {
            (0, 0)
        } else {
            ratio(value - self.min, self.max - self.min, bar_width)
        };
        let completion = if value < self.min {
            0.0
        } else if self.max == self.min {
            1.0
        } else {
            ((value - self.min) as f64 / (self.max - self.min) as f64).min(1.0) as f32
        };

        let inside = match (position, self.style) {
            (LabelPosition::Inside, BarStyle::Solid) => Some(label.as_str()),
            _ => None,
        };
        Bar {
            style: self.style,
            eighths: length * 8 + extra,
            ratio: completion,
            fill: &self.fill,
            track: self.track,
            label: inside,
        }
        .draw(&printer.cropped((bar_width, 1)));

        match (position, self.style) {
            (LabelPosition::Right, _) => {
                // Right-aligned in the reserved room, so that the last character stays in place.
                // When the label is cropped, keep its start.
                let align = if label.width() > label_width {
                    HAlign::Left
                } else {
                    HAlign::Right
                };
                printer.print_aligned((width - label_width, 0), label_width, align, &label);
            }
            (LabelPosition::Inside, BarStyle::Line | BarStyle::Block) => {
                printer.print_aligned((0, 0), width, HAlign::Center, &label);
            }
            _ => (),
        }
    }
}

#[crate::blueprint(ProgressBar::new())]
struct Blueprint {
    min: Option<usize>,
    max: Option<usize>,
    value: Option<usize>,
    color: Option<ColorType>,
    fill: Option<Fill>,
    style: Option<BarStyle>,
    label_position: Option<LabelPosition>,
    track: Option<ColorType>,
    label: Option<_>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;
    use crate::buffer::PrintBuffer;
    use crate::style::chart::Fill;
    use crate::style::{Color, ColorStyle};
    use crate::theme::Theme;
    use parking_lot::RwLock;
    use unicode_segmentation::UnicodeSegmentation;

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);
    const RED: Color = Color::Rgb(255, 0, 0);
    const BLUE: Color = Color::Rgb(0, 0, 255);

    fn buffer_with(width: usize, draw: impl FnOnce(&Printer)) -> RwLock<PrintBuffer> {
        let size = Vec2::new(width, 1);
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        draw(&Printer::new(size, &Theme::retro(), &buffer));
        buffer
    }

    fn buffer_of(bar: &ProgressBar, width: usize) -> RwLock<PrintBuffer> {
        buffer_with(width, |printer| bar.draw(printer))
    }

    /// The text of the first line, with `.` for the cells that were not printed
    /// and `_` for spaces.
    fn render(bar: &ProgressBar, width: usize) -> String {
        let buffer = buffer_of(bar, width);
        let buffer = buffer.read();
        (0..width)
            .map(|x| match buffer.cell_text(Vec2::new(x, 0)) {
                None | Some("") => ".",
                Some(" ") => "_",
                Some(text) => text,
            })
            .collect()
    }

    fn fronts(bar: &ProgressBar, width: usize) -> Vec<Color> {
        let buffer = buffer_of(bar, width);
        let buffer = buffer.read();
        (0..width)
            .map(|x| {
                buffer
                    .cell_style(Vec2::new(x, 0))
                    .map_or(Color::TerminalDefault, |s| s.color.front)
            })
            .collect()
    }

    fn backs(bar: &ProgressBar, width: usize) -> Vec<Color> {
        let buffer = buffer_of(bar, width);
        let buffer = buffer.read();
        (0..width)
            .map(|x| {
                buffer
                    .cell_style(Vec2::new(x, 0))
                    .map_or(Color::TerminalDefault, |s| s.color.back)
            })
            .collect()
    }

    fn bar(value: usize, style: BarStyle) -> ProgressBar {
        ProgressBar::new()
            .with_value(Counter::new(value))
            .style(style)
            .with_color(RED)
            .track(BLUE)
    }

    fn hidden(value: usize, style: BarStyle) -> ProgressBar {
        bar(value, style).label_position(LabelPosition::Hidden)
    }

    /// The drawing code of the solid bar before it was shared with the gauge.
    fn reference_draw(bar: &ProgressBar, color: ColorType, printer: &Printer) {
        fn sub_block(extra: usize) -> &'static str {
            ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"]
                .get(extra.wrapping_sub(1))
                .copied()
                .unwrap_or(" ")
        }

        let available = printer.size.x;
        let value = bar.value.get();
        let (length, extra) = if value < bar.min {
            (0, 0)
        } else {
            ratio(value - bar.min, bar.max - bar.min, available)
        };
        let label = (bar.label_maker)(value, (bar.min, bar.max));
        let offset = HAlign::Center.get_offset(label.len(), printer.size.x);

        let color_style = ColorStyle::new(PaletteColor::HighlightText, color);
        let reverse_style = ColorStyle::new(color, ColorType::InheritParent);

        printer.with_color(reverse_style, |printer| {
            printer.print((length, 0), sub_block(extra));
            printer.print((offset, 0), &label);
        });
        printer.with_color(color_style, |printer| {
            let printer = &printer.cropped((length, 1));
            printer.print_hline((0, 0), length, " ");
            printer.print((offset, 0), &label);
        });
    }

    /// Compares the solid bar with the drawing code it replaced, for a label.
    ///
    /// This only holds when the label is as long in bytes as it is wide on the screen:
    /// the old code centred it by its byte length, and the bar now centres it by its
    /// display width (an intended fix, that moves labels with accents or wide characters).
    fn assert_solid_matches_the_previous_drawing(label: fn(usize, (usize, usize)) -> String) {
        for color in [
            ColorType::from(PaletteColor::Highlight),
            ColorType::from(RED),
        ] {
            for (min, max) in [(0, 100), (10, 40), (5, 5), (0, 0), (0, 7)] {
                for width in 0..=14 {
                    for value in 0..=max + 3 {
                        let bar = ProgressBar::new()
                            .range(min, max)
                            .with_value(Counter::new(value))
                            .with_label(label)
                            .with_color(color);
                        let text = label(value, (min, max));
                        assert_eq!(text.len(), text.width(), "not a label for this test");
                        let new = buffer_of(&bar, width);
                        let old =
                            buffer_with(width, |printer| reference_draw(&bar, color, printer));
                        let (new, old) = (new.read(), old.read());
                        for x in 0..width {
                            let at = Vec2::new(x, 0);
                            let context = format!("{min}..{max} value {value} width {width} x {x}");
                            assert_eq!(new.cell_text(at), old.cell_text(at), "{context}");
                            assert_eq!(new.cell_style(at), old.cell_style(at), "{context}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn solid_is_identical_to_the_previous_drawing() {
        assert_solid_matches_the_previous_drawing(make_percentage);
        assert_solid_matches_the_previous_drawing(|_, _| String::new());
        assert_solid_matches_the_previous_drawing(|value, _| {
            format!("a very long label number {value}")
        });
    }

    /// Labels with wide characters or accents: the label is centred by its display width,
    /// and a character crossing the end of the filled part leaves a filled cell, as before.
    #[test]
    fn solid_wide_and_accented_labels() {
        let red = Color::Rgb(255, 0, 0);
        let labels = ["日本語", "éééé", "e\u{301}x", "a日b", "日", "xé日é"];
        for label in labels {
            for (min, max) in [(0, 10), (10, 40), (5, 5)] {
                for width in 0..=12 {
                    for value in 0..=max + 3 {
                        let make = |position| {
                            ProgressBar::new()
                                .range(min, max)
                                .with_value(Counter::new(value))
                                .with_label(move |_, _| label.to_string())
                                .with_color(red)
                                .label_position(position)
                        };
                        let bar = make(LabelPosition::Inside);
                        let without_label = buffer_of(&make(LabelPosition::Hidden), width);
                        let drawn = buffer_of(&bar, width);
                        let (drawn, without_label) = (drawn.read(), without_label.read());
                        let length = if value < min {
                            0
                        } else {
                            ratio(value - min, max - min, width).0
                        };
                        let context = format!("{label:?} {min}..{max} value {value} width {width}");

                        // The filled part stays filled everywhere (the second cell of a wide
                        // character that fits is not a cell of its own).
                        for x in 0..length.min(width) {
                            let at = Vec2::new(x, 0);
                            if drawn.cell_text(at).is_none() {
                                continue;
                            }
                            let style = drawn.cell_style(at).unwrap();
                            assert_eq!(style.color.back, red, "hole at {x}: {context}");
                        }

                        // A character that crosses the end of the filled part is not drawn.
                        let mut column = HAlign::Center.get_offset(label.width(), width);
                        for grapheme in label.graphemes(true) {
                            if column >= width {
                                break;
                            }
                            let crosses = column < length && column + grapheme.width() > length;
                            if crosses {
                                let at = Vec2::new(column, 0);
                                assert_eq!(drawn.cell_text(at), Some(" "), "{context}");
                                assert_eq!(
                                    drawn.cell_style(at),
                                    without_label.cell_style(at),
                                    "{context}"
                                );
                            }
                            column += grapheme.width();
                        }
                    }
                }
            }
        }
    }

    /// The label is centred by display width (it was by bytes).
    #[test]
    fn solid_label_is_centred_by_display_width() {
        let bar = ProgressBar::new()
            .range(0, 10)
            .with_value(Counter::new(0))
            .with_label(|_, _| String::from("éé"))
            .with_color(RED);
        // 2 columns of 6: starts at column 2, even though it is 4 bytes.
        // (The first cell is the empty partial block of the bar.)
        assert_eq!(render(&bar, 6), "_.éé..");
    }

    /// Nothing is printed outside the view, for every style and position.
    #[test]
    fn nothing_is_printed_outside_the_view() {
        let labels: [fn(usize, (usize, usize)) -> String; 4] = [
            make_percentage,
            |_, _| String::new(),
            |_, _| String::from("日本語日本語"),
            |v, _| format!("{v} of lots and lots"),
        ];
        for label in labels {
            for style in [BarStyle::Solid, BarStyle::Line, BarStyle::Block] {
                for position in [
                    LabelPosition::Inside,
                    LabelPosition::Right,
                    LabelPosition::Hidden,
                ] {
                    for (min, max) in [(0, 100), (10, 40), (5, 5)] {
                        for width in 0..=8usize {
                            for value in [0, 3, 5, 10, 25, 40, 100, 150] {
                                let bar = ProgressBar::new()
                                    .range(min, max)
                                    .with_value(Counter::new(value))
                                    .with_label(label)
                                    .style(style)
                                    .label_position(position);
                                // Draw in a printer cropped from a wider buffer.
                                let buffer =
                                    buffer_with(width + 4, |p| bar.draw(&p.cropped((width, 1))));
                                let buffer = buffer.read();
                                for x in width..width + 4 {
                                    let text = buffer.cell_text(Vec2::new(x, 0));
                                    assert!(
                                        matches!(text, None | Some("")),
                                        "{style:?} {position:?} {min}..{max} v{value} w{width} x{x}: {text:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn solid_glyphs() {
        assert_eq!(render(&bar(0, BarStyle::Solid), 10), "_..0_%....");
        assert_eq!(render(&bar(50, BarStyle::Solid), 10), "___50_%...");
        assert_eq!(render(&bar(100, BarStyle::Solid), 10), "__100_%___");
        // An eighth of a cell: 12 % of 10 cells is 9.6 eighths, rounded down.
        assert_eq!(render(&hidden(12, BarStyle::Solid), 10), "_▏........");
    }

    #[test]
    fn solid_colors() {
        let bar = bar(50, BarStyle::Solid);
        let fronts = fronts(&bar, 10);
        let backs = backs(&bar, 10);
        // The label is on the bar...
        assert_ne!(fronts[3], RED);
        assert_eq!(backs[3], RED);
        // ... or over the unfilled part.
        assert_eq!(fronts[5], RED);
        assert_eq!(fronts[6], RED);
        assert_ne!(backs[5], RED);
        // The track is ignored.
        assert!(!backs.contains(&BLUE));
        assert!(!fronts.contains(&BLUE));
    }

    #[test]
    fn line_glyphs() {
        assert_eq!(render(&hidden(0, BarStyle::Line), 4), "━━━━");
        assert_eq!(fronts(&hidden(0, BarStyle::Line), 4), [BLUE; 4]);
        assert_eq!(
            fronts(&hidden(50, BarStyle::Line), 4),
            [RED, RED, BLUE, BLUE]
        );
        assert_eq!(fronts(&hidden(100, BarStyle::Line), 4), [RED; 4]);
        // The precision is a cell: 30 % of 4 cells is 1.2.
        assert_eq!(
            fronts(&hidden(30, BarStyle::Line), 4),
            [RED, BLUE, BLUE, BLUE]
        );
    }

    #[test]
    fn block_glyphs() {
        assert_eq!(render(&hidden(0, BarStyle::Block), 4), "____");
        assert_eq!(render(&hidden(50, BarStyle::Block), 4), "██__");
        assert_eq!(render(&hidden(100, BarStyle::Block), 4), "████");
        // An eighth of a cell: 30 % of 4 cells is 9.6 eighths, rounded down.
        let thirty = hidden(30, BarStyle::Block);
        assert_eq!(render(&thirty, 4), "█▏__");
        // The track is the background, even behind the filled part.
        assert_eq!(backs(&thirty, 4), [BLUE; 4]);
        assert_eq!(fronts(&thirty, 4)[..2], [RED; 2]);
    }

    #[test]
    fn label_on_the_right_by_default() {
        // The bar leaves room for the widest label ("100 %") and a space.
        assert_eq!(render(&bar(0, BarStyle::Line), 10), "━━━━...0_%");
        assert_eq!(render(&bar(50, BarStyle::Line), 10), "━━━━..50_%");
        assert_eq!(
            fronts(&bar(50, BarStyle::Line), 10)[..4],
            [RED, RED, BLUE, BLUE]
        );
        assert_eq!(render(&bar(50, BarStyle::Block), 10), "██__..50_%");
        assert_eq!(render(&bar(100, BarStyle::Block), 10), "████.100_%");
    }

    #[test]
    fn the_bar_keeps_its_size_when_the_label_gains_a_digit() {
        // 20 cells, "100 %" is the widest label: the bar is 14 cells, then a gap.
        let width = 20;
        for style in [BarStyle::Line, BarStyle::Block] {
            for value in [9, 10, 100] {
                let text: Vec<char> = render(&bar(value, style), width).chars().collect();
                assert_eq!(text[14], '.', "{style:?} {value}: {text:?}");
                assert_ne!(text[13], '.', "{style:?} {value}: {text:?}");
            }
        }
        // The same filled length for the same ratio, whatever the label.
        let filled = |value| -> String {
            render(&bar(value, BarStyle::Block), width)
                .chars()
                .take(14)
                .collect()
        };
        assert_eq!(filled(100), "█".repeat(14));
        assert_eq!(filled(50), format!("{}{}", "█".repeat(7), "_".repeat(7)));
        assert_eq!(filled(9), format!("█▎{}", "_".repeat(12)),);
    }

    #[test]
    fn the_label_stays_at_the_right_edge() {
        for value in [0, 9, 10, 99, 100] {
            let text = render(&bar(value, BarStyle::Block), 20);
            assert!(text.ends_with(" %".replace(' ', "_").as_str()), "{text}");
            assert_eq!(text.chars().count(), 20);
        }
        let text = render(&bar(9, BarStyle::Line), 20);
        assert!(text.ends_with("9_%"), "{text}");
        assert!(!text.ends_with("_9_%"), "{text}");
    }

    #[test]
    fn the_widest_label_can_be_the_one_for_the_minimum() {
        let label = |value: usize, (min, _): (usize, usize)| {
            if value == min {
                String::from("not started")
            } else {
                format!("{value}")
            }
        };
        let bar = |value| {
            ProgressBar::new()
                .range(0, 10)
                .with_value(Counter::new(value))
                .style(BarStyle::Line)
                .with_label(label)
                .with_color(RED)
                .track(BLUE)
        };
        // 11 cells for "not started", and a space: a bar of 8 cells at any value.
        for value in [0, 3, 10] {
            let text = render(&bar(value), 20);
            assert_eq!(
                text.chars().take(8).collect::<String>(),
                "━".repeat(8),
                "{text}"
            );
            assert_eq!(text.chars().nth(8), Some('.'), "{text}");
        }
        // The short label is at the right edge of the room.
        assert!(render(&bar(3), 20).ends_with(".3"));
        assert!(render(&bar(0), 20).ends_with("not_started"));
    }

    #[test]
    fn a_label_wider_than_the_view_stays_inside() {
        let long = |value: usize, _| format!("{value} of a very long label");
        for style in [BarStyle::Solid, BarStyle::Line, BarStyle::Block] {
            for position in [LabelPosition::Right, LabelPosition::Inside] {
                for width in 0..=12 {
                    let bar = ProgressBar::new()
                        .with_value(Counter::new(50))
                        .style(style)
                        .label_position(position)
                        .with_label(long);
                    // `buffer_with` panics if something is drawn outside of the buffer.
                    let text = render(&bar, width);
                    assert_eq!(text.chars().count(), width);
                }
            }
        }
    }

    #[test]
    fn label_on_the_right_of_a_solid_bar() {
        let bar = bar(100, BarStyle::Solid).label_position(LabelPosition::Right);
        assert_eq!(render(&bar, 10), "____.100_%");
        assert_eq!(backs(&bar, 10)[..4], [RED; 4]);
    }

    #[test]
    fn label_wider_than_the_bar() {
        // No room left for the bar: only the label is shown.
        assert_eq!(render(&bar(100, BarStyle::Line), 6), ".100_%");
        assert_eq!(render(&bar(100, BarStyle::Line), 5), "100_%");
        // The label is cropped, but not the start of it.
        assert_eq!(render(&bar(100, BarStyle::Line), 3), "100");
        assert_eq!(render(&bar(100, BarStyle::Solid), 3), "100");
    }

    #[test]
    fn label_inside_a_line_or_block() {
        let inside = |style| bar(50, style).label_position(LabelPosition::Inside);
        assert_eq!(render(&inside(BarStyle::Line), 10), "━━━50_%━━━");
        assert_eq!(render(&inside(BarStyle::Block), 10), "███50_%___");
    }

    #[test]
    fn hidden_label() {
        for style in [BarStyle::Solid, BarStyle::Line, BarStyle::Block] {
            let text = render(&hidden(100, style), 6);
            assert!(!text.contains('%'), "{text}");
        }
        assert_eq!(render(&hidden(100, BarStyle::Line), 6), "━━━━━━");
    }

    #[test]
    fn default_label_position_follows_the_style() {
        let mut bar = ProgressBar::new();
        assert_eq!(bar.effective_label_position(), LabelPosition::Inside);
        bar.set_style(BarStyle::Line);
        assert_eq!(bar.effective_label_position(), LabelPosition::Right);
        bar.set_style(BarStyle::Block);
        assert_eq!(bar.effective_label_position(), LabelPosition::Right);
        bar.set_style(BarStyle::Solid);
        assert_eq!(bar.effective_label_position(), LabelPosition::Inside);

        // An explicit choice stays, whatever the style.
        bar.set_label_position(LabelPosition::Inside);
        bar.set_style(BarStyle::Line);
        assert_eq!(bar.effective_label_position(), LabelPosition::Inside);
        bar.set_label_position(LabelPosition::Right);
        bar.set_style(BarStyle::Solid);
        assert_eq!(bar.effective_label_position(), LabelPosition::Right);
    }

    #[test]
    fn track_color() {
        let bar = hidden(25, BarStyle::Line).track(WHITE);
        assert_eq!(fronts(&bar, 4), [RED, WHITE, WHITE, WHITE]);

        let bar = hidden(25, BarStyle::Block).track(WHITE);
        assert_eq!(backs(&bar, 4), [WHITE; 4]);

        // The default is the axis color of the theme.
        let bar = ProgressBar::new();
        assert_eq!(bar.track, ColorType::from(&AXIS));
    }

    #[test]
    fn fill_steps() {
        let steps = || Fill::steps([(0.0, RED), (0.5, BLUE)]);
        let low = hidden(40, BarStyle::Line).fill(steps()).track(WHITE);
        let mut expected = vec![RED; 4];
        expected.extend([WHITE; 6]);
        assert_eq!(fronts(&low, 10), expected);

        let high = hidden(60, BarStyle::Line).fill(steps()).track(WHITE);
        let mut expected = vec![BLUE; 6];
        expected.extend([WHITE; 4]);
        assert_eq!(fronts(&high, 10), expected);

        // Also for a solid bar.
        let solid = hidden(100, BarStyle::Solid).fill(steps());
        assert_eq!(backs(&solid, 4), [BLUE; 4]);
    }

    #[test]
    fn fill_gradient() {
        let gradient = Fill::horizontal([BLACK, WHITE]);
        let solid = hidden(100, BarStyle::Solid).fill(gradient.clone());
        let colors = backs(&solid, 5);
        assert_eq!(colors[0], BLACK);
        assert_eq!(colors[4], WHITE);
        let Color::Rgb(mid, _, _) = colors[2] else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&mid));

        // The gradient spans the whole bar, not just the filled part.
        let line = hidden(60, BarStyle::Line).fill(gradient).track(BLUE);
        let colors = fronts(&line, 5);
        assert_eq!(colors[0], BLACK);
        assert_ne!(colors[2], WHITE);
        assert_eq!(colors[3..], [BLUE; 2]);
    }

    #[test]
    fn color_and_fill_last_wins() {
        let bar = hidden(100, BarStyle::Solid).fill(BLUE).with_color(RED);
        assert_eq!(backs(&bar, 3), [RED; 3]);
        let bar = hidden(100, BarStyle::Solid).with_color(RED).fill(BLUE);
        assert_eq!(backs(&bar, 3), [BLUE; 3]);
    }

    #[test]
    fn default_color_is_the_highlight() {
        let bar = ProgressBar::new().with_value(Counter::new(100));
        let theme = Theme::retro();
        let highlight = theme.palette[PaletteColor::Highlight];
        assert_eq!(backs(&bar, 3), [highlight; 3]);
    }

    #[test]
    fn label_is_centered_by_width() {
        // "日本" is 6 bytes, but 4 cells wide.
        let bar = ProgressBar::new().with_label(|_, _| String::from("日本"));
        let buffer = buffer_of(&bar, 10);
        let buffer = buffer.read();
        assert_eq!(buffer.cell_text(Vec2::new(3, 0)), Some("日"));
        assert_eq!(buffer.cell_text(Vec2::new(5, 0)), Some("本"));

        let bar = bar.style(BarStyle::Line);
        let buffer = buffer_of(&bar, 10);
        let buffer = buffer.read();
        // On the right of a bar of 5 cells.
        assert_eq!(buffer.cell_text(Vec2::new(6, 0)), Some("日"));
        assert_eq!(buffer.cell_text(Vec2::new(8, 0)), Some("本"));
    }

    #[test]
    fn zero_width() {
        for style in [BarStyle::Solid, BarStyle::Line, BarStyle::Block] {
            assert_eq!(render(&bar(50, style), 0), "");
        }
    }

    #[test]
    fn empty_range() {
        // This used to divide by zero.
        let bar = ProgressBar::new().range(5, 5).with_value(Counter::new(5));
        assert_eq!(render(&bar, 10), "__100_%___");
        let bar = bar.with_value(Counter::new(2));
        assert_eq!(render(&bar, 10), "_..0_%....");
    }

    #[test]
    fn style_from_str() {
        assert_eq!("solid".parse(), Ok(BarStyle::Solid));
        assert_eq!("line".parse(), Ok(BarStyle::Line));
        assert_eq!("block".parse(), Ok(BarStyle::Block));
        assert_eq!("fancy".parse::<BarStyle>(), Err(()));

        assert_eq!("inside".parse(), Ok(LabelPosition::Inside));
        assert_eq!("right".parse(), Ok(LabelPosition::Right));
        assert_eq!("hidden".parse(), Ok(LabelPosition::Hidden));
        assert_eq!("left".parse::<LabelPosition>(), Err(()));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            ProgressBar:
                value: 50
                style: line
                label_position: hidden
                track: black
                fill: white
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let bar = view
            .downcast_ref::<ProgressBar>()
            .expect("the view should be a ProgressBar");
        assert_eq!(bar.style, BarStyle::Line);
        assert_eq!(bar.label_position, Some(LabelPosition::Hidden));
        use crate::style::BaseColor;
        let (white, black) = (BaseColor::White.dark(), BaseColor::Black.dark());
        assert_eq!(fronts(bar, 4), [white, white, black, black]);

        // Without a label position, the style picks it.
        let config: crate::builder::Config =
            crate::reexports::serde_json::from_str(r#"{"ProgressBar": {"style": "block"}}"#)
                .unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let bar = view.downcast_ref::<ProgressBar>().unwrap();
        assert_eq!(bar.label_position, None);
        assert_eq!(bar.effective_label_position(), LabelPosition::Right);

        let config: crate::builder::Config =
            crate::reexports::serde_json::from_str(r#"{"ProgressBar": {"style": "x"}}"#).unwrap();
        assert!(crate::builder::Context::new().build(&config).is_err());
    }
}
