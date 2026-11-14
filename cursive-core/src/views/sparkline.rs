use crate::Printer;
use crate::Vec2;
use crate::style::ColorStyle;
use crate::style::chart::{Fill, FillPos, series_color};
use crate::view::View;
use crate::views::{Series, ValueRange};

/// A small bar chart of one [`Series`], without axis or labels.
///
/// Each sample takes one column, and its height has a precision of an eighth
/// of a cell. The newest sample is in the rightmost column, and the chart fills
/// the whole area it is given, whatever its height.
///
/// The bounds of the chart come from the samples that are visible, through its
/// [`ValueRange`]. With fewer samples than columns, the left columns stay empty.
/// So do the columns of samples that are not finite numbers, which do not affect the range
/// either: see [Invalid values](Series#invalid-values).
///
/// This view has no preferred width other than the number of samples: wrap it
/// in [`full_width`](crate::view::Resizable::full_width) or
/// [`fixed_size`](crate::view::Resizable::fixed_size) to give it more room.
/// It is not focusable and ignores events.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::PaletteColor;
/// use cursive_core::style::chart::Fill;
/// use cursive_core::view::Resizable;
/// use cursive_core::views::{Series, Sparkline, ValueRange};
///
/// let series = Series::with_capacity(60);
/// series.extend([3.0, 5.0, 4.0, 8.0, 6.0]);
///
/// let sparkline = Sparkline::new(&series)
///     .fill(Fill::by_value([
///         PaletteColor::Secondary,
///         PaletteColor::Highlight,
///     ]))
///     .range(ValueRange::from(0.0..=10.0))
///     .full_width();
///
/// // The sparkline shows the new data as soon as it is pushed.
/// series.push(9.0);
/// ```
pub struct Sparkline {
    series: Series,
    fill: Fill,
    range: ValueRange,
    // The width we asked for the last time we were laid out.
    laid_out: Option<usize>,
}

impl Sparkline {
    /// Creates a new sparkline showing the given series.
    ///
    /// Pass a `&Series` to keep sharing the data with the caller.
    pub fn new(series: impl Into<Series>) -> Self {
        Sparkline {
            series: series.into(),
            fill: Fill::solid(series_color(0)),
            range: ValueRange::default(),
            laid_out: None,
        }
    }

    /// The width we ask for: one column per sample, and at least one.
    ///
    /// This is all that decides the size: the key and `needs_relayout` follow it.
    fn wanted_width(&self) -> usize {
        self.series.len().max(1)
    }

    /// Returns the series shown by this sparkline.
    pub fn series(&self) -> &Series {
        &self.series
    }

    /// Sets the series shown by this sparkline.
    pub fn set_series(&mut self, series: impl Into<Series>) {
        self.series = series.into();
    }

    /// Sets the fill used to color the bars.
    ///
    /// The default is a solid [`series_color(0)`](crate::style::chart::series_color).
    pub fn set_fill(&mut self, fill: impl Into<Fill>) {
        self.fill = fill.into();
    }

    /// Sets the fill used to color the bars.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn fill(self, fill: impl Into<Fill>) -> Self {
        crate::With::with(self, |s| s.set_fill(fill))
    }

    /// Sets how the range of values is picked.
    ///
    /// The default is [`ValueRange::default`]: it starts at zero and fits the data.
    pub fn set_range(&mut self, range: impl Into<ValueRange>) {
        self.range = range.into();
    }

    /// Sets how the range of values is picked.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn range(self, range: impl Into<ValueRange>) -> Self {
        crate::With::with(self, |s| s.set_range(range))
    }
}

impl View for Sparkline {
    fn layout_key(&self) -> u64 {
        // The size only depends on the number of samples (and the constraint).
        crate::view::combine_layout_key(
            crate::view::layout_key_seed::<Self>(),
            &self.wanted_width(),
        )
    }

    fn needs_relayout(&self) -> bool {
        self.laid_out != Some(self.wanted_width())
    }

    fn layout(&mut self, _: Vec2) {
        self.laid_out = Some(self.wanted_width());
    }

