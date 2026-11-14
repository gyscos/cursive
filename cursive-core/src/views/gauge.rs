use crate::Printer;
use crate::Vec2;
use crate::style::ColorType;
use crate::style::chart::{AXIS, Fill, series_color};
use crate::view::View;
use crate::views::bar::{Bar, BarStyle};
use crate::views::{Series, ValueRange};

/// Where a gauge reads its value.
enum Source {
    Series(Series),
    Value(f64),
    Live(Box<dyn Fn() -> f64 + Send + Sync>),
}

/// A one-line bar showing a value within a range.
///
/// The bar is filled from the left proportionally to where the value is in
/// the [`ValueRange`]. With [`BarStyle::Line`] and [`BarStyle::Block`], a track
/// runs across the whole width behind it; [`BarStyle::Solid`] has no track: the
/// part that is not filled shows the background of the view.
///
/// # Source
///
/// The value comes from one of three sources:
///
/// * The latest value of a [`Series`], with [`Gauge::new`]. An empty series
///   shows only the track (nothing with [`BarStyle::Solid`]).
/// * A fixed number, with [`Gauge::value`].
/// * A closure, with [`Gauge::live`]. It is called when the gauge is drawn.
///
/// Setting a new source replaces the previous one.
///
/// The [`Fill`] sees the whole bar as its area: [`Fill::by_value`] colors the
/// bar according to the current value, and [`Fill::horizontal`] gives a gradient
/// along the bar.
///
/// A gauge takes the width it is offered, and a single row. It is not
/// focusable and ignores events. Text like a label or a percentage is left to
/// the caller: put the gauge in a [`LinearLayout`](crate::views::LinearLayout)
/// next to a [`TextView`](crate::views::TextView).
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::BaseColor;
/// use cursive_core::style::chart::Fill;
/// use cursive_core::views::{BarStyle, Gauge, Series};
///
/// let load = Series::new();
/// load.push(0.25);
///
/// let gauge = Gauge::new(&load)
///     .style(BarStyle::Block)
///     .fill(Fill::by_value([
///         BaseColor::Green.dark(),
///         BaseColor::Red.dark(),
///     ]));
///
/// // The gauge shows the new value as soon as it is pushed.
/// load.push(0.75);
///
/// // Values from 0 to 200 instead of 0 to 1.
/// let gauge = Gauge::new([150.0]).range(0.0..=200.0);
///
/// // A fixed value, or a closure.
/// let gauge = Gauge::value(0.5);
/// let gauge = Gauge::live(|| 0.5);
/// ```
pub struct Gauge {
    source: Source,
    range: ValueRange,
    fill: Fill,
    track: ColorType,
    style: BarStyle,
}

impl Gauge {
    /// Creates a new gauge showing the latest value of the given series.
    ///
    /// Pass a `&Series` to keep sharing the data with the caller.
    ///
    /// By default the range is `0.0..=1.0`, the fill is the first
    /// [series color](series_color), the track uses the chart axis color, and
    /// the style is [`BarStyle::Line`].
    pub fn new(series: impl Into<Series>) -> Self {
        Self::with_source(Source::Series(series.into()))
    }

    /// Creates a new gauge showing a fixed value.
    ///
    /// Use [`set_value`](Self::set_value) to change it later.
    ///
    /// The other defaults are the same as [`Gauge::new`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::Gauge;
    ///
    /// // Half full.
    /// let gauge = Gauge::value(0.5);
    ///
    /// // 30 out of 120.
    /// let gauge = Gauge::value(30.0).range(0.0..=120.0);
    /// ```
    pub fn value(value: f64) -> Self {
        Self::with_source(Source::Value(value))
    }

    /// Creates a new gauge showing the value given by a closure.
    ///
    /// The closure is called once each time the gauge is drawn. This happens
    /// on the UI thread, so it must be fast: read an atomic or a lock, but
    /// don't wait for anything.
    ///
    /// A gauge has a fixed size, so the layout does not depend on the value:
    /// the closure is not called when computing the layout.
    ///
    /// The other defaults are the same as [`Gauge::new`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::Gauge;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// // Some value updated by another thread, as a percentage.
    /// let progress = Arc::new(AtomicUsize::new(0));
    ///
    /// let shared = Arc::clone(&progress);
    /// let gauge = Gauge::live(move || shared.load(Ordering::Relaxed) as f64 / 100.0);
    ///
    /// progress.store(40, Ordering::Relaxed);
    /// ```
    pub fn live<F>(source: F) -> Self
    where
        F: Fn() -> f64 + Send + Sync + 'static,
    {
        Self::with_source(Source::Live(Box::new(source)))
    }

    fn with_source(source: Source) -> Self {
        Gauge {
            source,
            range: ValueRange::fixed(0.0, 1.0),
            fill: Fill::solid(series_color(0)),
            track: ColorType::from(&AXIS),
            style: BarStyle::Line,
        }
    }

    /// Returns the series shown by this gauge.
    ///
    /// Returns `None` if the gauge shows a fixed value or a closure.
    pub fn series(&self) -> Option<&Series> {
        match &self.source {
            Source::Series(series) => Some(series),
            Source::Value(_) | Source::Live(_) => None,
        }
    }

    /// Sets the series shown by this gauge.
    ///
    /// This replaces a fixed value or a closure.
    pub fn set_series(&mut self, series: impl Into<Series>) {
        self.source = Source::Series(series.into());
    }

    /// Shows a fixed value.
    ///
    /// This replaces a series or a closure.
    pub fn set_value(&mut self, value: f64) {
        self.source = Source::Value(value);
    }

    /// Shows the value given by a closure.
    ///
    /// This replaces a series or a fixed value. See [`Gauge::live`] for when the
    /// closure is called.
    pub fn set_source<F>(&mut self, source: F)
    where
        F: Fn() -> f64 + Send + Sync + 'static,
    {
        self.source = Source::Live(Box::new(source));
    }

    /// Sets the range of values the bar goes across.
    ///
    /// The default is `0.0..=1.0`. Values outside of the range fill the bar
    /// completely, or not at all.
    pub fn set_range(&mut self, range: impl Into<ValueRange>) {
        self.range = range.into();
    }

    /// Sets the range of values the bar goes across.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn range(self, range: impl Into<ValueRange>) -> Self {
        crate::With::with(self, |s| s.set_range(range))
    }

    /// Sets the fill used to color the filled part of the bar.
    ///
    /// The default is a solid [`series_color(0)`](series_color).
    pub fn set_fill(&mut self, fill: impl Into<Fill>) {
        self.fill = fill.into();
    }

    /// Sets the fill used to color the filled part of the bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn fill(self, fill: impl Into<Fill>) -> Self {
        crate::With::with(self, |s| s.set_fill(fill))
    }

    /// Sets the color of the unfilled part of the bar.
    ///
    /// [`BarStyle::Solid`] has no track: it ignores this.
    ///
    /// The default is the [chart axis color](AXIS).
    pub fn set_track(&mut self, track: impl Into<ColorType>) {
        self.track = track.into();
    }

    /// Sets the color of the unfilled part of the bar.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn track(self, track: impl Into<ColorType>) -> Self {
        crate::With::with(self, |s| s.set_track(track))
    }

    /// Sets how the bar is drawn.
    ///
    /// The default is [`BarStyle::Line`].
    pub fn set_style(&mut self, style: BarStyle) {
        self.style = style;
    }

    /// Sets how the bar is drawn.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn style(self, style: BarStyle) -> Self {
        crate::With::with(self, |s| s.set_style(style))
    }
}

impl View for Gauge {
    fn layout_key(&self) -> u64 {
        // The size only depends on the constraint, never on the value or the settings.
        crate::view::layout_key_seed::<Self>()
    }

    fn needs_relayout(&self) -> bool {
        false
    }

    fn draw(&self, printer: &Printer) {
        let width = printer.size.x;
        if width == 0 || printer.size.y == 0 {
            return;
        }

        let value = match &self.source {
            Source::Series(series) => series.last(),
            Source::Value(value) => Some(*value),
            Source::Live(source) => Some(source()),
        };
        let ratio = match value {
            Some(value) => self.range.resolve([value]).ratio(value),
            None => 0.0,
        };
        Bar {
            style: self.style,
            eighths: self.style.eighths(ratio, width),
            ratio,
            fill: &self.fill,
            track: self.track,
            label: None,
        }
        .draw(printer);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2::new(constraint.x, 1)
    }
}