    fn draw(&self, printer: &Printer) {
        let size = printer.size;
        if size.x == 0 || size.y == 0 {
            return;
        }

        let samples = self.series.tail(size.x);
        let bounds = self.range.resolve(samples.iter().copied());
        let fill = self.fill.resolve(printer);

        // The newest sample goes in the last column.
        let start = size.x - samples.len();
        for (i, &sample) in samples.iter().enumerate() {
            let column = start + i;
            let Some(ratio) = bounds.finite_ratio(sample) else {
                continue;
            };
            // Both the bar and the fill count rows from the bottom.
            printer.print_vbar_with((column, 0), size.y, ratio, |row| {
                ColorStyle::front(fill.color_at(FillPos::cell(column, row, size, ratio)))
            });
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2::new(self.wanted_width().min(constraint.x), 1)
    }
}

#[crate::blueprint(Sparkline::new(series))]
struct Blueprint {
    series: Series,

    fill: Option<Fill>,
    range: Option<ValueRange>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::Color;
    use crate::theme::Theme;
    use parking_lot::RwLock;

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);

    fn buffer_of(sparkline: &Sparkline, size: impl Into<Vec2>) -> RwLock<PrintBuffer> {
        let size = size.into();
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        sparkline.draw(&Printer::new(size, &Theme::retro(), &buffer));
        buffer
    }