// Without a source, the gauge shows an empty series.
// If both are given, the value wins.
#[crate::blueprint(Gauge::new(Series::new()))]
struct Blueprint {
    series: Option<Series>,
    value: Option<f64>,

    range: Option<ValueRange>,
    fill: Option<Fill>,
    track: Option<ColorType>,
    style: Option<BarStyle>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::Color;
    use crate::theme::Theme;
    use parking_lot::{Mutex, RwLock};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);
    const RED: Color = Color::Rgb(255, 0, 0);
    const BLUE: Color = Color::Rgb(0, 0, 255);

    fn buffer_of(gauge: &Gauge, size: impl Into<Vec2>) -> RwLock<PrintBuffer> {
        let size = size.into();
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        gauge.draw(&Printer::new(size, &Theme::retro(), &buffer));
        buffer
    }

    /// The first line of text, with `.` for the cells that were not printed
    /// and `_` for spaces.
    fn render(gauge: &Gauge, width: usize) -> String {
        let buffer = buffer_of(gauge, (width, 1));
        let buffer = buffer.read();
        (0..width)
            .map(|x| match buffer.cell_text(Vec2::new(x, 0)) {
                None | Some("") => ".",
                Some(" ") => "_",
                Some(text) => text,
            })
            .collect()
    }

    fn fronts(gauge: &Gauge, width: usize) -> Vec<Color> {
        let buffer = buffer_of(gauge, (width, 1));
        let buffer = buffer.read();
        (0..width)
            .map(|x| {
                buffer
                    .cell_style(Vec2::new(x, 0))
                    .map_or(Color::TerminalDefault, |s| s.color.front)
            })
            .collect()
    }

    fn backs(gauge: &Gauge, width: usize) -> Vec<Color> {
        let buffer = buffer_of(gauge, (width, 1));
        let buffer = buffer.read();
        (0..width)
            .map(|x| {
                buffer
                    .cell_style(Vec2::new(x, 0))
                    .map_or(Color::TerminalDefault, |s| s.color.back)
            })
            .collect()
    }

    fn line(values: impl Into<Series>) -> Gauge {
        Gauge::new(values).fill(RED).track(BLUE)
    }

    fn block(values: impl Into<Series>) -> Gauge {
        line(values).style(BarStyle::Block)
    }

    #[test]
    fn line_glyphs() {
        // The track and the bar use the same glyph: check the colors.
        assert_eq!(render(&line([0.5]), 4), "━━━━");
        assert_eq!(fronts(&line([0.0]), 4), [BLUE; 4]);
        assert_eq!(fronts(&line([0.5]), 4), [RED, RED, BLUE, BLUE]);
        assert_eq!(fronts(&line([1.0]), 4), [RED; 4]);
        // The bar is rounded to a cell.
        assert_eq!(fronts(&line([0.3]), 4), [RED, BLUE, BLUE, BLUE]);
        assert_eq!(fronts(&line([0.4]), 4), [RED, RED, BLUE, BLUE]);
    }

    #[test]
    fn block_glyphs() {
        assert_eq!(render(&block([0.0]), 4), "____");
        assert_eq!(render(&block([0.5]), 4), "██__");
        assert_eq!(render(&block([1.0]), 4), "████");
        // Eighths of a cell: 0.3 * 4 cells = 9.6 eighths, so 10: a cell and a quarter.
        assert_eq!(render(&block([0.3]), 4), "█▎__");
    }

    #[test]
    fn values_outside_the_range() {
        assert_eq!(fronts(&line([-3.0]), 3), [BLUE; 3]);
        assert_eq!(render(&block([f64::NAN]), 3), "___");
        assert_eq!(render(&block([-3.0]), 3), "___");
        assert_eq!(render(&block([7.0]), 3), "███");
        assert_eq!(fronts(&line([7.0]), 3), [RED; 3]);
    }

    #[test]
    fn empty_series_shows_the_track() {
        assert_eq!(fronts(&line(Series::new()), 3), [BLUE; 3]);
        assert_eq!(render(&block(Series::new()), 3), "___");
        assert_eq!(backs(&block(Series::new()), 3), [BLUE; 3]);
    }

    #[test]
    fn only_the_last_value_counts() {
        assert_eq!(render(&block([1.0, 0.5]), 4), "██__");
    }

    #[test]
    fn custom_range() {
        let gauge = block([150.0]).range(100.0..=200.0);
        assert_eq!(render(&gauge, 4), "██__");
        // The range is applied to the value, not just clamped.
        let gauge = block([0.5]).range(ValueRange::fixed(-1.0, 1.0));
        assert_eq!(render(&gauge, 4), "███_");
    }

    #[test]
    fn fill_by_value() {
        // Halfway between black and white, for the whole bar.
        let gauge = Gauge::new([0.5])
            .fill(Fill::by_value([BLACK, WHITE]))
            .track(BLUE);
        let colors = fronts(&gauge, 4);
        let Color::Rgb(mid, _, _) = colors[0] else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&mid));
        assert_eq!(colors[0], colors[1]);
        assert_eq!(colors[2], BLUE);

        let gauge = Gauge::new([1.0]).fill(Fill::by_value([BLACK, WHITE]));
        assert_eq!(fronts(&gauge, 3), [WHITE; 3]);
    }

    #[test]
    fn horizontal_gradient_along_the_bar() {
        let gauge = Gauge::new([1.0]).fill(Fill::horizontal([BLACK, WHITE]));
        let colors = fronts(&gauge, 5);
        assert_eq!(colors[0], BLACK);
        assert_eq!(colors[4], WHITE);
        let Color::Rgb(mid, _, _) = colors[2] else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&mid));

        // The gradient spans the whole bar, not just the filled part.
        let gauge = Gauge::new([0.6])
            .fill(Fill::horizontal([BLACK, WHITE]))
            .style(BarStyle::Block);
        let colors = fronts(&gauge, 5);
        assert_eq!(colors[0], BLACK);
        assert_ne!(colors[2], WHITE);
    }

    #[test]
    fn track_color() {
        // Front for the line.
        let gauge = line([0.25]);
        assert_eq!(fronts(&gauge, 4)[1..], [BLUE; 3]);
        // Background for the blocks, including behind the filled part.
        let gauge = block([0.375]);
        assert_eq!(render(&gauge, 4), "█▌__");
        assert_eq!(backs(&gauge, 4), [BLUE; 4]);
        assert_eq!(fronts(&gauge, 4)[..2], [RED; 2]);
    }

    #[test]
    fn default_colors_come_from_the_theme() {
        let gauge = Gauge::new([0.5]);
        let colors = fronts(&gauge, 4);
        assert_ne!(colors[0], colors[3]);
    }

    #[test]
    fn zero_width() {
        assert_eq!(render(&line([0.5]), 0), "");
        assert_eq!(render(&block([0.5]), 0), "");
    }

    #[test]
    fn draws_only_the_first_row() {
        let buffer = buffer_of(&block([1.0]), (3, 2));
        let buffer = buffer.read();
        assert_eq!(buffer.cell_text(Vec2::new(0, 0)), Some("█"));
        assert!(matches!(
            buffer.cell_text(Vec2::new(0, 1)),
            None | Some("") | Some(" ")
        ));
    }

    #[test]
    fn required_size() {
        let mut gauge = Gauge::new(Series::new());
        assert_eq!(gauge.required_size(Vec2::new(10, 5)), Vec2::new(10, 1));
        assert_eq!(gauge.required_size(Vec2::new(0, 5)), Vec2::new(0, 1));
    }

    #[test]
    fn fixed_value() {
        let gauge = Gauge::value(0.5).fill(RED).track(BLUE);
        assert_eq!(fronts(&gauge, 4), [RED, RED, BLUE, BLUE]);
        assert!(gauge.series().is_none());

        let gauge = Gauge::value(150.0)
            .range(100.0..=200.0)
            .style(BarStyle::Block);
        assert_eq!(render(&gauge, 4), "██__");
        // Like for a series, values out of the range fill the bar or not at all.
        assert_eq!(
            render(&Gauge::value(f64::NAN).style(BarStyle::Block), 3),
            "___"
        );
        assert_eq!(render(&Gauge::value(7.0).style(BarStyle::Block), 3), "███");
    }

    #[test]
    fn closure_value() {
        let shared = Arc::new(Mutex::new(0.25));
        let source = Arc::clone(&shared);
        let gauge = Gauge::live(move || *source.lock()).style(BarStyle::Block);
        assert!(gauge.series().is_none());
        assert_eq!(render(&gauge, 4), "█___");
        // The closure is evaluated at each draw.
        *shared.lock() = 0.75;
        assert_eq!(render(&gauge, 4), "███_");
    }

    #[test]
    fn closure_is_called_once_per_draw() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let mut gauge = Gauge::live(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            0.5
        });
        // Not for the layout.
        gauge.required_size(Vec2::new(10, 1));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        render(&gauge, 4);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        render(&gauge, 4);
        render(&gauge, 4);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        // Even with several styles of drawing.
        gauge.set_style(BarStyle::Solid);
        render(&gauge, 4);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn switching_sources() {
        let series = Series::new();
        series.push(1.0);
        let mut gauge = Gauge::new(&series).style(BarStyle::Block);
        assert_eq!(render(&gauge, 4), "████");
        assert!(gauge.series().is_some());

        gauge.set_value(0.5);
        assert_eq!(render(&gauge, 4), "██__");
        assert!(gauge.series().is_none());
        // The series is not read anymore.
        series.push(0.0);
        assert_eq!(render(&gauge, 4), "██__");

        gauge.set_source(|| 0.25);
        assert_eq!(render(&gauge, 4), "█___");
        assert!(gauge.series().is_none());

        gauge.set_value(1.0);
        assert_eq!(render(&gauge, 4), "████");

        gauge.set_source(|| 0.75);
        gauge.set_series(&series);
        assert_eq!(render(&gauge, 4), "____");
        assert_eq!(gauge.series().unwrap().to_vec(), [1.0, 0.0]);
    }

    #[test]
    fn shares_the_series() {
        let series = Series::new();
        let mut gauge = block(&series);
        assert_eq!(render(&gauge, 4), "____");
        series.push(0.5);
        assert_eq!(render(&gauge, 4), "██__");
        series.push(1.0);
        assert_eq!(render(&gauge, 4), "████");
        assert_eq!(gauge.series().unwrap().len(), 2);

        gauge.set_series([0.25]);
        assert_eq!(render(&gauge, 4), "█___");
    }

    #[test]
    fn setters() {
        let mut gauge = Gauge::new([5.0]);
        gauge.set_range(0.0..=10.0);
        gauge.set_fill(RED);
        gauge.set_track(BLUE);
        gauge.set_style(BarStyle::Block);
        assert_eq!(render(&gauge, 4), "██__");
        assert_eq!(backs(&gauge, 4)[3], BLUE);
    }

    #[test]
    fn solid_style() {
        let solid = |value| line([value]).style(BarStyle::Solid);
        // A solid bar has no track, and no label.
        assert_eq!(render(&solid(0.0), 4), "_...");
        assert_eq!(render(&solid(0.5), 4), "___.");
        assert_eq!(render(&solid(1.0), 4), "____");
        // Eighths of a cell: 0.3 * 4 cells = 9.6 eighths, so 10.
        assert_eq!(render(&solid(0.3), 4), "_▎..");
        // The bar is the background of the filled cells.
        assert_eq!(backs(&solid(0.5), 4)[..2], [RED; 2]);
        assert!(!fronts(&solid(0.5), 4).contains(&BLUE));
        assert!(!backs(&solid(0.5), 4).contains(&BLUE));
        // The fill gives the color of each cell.
        let gauge = Gauge::new([1.0])
            .fill(Fill::horizontal([BLACK, WHITE]))
            .style(BarStyle::Solid);
        let colors = backs(&gauge, 5);
        assert_eq!((colors[0], colors[4]), (BLACK, WHITE));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            Gauge:
                series: [0.25, 0.5]
                range: [0, 1]
                fill: white
                track: black
                style: block
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let gauge = view
            .downcast_ref::<Gauge>()
            .expect("the view should be a Gauge");
        assert_eq!(gauge.series().unwrap().to_vec(), [0.25, 0.5]);
        assert_eq!(gauge.style, BarStyle::Block);
        assert_eq!(render(gauge, 4), "██__");
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_with_a_value() {
        let build = |config: &str| {
            let config: crate::builder::Config =
                crate::reexports::serde_json::from_str(config).unwrap();
            crate::builder::Context::new().build(&config)
        };

        let view = build(r#"{"Gauge": {"value": 0.5, "style": "block"}}"#).unwrap();
        let gauge = view.downcast_ref::<Gauge>().unwrap();
        assert!(gauge.series().is_none());
        assert_eq!(render(gauge, 4), "██__");

        // A value can also be an integer.
        let view = build(r#"{"Gauge": {"value": 1, "style": "block"}}"#).unwrap();
        assert_eq!(render(view.downcast_ref::<Gauge>().unwrap(), 4), "████");

        // The value wins if there is both.
        let view =
            build(r#"{"Gauge": {"series": [1.0], "value": 0.5, "style": "block"}}"#).unwrap();
        assert_eq!(render(view.downcast_ref::<Gauge>().unwrap(), 4), "██__");

        // Without either, there is only the track.
        let view = build(r#"{"Gauge": {"style": "block"}}"#).unwrap();
        assert_eq!(render(view.downcast_ref::<Gauge>().unwrap(), 4), "____");

        assert!(build(r#"{"Gauge": {"value": "high"}}"#).is_err());
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_with_a_stored_series() {
        let series = Series::new();
        let mut context = crate::builder::Context::new();
        context.store("load", series.clone());

        let config: crate::builder::Config = crate::reexports::serde_json::from_str(
            r#"{"Gauge": {"series": "$load", "style": "line"}}"#,
        )
        .unwrap();
        let view = context.build(&config).unwrap();
        series.push(1.0);
        let gauge = view.downcast_ref::<Gauge>().unwrap();
        assert_eq!(gauge.series().unwrap().to_vec(), [1.0]);
        assert_eq!(gauge.style, BarStyle::Line);

        let config: crate::builder::Config =
            crate::reexports::serde_json::from_str(r#"{"Gauge": {"series": [], "style": "x"}}"#)
                .unwrap();
        assert!(context.build(&config).is_err());
    }

    #[test]
    fn layout_is_cached() {
        crate::views::layout_key_tests::assert_layout_is_cached(|| Gauge::value(0.5));
    }

    #[test]
    fn data_and_settings_do_not_change_the_layout_key() {
        // The size is the width offered by the constraint, one row:
        // nothing about the value or the look can change it.
        let series = Series::new();
        let mut gauge = Gauge::new(&series);
        let key = gauge.layout_key();
        series.push(0.9);
        gauge.set_style(BarStyle::Line);
        gauge.set_range(ValueRange::fixed(0.0, 5.0));
        gauge.set_fill(Fill::solid(RED));
        assert_eq!(gauge.layout_key(), key);
        assert!(!gauge.needs_relayout());
        assert_eq!(gauge.required_size(Vec2::new(7, 4)), Vec2::new(7, 1));
    }

    #[test]
    fn tiny_sizes_and_odd_values_stay_inside_the_view() {
        let values = [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            0.5,
            1.0,
            2.0,
            f64::MAX,
            f64::MIN,
        ];
        let ranges = [
            ValueRange::fixed(0.0, 1.0),
            ValueRange::fixed(1.0, 1.0),
            ValueRange::fixed(2.0, 1.0),
            ValueRange::fixed(f64::NAN, f64::NAN),
            ValueRange::fixed(f64::NEG_INFINITY, f64::INFINITY),
            ValueRange::fixed(f64::MIN, f64::MAX),
            ValueRange::auto(),
            ValueRange::default(),
        ];
        for style in [BarStyle::Solid, BarStyle::Line, BarStyle::Block] {
            for range in ranges {
                for value in values {
                    for (w, h) in [(0, 0), (1, 1), (2, 1), (3, 1), (0, 1), (1, 0), (5, 3)] {
                        let gauge = Gauge::value(value).range(range).style(style);
                        // A buffer larger than the printer: what is drawn
                        // beyond the printer or after the first row would show.
                        let size = Vec2::new(w + 3, h + 1);
                        let mut buffer = PrintBuffer::new();
                        buffer.resize(size);
                        let buffer = RwLock::new(buffer);
                        let theme = Theme::retro();
                        let printer = Printer::new(size, &theme, &buffer).cropped(Vec2::new(w, h));
                        gauge.draw(&printer);
                        let buffer = buffer.read();
                        for x in 0..size.x {
                            for y in 0..size.y {
                                if x >= w || y >= 1 || y >= h {
                                    let t = buffer.cell_text(Vec2::new(x, y));
                                    assert!(
                                        matches!(t, None | Some("")),
                                        "{style:?} {range:?} {value} {w}x{h} at {x},{y}: {t:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