    /// The lines of text, with `.` for the cells that were not printed.
    fn render(sparkline: &Sparkline, size: impl Into<Vec2>) -> Vec<String> {
        let size = size.into();
        let buffer = buffer_of(sparkline, size);
        let buffer = buffer.read();
        (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                        None | Some("") | Some(" ") => ".",
                        Some(text) => text,
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn glyphs_for_a_known_series() {
        let sparkline = Sparkline::new([0.0, 1.0, 2.0, 4.0, 8.0]).range(0.0..=8.0);
        assert_eq!(render(&sparkline, (5, 1)), [".▁▂▄█"]);

        let sparkline = Sparkline::new([0.0, 2.0, 4.0, 6.0, 8.0, 10.0]).range(0.0..=8.0);
        // Out-of-range values are clamped.
        assert_eq!(render(&sparkline, (6, 1)), [".▂▄▆██"]);
    }

    #[test]
    fn default_range_starts_at_zero() {
        let sparkline = Sparkline::new([10.0, 20.0, 40.0]);
        assert_eq!(render(&sparkline, (3, 1)), ["▂▄█"]);
    }

    #[test]
    fn newest_sample_is_on_the_right() {
        let sparkline = Sparkline::new([4.0, 8.0]).range(0.0..=8.0);
        assert_eq!(render(&sparkline, (5, 1)), ["...▄█"]);

        // With more samples than columns, only the newest are shown.
        let sparkline = Sparkline::new([1.0, 8.0, 4.0, 2.0]).range(0.0..=8.0);
        assert_eq!(render(&sparkline, (2, 1)), ["▄▂"]);
    }

    #[test]
    fn bounds_come_from_the_visible_samples() {
        let sparkline = Sparkline::new([100.0, 1.0, 2.0]).range(ValueRange::auto());
        // The first sample is hidden, so the range is 1..=2.
        assert_eq!(render(&sparkline, (2, 1)), [".█"]);
    }

    #[test]
    fn fills_several_rows() {
        let sparkline = Sparkline::new([0.0, 2.0, 4.0, 6.0, 8.0]).range(0.0..=8.0);
        assert_eq!(render(&sparkline, (5, 2)), ["...▄█", ".▄███"]);

        // 3 rows: 24 eighths in total.
        let sparkline = Sparkline::new([3.0, 12.0, 24.0]).range(0.0..=24.0);
        assert_eq!(render(&sparkline, (3, 3)), ["..█", ".▄█", "▃██"]);
    }

    #[test]
    fn empty_and_degenerate() {
        let sparkline = Sparkline::new(Series::new());
        assert_eq!(render(&sparkline, (3, 2)), ["...", "..."]);

        let sparkline = Sparkline::new([1.0]);
        assert!(
            render(&sparkline, (0, 2))
                .iter()
                .all(|line| line.is_empty())
        );
        assert!(render(&sparkline, (3, 0)).is_empty());
    }

    const NAN: f64 = f64::NAN;
    const INF: f64 = f64::INFINITY;
    const NEG_INF: f64 = f64::NEG_INFINITY;

    /// Inserts an empty column at each of the given (final) positions.
    fn with_gaps(rows: Vec<String>, gaps: &[usize]) -> Vec<String> {
        rows.into_iter()
            .map(|row| {
                let mut row = row.chars();
                let width = row.clone().count() + gaps.len();
                (0..width)
                    .map(|x| {
                        if gaps.contains(&x) {
                            '.'
                        } else {
                            row.next().unwrap()
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn invalid_samples_leave_empty_columns() {
        // NaN, +inf and -inf are not data: their column is empty, like in an area plot.
        let sparkline = Sparkline::new([NAN, 4.0, INF, 2.0, NEG_INF]).range(0.0..=4.0);
        assert_eq!(render(&sparkline, (5, 1)), [".█.▄."]);
        assert_eq!(render(&sparkline, (5, 2)), [".█...", ".█.█."]);
    }

    #[test]
    fn invalid_samples_do_not_change_the_scale() {
        let mixed = [2.0, NAN, 4.0, INF, NEG_INF, 3.0];
        let finite = [2.0, 4.0, 3.0];
        for range in [
            ValueRange::default(),
            ValueRange::auto(),
            ValueRange::auto().include(0.0).step(5.0).margin(0.2),
        ] {
            for height in 1..=3 {
                let expected = with_gaps(
                    render(&Sparkline::new(finite).range(range), (3, height)),
                    &[1, 3, 4],
                );
                let got = render(&Sparkline::new(mixed).range(range), (6, height));
                assert_eq!(got, expected, "{range:?} at height {height}");
            }
        }
        // Non-finite samples that are scrolled out of view change nothing either.
        let sparkline = Sparkline::new([INF, NEG_INF, 2.0, 4.0]).range(ValueRange::auto());
        let reference = Sparkline::new([2.0, 4.0]).range(ValueRange::auto());
        assert_eq!(render(&sparkline, (2, 1)), render(&reference, (2, 1)));
    }

    #[test]
    fn only_invalid_samples_draw_like_no_data() {
        let empty = render(&Sparkline::new(Series::new()), (4, 2));
        for samples in [[NAN; 4], [INF; 4], [NEG_INF; 4], [NAN, INF, NEG_INF, NAN]] {
            for range in [
                ValueRange::default(),
                ValueRange::auto(),
                (0.0..=1.0).into(),
            ] {
                let sparkline = Sparkline::new(samples).range(range);
                assert_eq!(render(&sparkline, (4, 2)), empty, "{samples:?} {range:?}");
            }
        }
    }

    #[test]
    fn invalid_samples_survive_every_size() {
        let sparkline = Sparkline::new([1.0, NAN, INF, 3.0, NEG_INF, NAN, 2.0]);
        for width in 0..=6 {
            for height in 0..=4 {
                render(&sparkline, (width, height));
            }
        }
    }

    #[test]
    fn solid_fill_color() {
        let sparkline = Sparkline::new([1.0, 2.0]).fill(WHITE).range(0.0..=2.0);
        let buffer = buffer_of(&sparkline, (2, 1));
        let buffer = buffer.read();
        for x in 0..2 {
            let style = buffer.cell_style(Vec2::new(x, 0)).unwrap();
            assert_eq!(style.color.front, WHITE);
        }
    }

    #[test]
    fn by_value_fill_colors() {
        let sparkline = Sparkline::new([0.0, 1.0, 2.0])
            .fill(Fill::by_value([BLACK, WHITE]))
            .range(0.0..=2.0);
        let buffer = buffer_of(&sparkline, (3, 1));
        let buffer = buffer.read();

        let front = |x| buffer.cell_style(Vec2::new(x, 0)).unwrap().color.front;
        // The empty sample is not printed.
        assert_eq!(buffer.cell_text(Vec2::new(0, 0)).unwrap_or(""), "");
        let Color::Rgb(mid, _, _) = front(1) else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&mid));
        assert_eq!(front(2), WHITE);
    }

    #[test]
    fn vertical_fill_goes_from_bottom_to_top() {
        let sparkline = Sparkline::new([4.0])
            .fill(Fill::vertical([BLACK, WHITE]))
            .range(0.0..=4.0);
        let buffer = buffer_of(&sparkline, (1, 2));
        let buffer = buffer.read();
        // The bottom row of the screen is the start of the gradient.
        let front = |y| buffer.cell_style(Vec2::new(0, y)).unwrap().color.front;
        assert_eq!(front(1), BLACK);
        assert_eq!(front(0), WHITE);
    }

    #[test]
    fn required_size() {
        let mut sparkline = Sparkline::new(Series::new());
        assert_eq!(sparkline.required_size(Vec2::new(10, 5)), Vec2::new(1, 1));

        sparkline.set_series([1.0, 2.0, 3.0]);
        assert_eq!(sparkline.required_size(Vec2::new(10, 5)), Vec2::new(3, 1));
        assert_eq!(sparkline.required_size(Vec2::new(2, 5)), Vec2::new(2, 1));
        assert_eq!(sparkline.required_size(Vec2::new(0, 5)), Vec2::new(0, 1));
    }

    #[test]
    fn shares_the_series() {
        let series = Series::new();
        let sparkline = Sparkline::new(&series).range(0.0..=8.0);
        assert_eq!(render(&sparkline, (2, 1)), [".."]);
        series.push(8.0);
        assert_eq!(render(&sparkline, (2, 1)), [".█"]);
        assert_eq!(sparkline.series().len(), 1);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            Sparkline:
                series: [0, 4, 8]
                range: [0, 8]
                fill: white
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let sparkline = view
            .downcast_ref::<Sparkline>()
            .expect("the view should be a Sparkline");
        assert_eq!(sparkline.series().to_vec(), [0.0, 4.0, 8.0]);
        assert_eq!(render(sparkline, (3, 1)), [".▄█"]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_with_a_stored_series() {
        let series = Series::new();
        let mut context = crate::builder::Context::new();
        context.store("data", series.clone());

        let config: crate::builder::Config =
            crate::reexports::serde_json::from_str(r#"{"Sparkline": {"series": "$data"}}"#)
                .unwrap();
        let view = context.build(&config).unwrap();
        series.push(1.0);
        let sparkline = view.downcast_ref::<Sparkline>().unwrap();
        assert_eq!(sparkline.series().to_vec(), [1.0]);
    }

    #[test]
    fn layout_is_cached() {
        crate::views::layout_key_tests::assert_layout_is_cached(|| Sparkline::new([1.0, 2.0]));
        crate::views::layout_key_tests::assert_layout_is_cached(|| Sparkline::new(Series::new()));
    }

    #[test]
    fn layout_key_follows_the_number_of_samples() {
        let series = Series::new();
        let mut sparkline = Sparkline::new(&series);
        let empty = sparkline.layout_key();
        sparkline.layout(Vec2::new(1, 1));
        assert!(!sparkline.needs_relayout());

        series.push(1.0);
        // An empty series asks for a column, like a single sample.
        assert_eq!(sparkline.layout_key(), empty);
        assert!(!sparkline.needs_relayout());

        series.push(2.0);
        assert_ne!(sparkline.layout_key(), empty);
        assert!(sparkline.needs_relayout());
        let two = sparkline.layout_key();
        assert_eq!(sparkline.required_size(Vec2::new(10, 5)), Vec2::new(2, 1));
        sparkline.layout(Vec2::new(2, 1));
        assert!(!sparkline.needs_relayout());

        // Replacing the series by another of the same length changes nothing.
        sparkline.set_series([5.0, 6.0]);
        assert_eq!(sparkline.layout_key(), two);
        assert!(!sparkline.needs_relayout());
        sparkline.set_series(Series::new());
        assert_eq!(sparkline.layout_key(), empty);
        assert!(sparkline.needs_relayout());
    }

    #[test]
    fn same_size_data_keeps_the_layout_key() {
        // A full series drops its oldest sample on each push: same size.
        let series = Series::with_capacity(3);
        series.extend([1.0, 2.0, 3.0]);
        let mut sparkline = Sparkline::new(&series);
        sparkline.layout(Vec2::new(3, 1));
        let key = sparkline.layout_key();
        series.push(9.0);
        sparkline.set_fill(Color::Rgb(255, 255, 255));
        sparkline.set_range(0.0..=1.0);
        assert_eq!(sparkline.layout_key(), key);
        assert!(!sparkline.needs_relayout());
    }
}
