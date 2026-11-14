use crate::style::ColorStyle;
use crate::style::chart::{AXIS, Fill, FillPos, ResolvedFill, fraction, series_color};
use crate::utils::{DotGrid, format_compact};
use crate::view::View;
use crate::views::{Series, ValueRange};
use crate::{Printer, Vec2};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

/// How a [`Plot`] is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlotKind {
    /// A thin line, drawn with braille dots. Each cell shows two samples.
    Line,
    /// Columns of blocks, filled from the bottom. Each cell shows one sample.
    Area,
}

impl PlotKind {
    /// Number of samples shown by one cell of the plot area.
    fn samples_per_cell(self) -> usize {
        match self {
            PlotKind::Line => 2,
            PlotKind::Area => 1,
        }
    }
}

/// One series in a [`Chart`], and how to draw it.
///
/// Options that only make sense for a kind of plot (or for a single plot) are set here; options
/// shared by the whole chart are on [`Chart`].
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::BaseColor;
/// use cursive_core::views::{Plot, PlotKind, Series};
///
/// let series = Series::with_capacity(60);
///
/// // A filled plot, in red, with a name for the legend.
/// let plot = Plot::area(&series)
///     .fill(BaseColor::Red.dark())
///     .label("requests");
///
/// assert_eq!(plot.kind(), PlotKind::Area);
/// assert_eq!(plot.get_label(), Some("requests"));
///
/// // The plot shares its data with `series`.
/// series.push(3.0);
/// assert_eq!(plot.series().last(), Some(3.0));
/// ```
#[derive(Clone, Debug)]
pub struct Plot {
    kind: PlotKind,
    series: Series,
    fill: Option<Fill>,
    label: Option<String>,
}

impl Plot {
    fn new(kind: PlotKind, series: impl Into<Series>) -> Self {
        Plot {
            kind,
            series: series.into(),
            fill: None,
            label: None,
        }
    }

    /// Creates a line plot.
    ///
    /// The line is drawn with braille dots, so a cell shows two samples. Consecutive samples
    /// are connected.
    ///
    /// Samples that are not finite leave a gap in the line: see
    /// [Invalid values](Series#invalid-values).
    pub fn line(series: impl Into<Series>) -> Self {
        Self::new(PlotKind::Line, series)
    }

    /// Creates an area plot.
    ///
    /// Each sample is a column of blocks, from the bottom of the chart up to the sample's value.
    /// A cell shows one sample.
    ///
    /// Samples that are not finite leave an empty column: see
    /// [Invalid values](Series#invalid-values).
    pub fn area(series: impl Into<Series>) -> Self {
        Self::new(PlotKind::Area, series)
    }

    /// Sets how this plot is colored.
    ///
    /// Anything that makes a [`Fill`] works, including a single color.
    ///
    /// If this is not set, the plot uses a default color that depends on its position in the
    /// chart: see [`series_color`].
    #[must_use]
    pub fn fill(mut self, fill: impl Into<Fill>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    /// Sets the name of this plot, shown in the chart's legend.
    ///
    /// Plots without a label are not in the legend.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns how this plot is drawn.
    pub fn kind(&self) -> PlotKind {
        self.kind
    }

    /// Returns the data of this plot.
    ///
    /// This is a handle to the same data as the one given when creating the plot.
    pub fn series(&self) -> &Series {
        &self.series
    }

    /// Returns the name shown in the legend, if any.
    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

type LabelFormatter = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A chart of one or several [`Plot`]s, with an optional axis and legend.
///
/// Time goes from left to right: the most recent sample of each series is on the right edge of
/// the plot, and a series with fewer samples than the chart is wide leaves the left side empty.
/// All the plots share the same vertical scale, picked by a [`ValueRange`] from the samples
/// that are visible.
///
/// # Data
///
/// A chart does not own its data. Each [`Plot`] holds a [`Series`], which is a handle to shared
/// data: keep a clone of the series, and push values to it whenever you want (possibly from
/// another thread). The chart reads the series every time it is drawn.
///
/// Adding values does not by itself redraw the screen. Either call
/// [`Cursive::set_fps`](crate::Cursive::set_fps) to refresh periodically, or trigger a refresh
/// from a callback sent through [`Cursive::cb_sink`](crate::Cursive::cb_sink).
///
/// Samples that are not finite numbers are not drawn and do not affect the scale: see
/// [Invalid values](Series#invalid-values).
///
/// # Size
///
/// A chart asks for a small fixed size (see [`required_size`](View::required_size)), no matter
/// its data. Change it with [`min_size`](Chart::min_size), or use [`full_screen`](crate::view::Resizable::full_screen) or
/// [`fixed_size`](crate::view::Resizable::fixed_size) to give it room.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::BaseColor;
/// use cursive_core::view::Resizable;
/// use cursive_core::views::{Chart, Plot, Series, ValueRange};
///
/// let requests = Series::with_capacity(100);
/// let errors = Series::with_capacity(100);
///
/// let chart = Chart::new()
///     // The first plot gets the first default color.
///     .plot(Plot::area(&requests).label("requests"))
///     .plot(
///         Plot::line(&errors)
///             .fill(BaseColor::Red.light())
///             .label("errors"),
///     )
///     // Always start at zero, and round the top to a multiple of 100.
///     .range(ValueRange::auto().include(0.0).step(100.0))
///     .legend(true)
///     .fixed_size((40, 8));
///
/// // Later, from anywhere:
/// requests.push(120.0);
/// errors.push(7.0);
/// # let _ = chart;
/// ```
pub struct Chart {
    plots: Vec<Plot>,
    range: ValueRange,
    y_labels: LabelFormatter,
    axis: bool,
    grid_lines: usize,
    legend: bool,
    min_size: Vec2,
    // The size we asked for the last time we were laid out.
    laid_out: Option<Vec2>,
}

impl Default for Chart {
    fn default() -> Self {
        Self::new()
    }
}

/// Position of the grid lines in a plot area of the given height.
///
/// Returns the row of each line, and its distance from the top as a fraction of the height
/// of the axis (0 is the maximum, 1 the minimum).
fn grid_rows(height: usize, lines: usize) -> Vec<(usize, f64)> {
    if height == 0 || lines == 0 {
        return Vec::new();
    }
    if lines == 1 {
        return vec![(0, 0.0)];
    }

    let mut rows: Vec<(usize, f64)> = Vec::with_capacity(lines);
    for i in 0..lines {
        // Evenly spread, with the middle line of 3 on row `height / 2`.
        let row = (2 * (height - 1) * i + (lines - 1)) / (2 * (lines - 1));
        let t = i as f64 / (lines - 1) as f64;
        match rows.last_mut() {
            // Not enough rows. Keep the first one, but the last line always stays at the bottom.
            Some(last) if last.0 == row => {
                if i == lines - 1 {
                    *last = (row, t);
                }
            }
            _ => rows.push((row, t)),
        }
    }
    rows
}

/// Value at the given distance from the top of the axis, where 0 is the max and 1 the min.
fn value_at(bounds: crate::views::Bounds, t: f64) -> f64 {
    if t <= 0.0 {
        bounds.max
    } else if t >= 1.0 {
        bounds.min
    } else {
        bounds.max - (bounds.max - bounds.min) * t
    }
}

impl Chart {
    /// Creates a new empty chart.
    ///
    /// By default the chart has an axis with 3 grid lines, no legend, and uses
    /// [`ValueRange::default`] (automatic, including zero).
    pub fn new() -> Self {
        Chart {
            plots: Vec::new(),
            range: ValueRange::default(),
            y_labels: Arc::new(format_compact),
            axis: true,
            grid_lines: 3,
            legend: false,
            min_size: Vec2::new(20, 5),
            laid_out: None,
        }
    }

    /// Adds a plot to this chart.
    ///
    /// Plots are drawn in the order they were added: later plots are on top of earlier ones.
    #[must_use]
    pub fn plot(mut self, plot: Plot) -> Self {
        self.add_plot(plot);
        self
    }

    /// Adds a plot to this chart.
    ///
    /// Plots are drawn in the order they were added: later plots are on top of earlier ones.
    pub fn add_plot(&mut self, plot: Plot) {
        self.plots.push(plot);
    }

    /// Removes all the plots from this chart.
    pub fn clear_plots(&mut self) {
        self.plots.clear();
    }

    /// Returns the plots of this chart, in drawing order.
    pub fn plots(&self) -> &[Plot] {
        &self.plots
    }

    /// Sets the range of values shown by this chart.
    ///
    /// The range is resolved for each draw, from the samples that are visible in all the plots.
    ///
    /// Default is [`ValueRange::default`].
    #[must_use]
    pub fn range(mut self, range: impl Into<ValueRange>) -> Self {
        self.set_range(range);
        self
    }

    /// Sets the range of values shown by this chart.
    ///
    /// See [`range`](Self::range).
    pub fn set_range(&mut self, range: impl Into<ValueRange>) {
        self.range = range.into();
    }

    /// Sets how the values on the axis are written.
    ///
    /// The formatter may be called several times for each draw.
    ///
    /// The default gives compact labels of about 5 characters or less, like `0`, `50`, `1.0k` or
    /// `1.5M`.
    #[must_use]
    pub fn y_labels(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.set_y_labels(format);
        self
    }

    /// Sets how the values on the axis are written.
    ///
    /// See [`y_labels`](Self::y_labels).
    pub fn set_y_labels(&mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) {
        self.y_labels = Arc::new(format);
    }

    /// Shows or hides the axis: the value labels, and the horizontal grid lines.
    ///
    /// Without an axis, the plots take the whole area.
    ///
    /// If the labels are so wide that no column is left for the plots, the axis is dropped
    /// (labels and grid lines) and the plots take the whole width instead.
    ///
    /// Default is `true`.
    #[must_use]
    pub fn axis(mut self, axis: bool) -> Self {
        self.set_axis(axis);
        self
    }

    /// Shows or hides the axis.
    ///
    /// See [`axis`](Self::axis).
    pub fn set_axis(&mut self, axis: bool) {
        self.axis = axis;
    }

    /// Sets the number of horizontal grid lines, each with a value label.
    ///
    /// The lines are evenly spread from the top row (the highest value) to the bottom row (the
    /// lowest). A single line is drawn on the top row. If there are not enough rows, some lines
    /// are skipped. With `0`, there is no axis at all.
    ///
    /// Default is `3`.
    #[must_use]
    pub fn grid_lines(mut self, lines: usize) -> Self {
        self.set_grid_lines(lines);
        self
    }

    /// Sets the number of horizontal grid lines.
    ///
    /// See [`grid_lines`](Self::grid_lines).
    pub fn set_grid_lines(&mut self, lines: usize) {
        self.grid_lines = lines;
    }

    /// Shows or hides the legend.
    ///
    /// The legend is drawn on the top row of the plot area. It lists the plots that have a
    /// [label](Plot::label), as long as they fit.
    ///
    /// Default is `false`.
    #[must_use]
    pub fn legend(mut self, legend: bool) -> Self {
        self.set_legend(legend);
        self
    }

    /// Shows or hides the legend.
    ///
    /// See [`legend`](Self::legend).
    pub fn set_legend(&mut self, legend: bool) {
        self.legend = legend;
    }

    /// Sets the size the chart asks for.
    ///
    /// This is the size returned by [`required_size`](View::required_size): the chart does not
    /// depend on its data or on the available space, so this is the size it gets unless a
    /// parent view gives it another one (see [`Resizable`](crate::view::Resizable)).
    /// For example, in a [`LinearLayout`](crate::views::LinearLayout), a chart with a larger
    /// minimum size claims more room than the default.
    ///
    /// Default is 20 columns by 5 rows.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::Vec2;
    /// use cursive_core::view::View;
    /// use cursive_core::views::Chart;
    ///
    /// let mut chart = Chart::new().min_size((40, 10));
    /// assert_eq!(chart.required_size(Vec2::new(100, 100)), Vec2::new(40, 10));
    /// ```
    #[must_use]
    pub fn min_size(mut self, size: impl Into<Vec2>) -> Self {
        self.set_min_size(size);
        self
    }

    /// Sets the size the chart asks for.
    ///
    /// See [`min_size`](Self::min_size).
    pub fn set_min_size(&mut self, size: impl Into<Vec2>) {
        self.min_size = size.into();
    }

    /// Resolves the bounds shared by all plots, for the given width of the plot area.
    fn bounds(&self, samples: &[Vec<f64>], plot_width: usize) -> crate::views::Bounds {
        let visible = self.plots.iter().zip(samples).flat_map(|(plot, samples)| {
            let count = plot.kind.samples_per_cell() * plot_width;
            samples[samples.len().saturating_sub(count)..]
                .iter()
                .copied()
        });
        self.range.resolve(visible)
    }

    /// Draws a filled plot: one column of blocks per sample.
    fn draw_area(
        printer: &Printer,
        values: &[f64],
        bounds: crate::views::Bounds,
        fill: &ResolvedFill,
    ) {
        let size = printer.size;
        let visible = &values[values.len().saturating_sub(size.x)..];
        let start = size.x - visible.len();
        for (i, &value) in visible.iter().enumerate() {
            let Some(ratio) = bounds.finite_ratio(value) else {
                continue;
            };
            let column = start + i;
            printer.print_vbar_with((column, 0), size.y, ratio, |row| {
                ColorStyle::front(fill.color_at(FillPos::cell(column, row, size, ratio)))
            });
        }
    }

    /// Adds a line plot to a grid of braille dots.
    fn draw_line(
        grid: &mut DotGrid,
        values: &[f64],
        bounds: crate::views::Bounds,
        fill: &ResolvedFill,
    ) {
        let dots = grid.resolution();
        let visible = &values[values.len().saturating_sub(dots.x)..];
        let start = dots.x - visible.len();

        let mut previous = None;
        for (i, &value) in visible.iter().enumerate() {
            let Some(ratio) = bounds.finite_ratio(value) else {
                // Leave a gap.
                previous = None;
                continue;
            };
            let x = start + i;
            let y = dots.y - 1 - (ratio * (dots.y - 1) as f32).round() as usize;
            let color = fill.color_at(FillPos {
                x: fraction(x, dots.x),
                y: fraction(dots.y - 1 - y, dots.y),
                value: ratio,
            });
            // Connect to the previous sample so the line has no gaps.
            grid.line((x, previous.unwrap_or(y)), (x, y), color);
            previous = Some(y);
        }
    }
}

impl View for Chart {
    fn layout_key(&self) -> u64 {
        // The minimum size is all that decides the size: not the data, the labels or the legend.
        crate::view::combine_layout_key(crate::view::layout_key_seed::<Self>(), &self.min_size)
    }

    fn needs_relayout(&self) -> bool {
        self.laid_out != Some(self.min_size)
    }

    fn layout(&mut self, _: Vec2) {
        self.laid_out = Some(self.min_size);
    }

    fn draw(&self, printer: &Printer) {
        let size = printer.size;
        if size.x == 0 || size.y == 0 {
            return;
        }

        // Lock each series only once.
        let samples: Vec<Vec<f64>> = self
            .plots
            .iter()
            .map(|plot| plot.series.tail(plot.kind.samples_per_cell() * size.x))
            .collect();

        // The width of the labels depends on the bounds, which depend on the visible samples,
        // which depend on the width of the plot area. Only grow the label column until it fits.
        let mut lines = if self.axis {
            grid_rows(size.y, self.grid_lines)
        } else {
            Vec::new()
        };
        let mut label_column = 0;
        let (bounds, labels, plot_width) = loop {
            let plot_width = size.x.saturating_sub(label_column);
            if plot_width == 0 {
                // The labels take the whole width: drop the axis, and start again with
                // the full width for the plots.
                lines.clear();
                label_column = 0;
                continue;
            }
            let bounds = self.bounds(&samples, plot_width);
            let labels: Vec<String> = lines
                .iter()
                .map(|&(_, t)| (self.y_labels)(value_at(bounds, t)))
                .collect();
            let needed = labels
                .iter()
                .map(|label| label.width() + 1)
                .max()
                .unwrap_or(0);
            if needed <= label_column {
                break (bounds, labels, plot_width);
            }
            label_column = needed;
        };

        // Axis
        printer.with_color(ColorStyle::front(&AXIS), |printer| {
            for (&(row, _), label) in lines.iter().zip(&labels) {
                printer.print((label_column - 1 - label.width(), row), label);
                printer.print_hline((label_column, row), plot_width, "┄");
            }
        });

        // Plots
        let plot_size = Vec2::new(plot_width, size.y);
        let printer = printer.offset((label_column, 0)).cropped(plot_size);
        let fills: Vec<ResolvedFill> = self
            .plots
            .iter()
            .enumerate()
            .map(|(index, plot)| match &plot.fill {
                Some(fill) => fill.resolve(&printer),
                None => Fill::solid(series_color(index)).resolve(&printer),
            })
            .collect();

        // All the consecutive line plots share a grid.
        let mut grid: Option<DotGrid> = None;
        for ((plot, values), fill) in self.plots.iter().zip(&samples).zip(&fills) {
            match plot.kind {
                PlotKind::Area => {
                    if let Some(grid) = grid.take() {
                        grid.draw(&printer);
                    }
                    Self::draw_area(&printer, values, bounds, fill);
                }
                PlotKind::Line => {
                    let grid = grid.get_or_insert_with(|| DotGrid::braille(plot_size));
                    Self::draw_line(grid, values, bounds, fill);
                }
            }
        }
        if let Some(grid) = grid {
            grid.draw(&printer);
        }

        // Legend
        if self.legend {
            let mut x = 1;
            for (plot, fill) in self.plots.iter().zip(&fills) {
                let Some(label) = &plot.label else {
                    continue;
                };
                let text = format!("━ {label}");
                let width = text.width();
                if x + width > plot_width {
                    break;
                }
                let color = fill.color_at(FillPos {
                    x: 1.0,
                    y: 1.0,
                    value: 1.0,
                });
                printer.with_color(ColorStyle::front(color), |printer| {
                    printer.print((x, 0), &text);
                });
                x += width + 2;
            }
        }
    }

    /// A chart asks for its [minimum size](Chart::min_size) (by default 20 columns and 5
    /// rows), whatever the data and the constraint.
    ///
    /// Use [`Resizable`](crate::view::Resizable) to give it another size.
    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        self.min_size
    }
}

#[crate::blueprint(Chart::new())]
struct Blueprint {
    #[blueprint(foreach = add_plot)]
    plots: Vec<Plot>,

    range: Option<ValueRange>,
    axis: Option<bool>,
    grid_lines: Option<usize>,
    legend: Option<bool>,
    min_size: Option<Vec2>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::chart::SERIES;
    use crate::style::{Color, ColorKey};
    use crate::theme::Theme;
    use crate::views::{Heatmap, Sparkline};
    use parking_lot::RwLock;

    /// Draws the chart in a buffer of the given size.
    fn draw(chart: &Chart, size: (usize, usize)) -> RwLock<PrintBuffer> {
        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(size.into());
        let buffer = RwLock::new(buffer);
        chart.draw(&Printer::new(size, &theme, &buffer));
        buffer
    }

    /// The text of each row. Cells that were not printed are spaces.
    fn rows(chart: &Chart, size: (usize, usize)) -> Vec<String> {
        let buffer = draw(chart, size);
        let buffer = buffer.read();
        (0..size.1)
            .map(|y| {
                (0..size.0)
                    .map(|x| buffer.cell_text(Vec2::new(x, y)).unwrap_or(" ").to_string())
                    .collect()
            })
            .collect()
    }

    /// The front color of a printed cell.
    fn front(chart: &Chart, size: (usize, usize), pos: (usize, usize)) -> Color {
        let buffer = draw(chart, size);
        let buffer = buffer.read();
        buffer
            .cell_text(pos.into())
            .expect("cell should be printed");
        buffer
            .cell_style(pos.into())
            .expect("cell should be printed")
            .color
            .front
    }

    /// What a palette key resolves to in the theme used by the tests.
    fn resolved(key: &'static ColorKey) -> Color {
        let buffer = RwLock::new(PrintBuffer::new());
        Printer::new((1, 1), &Theme::retro(), &buffer).resolve_color(key)
    }

    /// A chart without axis, showing a fixed range.
    fn bare(min: f64, max: f64) -> Chart {
        Chart::new().axis(false).range(ValueRange::fixed(min, max))
    }

    /// A chart without axis, with the default range.
    fn bare_auto() -> Chart {
        Chart::new().axis(false)
    }

    #[test]
    fn grid_row_positions() {
        // Like the showcase: top, `height / 2` and bottom.
        for height in 3..20 {
            let rows: Vec<usize> = grid_rows(height, 3).iter().map(|&(row, _)| row).collect();
            assert_eq!(rows, [0, height / 2, height - 1], "height {height}");
        }
        assert_eq!(grid_rows(5, 1), [(0, 0.0)]);
        assert_eq!(grid_rows(5, 2), [(0, 0.0), (4, 1.0)]);
        assert!(grid_rows(5, 0).is_empty());
        assert!(grid_rows(0, 3).is_empty());
        // Not enough rows: lines that share a row are skipped, the bottom one is kept.
        assert_eq!(grid_rows(2, 3), [(0, 0.0), (1, 1.0)]);
        assert_eq!(grid_rows(1, 3), [(0, 1.0)]);
        assert_eq!(grid_rows(2, 5).len(), 2);
    }

    #[test]
    fn area_glyphs() {
        // Height 2 gives 16 levels: values 0, 4, 8, 12 and 16 of 16.
        let chart = bare(0.0, 16.0).plot(Plot::area([0.0, 4.0, 8.0, 12.0, 16.0]));
        assert_eq!(rows(&chart, (5, 2)), ["   ▄█", " ▄███"]);

        // When there is not enough room, the oldest samples are not shown.
        assert_eq!(rows(&chart, (3, 2)), [" ▄█", "███"]);

        // Values outside of the range are clamped.
        let chart = bare(4.0, 8.0).plot(Plot::area([0.0, 6.0, 100.0]));
        assert_eq!(rows(&chart, (3, 1)), [" ▄█"]);
    }

    #[test]
    fn area_skips_non_finite_samples() {
        // Like lines: an infinite sample is not drawn as a full (or empty) bar.
        let chart = bare(0.0, 8.0).plot(Plot::area([
            8.0,
            f64::INFINITY,
            4.0,
            f64::NEG_INFINITY,
            f64::NAN,
        ]));
        assert_eq!(rows(&chart, (5, 1)), ["█ ▄  "]);
    }

    #[test]
    fn area_newest_sample_is_on_the_right() {
        let chart = bare(0.0, 8.0).plot(Plot::area([8.0, 4.0]));
        // Two samples in six columns: the left side is empty.
        assert_eq!(rows(&chart, (6, 1)), ["    █▄"]);
    }

    #[test]
    fn line_glyphs() {
        // A 4x1 area has 8x4 dots. A flat line at the top is on the first row of dots.
        // Two samples per cell, so 4 samples fill the two cells on the right.
        let chart = bare(0.0, 3.0).plot(Plot::line([3.0, 3.0, 3.0, 3.0]));
        assert_eq!(rows(&chart, (4, 1)), ["  ⠉⠉"]);

        // Flat at the bottom: the last row of dots.
        let chart = bare(0.0, 3.0).plot(Plot::line([0.0, 0.0, 0.0, 0.0]));
        assert_eq!(rows(&chart, (4, 1)), ["  ⣀⣀"]);

        // From the bottom to the top: the left dot is at the bottom, and the right column is
        // lit from the top to the previous sample, so there is no gap.
        let chart = bare(0.0, 3.0).plot(Plot::line([0.0, 3.0]));
        assert_eq!(rows(&chart, (1, 1)), ["⣸"]);

        // Up and down: the line goes through the full height in the middle cell.
        let chart = bare(0.0, 3.0).plot(Plot::line([0.0, 3.0, 0.0]));
        assert_eq!(rows(&chart, (2, 1)), ["⢀⣿"]);
    }

    #[test]
    fn line_newest_sample_is_on_the_right() {
        // A single sample is the top right dot of the last cell.
        let chart = bare(0.0, 3.0).plot(Plot::line([3.0]));
        assert_eq!(rows(&chart, (3, 1)), ["  ⠈"]);
    }

    #[test]
    fn line_skips_invalid_samples() {
        // The gap is not connected. Dots: bottom left, nothing, top right.
        let chart = bare(0.0, 3.0).plot(Plot::line([0.0, f64::NAN, 3.0]));
        assert_eq!(rows(&chart, (2, 1)), ["⢀⠈"]);
    }

    const NAN: f64 = f64::NAN;
    const INF: f64 = f64::INFINITY;
    const NEG_INF: f64 = f64::NEG_INFINITY;

    #[test]
    fn invalid_samples_do_not_change_the_scale() {
        // Area: the scale is the one of the finite samples (0 to 4 with the default range).
        let mixed = [2.0, NAN, 4.0, INF, NEG_INF, 3.0];
        let area = bare_auto().plot(Plot::area(mixed));
        let reference = bare(0.0, 4.0).plot(Plot::area(mixed));
        assert_eq!(rows(&area, (6, 2)), rows(&reference, (6, 2)));
        assert_eq!(rows(&area, (6, 2)), ["  █  ▄", "█ █  █"]);

        // Line.
        let line = bare_auto().plot(Plot::line(mixed));
        let reference = bare(0.0, 4.0).plot(Plot::line(mixed));
        assert_eq!(rows(&line, (3, 2)), rows(&reference, (3, 2)));

        // The axis only depends on the finite samples.
        for plot in [Plot::area, Plot::line] {
            let mixed = Chart::new().plot(plot(vec![40.0, NAN, INF, NEG_INF, 100.0]));
            let finite = Chart::new().plot(plot(vec![40.0, 100.0]));
            let labels = |chart: &Chart| -> Vec<String> {
                rows(chart, (12, 5))
                    .iter()
                    .map(|row| row.chars().take(5).collect())
                    .collect()
            };
            assert_eq!(labels(&mixed), labels(&finite));
            assert!(labels(&mixed)[0].contains("100"));
        }
    }

    #[test]
    fn only_invalid_samples_draw_like_no_data() {
        let empty = Chart::new()
            .plot(Plot::area(Series::new()).label("a"))
            .plot(Plot::line(Series::new()).label("b"))
            .legend(true);
        let invalid = Chart::new()
            .plot(Plot::area([NAN, INF, NEG_INF]).label("a"))
            .plot(Plot::line([NAN, NAN, INF, NEG_INF]).label("b"))
            .legend(true);
        assert_eq!(rows(&invalid, (20, 5)), rows(&empty, (20, 5)));

        // They do not disturb the scale, the axis or the legend of the other plots either.
        let with = Chart::new()
            .plot(Plot::area([1.0, 5.0, 3.0]).label("a"))
            .plot(Plot::line([INF, NAN, NEG_INF]).label("b"))
            .legend(true);
        let without = Chart::new()
            .plot(Plot::area([1.0, 5.0, 3.0]).label("a"))
            .plot(Plot::line(Series::new()).label("b"))
            .legend(true);
        assert_eq!(rows(&with, (20, 5)), rows(&without, (20, 5)));
        assert!(rows(&with, (20, 5))[0].contains('a') && rows(&with, (20, 5))[0].contains('b'));
    }

    #[test]
    fn invalid_samples_survive_every_size() {
        let chart = Chart::new()
            .plot(Plot::area([1.0, NAN, INF, 3.0, NEG_INF]).label("a"))
            .plot(Plot::line([NAN, 2.0, INF, NEG_INF, NAN, 1.0]).label("b"))
            .plot(Plot::line([NAN; 3]))
            .legend(true);
        for width in 0..=6 {
            for height in 0..=4 {
                draw(&chart, (width, height));
            }
        }
    }

    #[test]
    fn axis_labels_and_lines() {
        let chart = Chart::new().range(ValueRange::fixed(0.0, 100.0));
        assert_eq!(
            rows(&chart, (10, 5)),
            [
                "100 ┄┄┄┄┄┄",
                "          ",
                " 50 ┄┄┄┄┄┄",
                "          ",
                "  0 ┄┄┄┄┄┄",
            ]
        );

        // Fewer lines
        let chart = chart.grid_lines(2);
        assert_eq!(rows(&chart, (8, 3)), ["100 ┄┄┄┄", "        ", "  0 ┄┄┄┄"]);
        let chart = chart.grid_lines(1);
        assert_eq!(rows(&chart, (8, 2)), ["100 ┄┄┄┄", "        "]);

        // Too many lines for the height.
        let chart = chart.grid_lines(5);
        assert_eq!(rows(&chart, (8, 2)), ["100 ┄┄┄┄", "  0 ┄┄┄┄"]);
    }

    #[test]
    fn no_axis() {
        for chart in [
            Chart::new()
                .range(ValueRange::fixed(0.0, 100.0))
                .axis(false),
            Chart::new()
                .range(ValueRange::fixed(0.0, 100.0))
                .grid_lines(0),
        ] {
            assert_eq!(rows(&chart, (6, 3)), ["      "; 3]);
        }

        // The plot takes the whole width.
        let chart = bare(0.0, 8.0).plot(Plot::area([8.0]));
        assert_eq!(rows(&chart, (3, 1)), ["  █"]);
        let chart = Chart::new()
            .range(ValueRange::fixed(0.0, 8.0))
            .grid_lines(0)
            .plot(Plot::area([8.0]));
        assert_eq!(rows(&chart, (3, 1)), ["  █"]);
    }

    #[test]
    fn custom_labels() {
        let chart = Chart::new()
            .range(ValueRange::fixed(0.0, 100.0))
            .y_labels(|value| format!("{value:.0}%"));
        assert_eq!(
            rows(&chart, (9, 3)),
            ["100% ┄┄┄┄", " 50% ┄┄┄┄", "  0% ┄┄┄┄"]
        );

        // The formatter can be changed afterwards, and gets the value of each line.
        let mut chart = Chart::new().range(ValueRange::fixed(-10.0, 10.0));
        chart.set_y_labels(|value| format!("<{value}>"));
        assert_eq!(
            rows(&chart, (10, 3)),
            [" <10> ┄┄┄┄", "  <0> ┄┄┄┄", "<-10> ┄┄┄┄"]
        );
    }

    #[test]
    fn empty_chart() {
        // The default range includes zero, and is at least one unit tall.
        let chart = Chart::new();
        let expected = ["  1 ┄┄┄┄┄┄", "0.5 ┄┄┄┄┄┄", "  0 ┄┄┄┄┄┄"];
        assert_eq!(rows(&chart, (10, 3)), expected);

        // Same with plots that have no samples.
        let chart = Chart::new()
            .plot(Plot::area(Series::new()))
            .plot(Plot::line(Series::new()));
        assert_eq!(rows(&chart, (10, 3)), expected);
    }

    #[test]
    fn plots_are_drawn_over_the_grid() {
        // Labels are 1 character, so the plot is 4 cells wide.
        let chart = Chart::new()
            .range(ValueRange::fixed(0.0, 2.0))
            .plot(Plot::area([2.0, 1.0]));
        assert_eq!(rows(&chart, (6, 3)), ["2 ┄┄█┄", "1 ┄┄█▄", "0 ┄┄██"]);
    }

    #[test]
    fn labels_change_the_plot_width() {
        // The label column takes space from the plot, and the samples that remain visible decide
        // the bounds, so the width of the labels follows them. Here the large oldest sample is
        // only visible when there is no label column, and its labels are as wide as "1000".
        let chart = Chart::new()
            .grid_lines(2)
            .plot(Plot::area([1000.0, 5.0, 5.0, 5.0]));
        // Width 5: labels of "1.0k" would take 5 columns and leave none for the plot, so the
        // axis is dropped and the plot takes the whole width (including the large sample).
        assert_eq!(rows(&chart, (5, 2)), [" █   "; 2]);
        // Width 6: the labels took 5 columns, leaving one for the plot, which then only shows
        // the last sample. The bounds are now 0 to 5, but the label column does not shrink back.
        assert_eq!(rows(&chart, (6, 2)), ["   5 █", "   0 █"]);
    }

    #[test]
    fn too_small() {
        let chart = Chart::new()
            .range(ValueRange::fixed(0.0, 100.0))
            .plot(Plot::area([1.0, 2.0]))
            .plot(Plot::line([1.0, 2.0]))
            .legend(true);
        draw(&chart, (0, 0));
        draw(&chart, (0, 4));
        draw(&chart, (4, 0));

        // The labels are 3 characters, and need a space and a column for the plot: without
        // that much room, the axis is dropped and the plots are drawn across the width.
        for width in 1..=4 {
            let rows = rows(&chart, (width, 3));
            let text = rows.concat();
            assert!(!text.contains(['0', '1', '┄']), "width {width}: {rows:?}");
            assert_ne!(rows[2].trim(), "", "width {width}: {rows:?}");
        }
        assert_eq!(rows(&chart, (5, 3))[0], "100 ┄");
    }

    #[test]
    fn legend() {
        let explicit = Color::Rgb(1, 2, 3);
        let chart = bare(0.0, 1.0)
            .legend(true)
            .plot(Plot::area(Series::new()).label("cpu"))
            // No label: not in the legend, but still counts for the default color.
            .plot(Plot::line(Series::new()))
            .plot(Plot::line(Series::new()).fill(explicit).label("mem"))
            .plot(Plot::area(Series::new()).label("disk"));
        let size = (24, 2);
        assert_eq!(rows(&chart, size)[0], " ━ cpu  ━ mem  ━ disk   ");
        assert_eq!(rows(&chart, size)[1], " ".repeat(24));

        // Each entry has the color of its plot.
        assert_eq!(front(&chart, size, (1, 0)), resolved(series_color(0)));
        assert_eq!(front(&chart, size, (4, 0)), resolved(series_color(0)));
        assert_eq!(front(&chart, size, (10, 0)), explicit);
        assert_eq!(front(&chart, size, (17, 0)), resolved(series_color(3)));

        // An entry that does not fit is dropped, with all the following ones.
        assert_eq!(rows(&chart, (15, 2))[0], " ━ cpu  ━ mem  ");
        assert_eq!(rows(&chart, (14, 2))[0], " ━ cpu  ━ mem ");
        assert_eq!(rows(&chart, (12, 2))[0], " ━ cpu      ");
        assert_eq!(rows(&chart, (6, 2))[0], " ━ cpu");
        assert_eq!(rows(&chart, (5, 2))[0], "     ");

        // Hidden by default.
        let chart = Chart::new()
            .axis(false)
            .plot(Plot::area(Series::new()).label("cpu"));
        assert_eq!(rows(&chart, (8, 1)), ["        "]);
    }

    #[test]
    fn legend_is_in_the_plot_area() {
        let chart = Chart::new()
            .range(ValueRange::fixed(0.0, 10.0))
            .grid_lines(2)
            .legend(true)
            .plot(Plot::area(Series::new()).label("a"));
        assert_eq!(rows(&chart, (9, 2)), ["10 ┄━ a┄┄", " 0 ┄┄┄┄┄┄"]);
    }

    #[test]
    fn default_and_explicit_colors() {
        assert_ne!(resolved(series_color(0)), resolved(series_color(1)));

        let chart = bare(0.0, 8.0)
            .plot(Plot::area([8.0, 0.0, 0.0]))
            .plot(Plot::line([0.0, 0.0, 8.0, 8.0]));
        // The area is in the first color, the line in the second one.
        assert_eq!(rows(&chart, (3, 1)), ["█⣀⡏"]);
        assert_eq!(front(&chart, (3, 1), (0, 0)), resolved(series_color(0)));
        assert_eq!(front(&chart, (3, 1), (1, 0)), resolved(series_color(1)));
        assert_eq!(front(&chart, (3, 1), (2, 0)), resolved(series_color(1)));

        // Explicit fills.
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let chart = bare(0.0, 8.0)
            .plot(Plot::area([8.0, 0.0]).fill(red))
            .plot(Plot::area([0.0, 8.0]).fill(blue));
        assert_eq!(front(&chart, (2, 1), (0, 0)), red);
        assert_eq!(front(&chart, (2, 1), (1, 0)), blue);

        // Default colors cycle.
        let mut chart = bare(0.0, 8.0);
        for _ in 0..=SERIES.len() {
            chart.add_plot(Plot::area(Series::new()));
        }
        chart.add_plot(Plot::area([8.0]));
        assert_eq!(
            front(&chart, (1, 1), (0, 0)),
            resolved(series_color(SERIES.len() + 1))
        );
    }

    #[test]
    fn gradients() {
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(255, 255, 255);

        // Vertical gradient: the top cell is brighter than the bottom one.
        let chart = bare(0.0, 8.0).plot(Plot::area([8.0]).fill(Fill::vertical([black, white])));
        assert_eq!(front(&chart, (1, 2), (0, 1)), black);
        assert_eq!(front(&chart, (1, 2), (0, 0)), white);

        // By value: the line is white where it reaches the max.
        let chart = bare(0.0, 3.0).plot(Plot::line([3.0]).fill(Fill::by_value([black, white])));
        assert_eq!(front(&chart, (1, 1), (0, 0)), white);
        let chart = bare(0.0, 3.0).plot(Plot::line([0.0]).fill(Fill::by_value([black, white])));
        assert_eq!(front(&chart, (1, 1), (0, 0)), black);
    }

    #[test]
    fn plots_share_the_bounds() {
        // Independent scales would draw the first plot at full height.
        let chart = bare_auto()
            .plot(Plot::area([50.0, 0.0]))
            .plot(Plot::area([0.0, 100.0]));
        assert_eq!(rows(&chart, (2, 1)), ["▄█"]);

        // Lines and areas too.
        let chart = bare_auto()
            .plot(Plot::area([100.0, 50.0]))
            .plot(Plot::line([0.0, 0.0]));
        assert_eq!(rows(&chart, (2, 1)), ["█⣀"]);
    }

    #[test]
    fn bounds_only_use_visible_samples() {
        // Only 2 columns, so the large value is out of sight: the bounds are 0 to 10.
        let chart = bare_auto().plot(Plot::area([1000.0, 10.0, 10.0]));
        assert_eq!(rows(&chart, (2, 1)), ["██"]);

        // Lines show 2 samples per column.
        let chart = bare_auto().plot(Plot::line([1000.0, 10.0, 10.0]));
        assert_eq!(rows(&chart, (1, 1)), ["⠉"]);
        let chart = bare_auto().plot(Plot::line([1000.0, 10.0, 10.0, 10.0]));
        assert_eq!(rows(&chart, (1, 1)), ["⠉"]);
        let chart = bare_auto().plot(Plot::line([1000.0, 10.0]));
        assert_ne!(rows(&chart, (1, 1)), ["⠉"]);
    }

    #[test]
    fn plots_are_drawn_in_order() {
        let line = || Plot::line([1.0, 1.0]);
        let area = || Plot::area([1.0]);

        // The area is last: it replaces the line.
        let chart = bare(0.0, 1.0).plot(line()).plot(area());
        assert_eq!(rows(&chart, (1, 1)), ["█"]);
        // The line is last, and replaces the area.
        let chart = bare(0.0, 1.0).plot(area()).plot(line());
        assert_eq!(rows(&chart, (1, 1)), ["⠉"]);

        // Lines before and after an area are not drawn together: the later one is on top.
        let chart = bare(0.0, 1.0)
            .plot(Plot::line([1.0, 1.0]))
            .plot(area())
            .plot(Plot::line([0.0, 0.0]));
        assert_eq!(rows(&chart, (1, 1)), ["⣀"]);
    }

    #[test]
    fn manage_plots() {
        let series = Series::new();
        let mut chart = Chart::default();
        assert!(chart.plots().is_empty());
        chart.add_plot(Plot::line(&series));
        chart = chart.plot(Plot::area(&series).label("a"));
        assert_eq!(chart.plots().len(), 2);
        assert_eq!(chart.plots()[0].kind(), PlotKind::Line);
        assert_eq!(chart.plots()[0].get_label(), None);
        assert_eq!(chart.plots()[1].kind(), PlotKind::Area);
        assert_eq!(chart.plots()[1].get_label(), Some("a"));

        // Plots share the series.
        series.push(1.0);
        assert_eq!(chart.plots()[1].series().to_vec(), [1.0]);

        chart.clear_plots();
        assert!(chart.plots().is_empty());
    }

    #[test]
    fn data_added_later_is_drawn() {
        let series = Series::new();
        let chart = bare(0.0, 8.0).plot(Plot::area(&series));
        assert_eq!(rows(&chart, (3, 1)), ["   "]);
        series.extend([8.0, 4.0]);
        assert_eq!(rows(&chart, (3, 1)), [" █▄"]);
    }

    #[test]
    fn labels_without_room_for_the_plot_are_dropped() {
        // "1.0k" and a space take 5 columns: the plot would have no room at all.
        let chart = Chart::new().plot(Plot::area([1000.0, 1000.0]));
        assert_eq!(rows(&chart, (5, 2)), ["   ██", "   ██"]);
        assert_eq!(rows(&chart, (4, 2)), ["  ██", "  ██"]);

        // One more column and the axis comes back, with its labels and lines.
        let with_axis = rows(&chart, (6, 3));
        assert!(with_axis[0].starts_with("1.0k"), "{with_axis:?}");
        assert!(with_axis[2].starts_with("   0"), "{with_axis:?}");

        // The legend is drawn over the full width too.
        let chart = Chart::new()
            .legend(true)
            .plot(Plot::area([1000.0]).label("a"));
        assert!(
            rows(&chart, (5, 2))[0].starts_with(" ━ a"),
            "{:?}",
            rows(&chart, (5, 2))
        );
    }

    #[test]
    fn layout_is_cached() {
        crate::views::layout_key_tests::assert_layout_is_cached(|| {
            Chart::new().plot(Plot::line([1.0, 2.0]))
        });
    }

    #[test]
    fn layout_key_follows_the_minimum_size_only() {
        let series = Series::new();
        let mut chart = Chart::new().plot(Plot::area(&series).label("a"));
        chart.layout(Vec2::new(20, 5));
        let key = chart.layout_key();
        assert!(!chart.needs_relayout());

        // Data, labels, legend, axis and range do not change what is asked for.
        series.extend((0..1000).map(f64::from));
        chart.set_legend(true);
        chart.set_axis(false);
        chart.set_grid_lines(7);
        chart.set_range(ValueRange::fixed(0.0, 1.0));
        chart.add_plot(Plot::line([1.0, 2.0]).label("b"));
        chart.set_y_labels(|_| "a very long label".to_string());
        assert_eq!(chart.layout_key(), key);
        assert!(!chart.needs_relayout());

        // The minimum size does, and the key goes back with it.
        chart.set_min_size(Vec2::new(30, 5));
        assert_ne!(chart.layout_key(), key);
        assert!(chart.needs_relayout());
        assert_eq!(chart.required_size(Vec2::new(100, 100)), Vec2::new(30, 5));
        chart.layout(Vec2::new(30, 5));
        assert!(!chart.needs_relayout());
        chart.set_min_size(Vec2::new(20, 5));
        assert_eq!(chart.layout_key(), key);
        assert!(chart.needs_relayout());
    }

    #[test]
    fn required_size_does_not_depend_on_data() {
        let mut chart = Chart::new();
        let empty = chart.required_size(Vec2::new(100, 100));
        chart.add_plot(Plot::area((0..1000).map(f64::from).collect::<Series>()));
        assert_eq!(chart.required_size(Vec2::new(100, 100)), empty);
        assert_eq!(empty, Vec2::new(20, 5));
    }

    #[test]
    fn min_size_is_the_required_size() {
        let mut chart = Chart::new().min_size((40, 10));
        assert_eq!(chart.required_size(Vec2::new(100, 100)), Vec2::new(40, 10));
        // Whatever the constraint, even a smaller one.
        assert_eq!(chart.required_size(Vec2::new(5, 2)), Vec2::new(40, 10));
        chart.add_plot(Plot::area([1.0, 2.0]));
        assert_eq!(chart.required_size(Vec2::new(100, 100)), Vec2::new(40, 10));

        chart.set_min_size(Vec2::new(8, 3));
        assert_eq!(chart.required_size(Vec2::new(100, 100)), Vec2::new(8, 3));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn chart_from_config() {
        use crate::style::BaseColor;

        let config = r#"
            Chart:
                range: [0, 16]
                axis: false
                legend: true
                plots:
                    - area: [8, 4]
                      fill: red
                      label: cpu
                    - line: $data
                      label: mem
        "#;
        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let data = Series::new();
        let mut context = crate::builder::Context::new();
        context.store("data", data.clone());
        let view = context.build(&config).unwrap();
        let chart = view
            .downcast_ref::<Chart>()
            .expect("the view should be a chart");

        let plots = chart.plots();
        assert_eq!(plots.len(), 2);
        assert_eq!(plots[0].kind(), PlotKind::Area);
        assert_eq!(plots[0].series().to_vec(), [8.0, 4.0]);
        assert_eq!(plots[0].get_label(), Some("cpu"));
        assert_eq!(plots[1].kind(), PlotKind::Line);
        assert_eq!(plots[1].get_label(), Some("mem"));

        // The second plot is the stored series.
        data.extend([16.0, 16.0]);
        assert_eq!(plots[1].series().to_vec(), [16.0, 16.0]);

        // The range is fixed, there is no axis, and there is a legend. The area is 2 rows high:
        // 8/16 fills the bottom row, and 4/16 half of it. The line is at the top right.
        let size = (14, 2);
        assert_eq!(rows(chart, size), [" ━ cpu  ━ mem⠉", "            █▄"]);
        assert_eq!(front(chart, size, (12, 1)), Color::Dark(BaseColor::Red));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn chart_defaults_from_config() {
        let config: crate::builder::Config = yaml_serde::from_str("Chart: {}").unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let chart = view.downcast_ref::<Chart>().unwrap();
        assert!(chart.plots().is_empty());
        assert!(chart.axis && !chart.legend && chart.grid_lines == 3);
        assert_eq!(chart.min_size, Vec2::new(20, 5));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn min_size_from_config() {
        let config: crate::builder::Config =
            yaml_serde::from_str("Chart: {min_size: [40, 10]}").unwrap();
        let view = crate::builder::Context::new().build(&config).unwrap();
        let chart = view.downcast_ref::<Chart>().unwrap();
        assert_eq!(chart.min_size, Vec2::new(40, 10));
    }

    #[cfg(feature = "builder")]
    #[test]
    fn invalid_plots_from_config() {
        let build = |config: &str| {
            let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
            crate::builder::Context::new().build(&config)
        };
        // Both kinds, neither, and not a plot at all.
        assert!(build("Chart: {plots: [{line: [1], area: [1]}]}").is_err());
        assert!(build("Chart: {plots: [{label: a}]}").is_err());
        assert!(build("Chart: {plots: [3]}").is_err());
        // Bad series, unknown variable, bad fill.
        assert!(build("Chart: {plots: [{line: [a]}]}").is_err());
        assert!(build("Chart: {plots: [{line: $nope}]}").is_err());
        assert!(build("Chart: {plots: [{area: [1], fill: [1]}]}").is_err());
        assert!(build("Chart: {plots: [{area: [1]}]}").is_ok());
    }

    fn odd_datasets() -> Vec<Vec<f64>> {
        vec![
            vec![],
            vec![1.0],
            vec![5.0; 30],
            vec![f64::NAN; 4],
            vec![1.0, f64::NAN, 3.0, f64::INFINITY, f64::NEG_INFINITY, 2.0],
            vec![1e300, -1e300, 1e300],
            vec![f64::MAX, f64::MIN],
            (0..40).map(|i| (i as f64 * 0.7).sin() * 100.0).collect(),
            (0..40).map(|i| i as f64).collect(),
            vec![-3.0, -2.0, -1.0],
        ]
    }

    fn odd_ranges() -> Vec<ValueRange> {
        vec![
            ValueRange::default(),
            ValueRange::fixed(5.0, 5.0),
            ValueRange::auto().include(0.0).step(100.0).margin(0.1),
            ValueRange::auto().step(0.001),
        ]
    }

    /// Draws `draw` into a buffer bigger than the printer and checks the margin is untouched.
    fn assert_draws_inside(size: (usize, usize), draw: impl Fn(&Printer), ctx: &str) {
        let (bw, bh) = (size.0 + 3, size.1 + 3);
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(bw, bh));
        let theme = Theme::retro();
        let buffer = RwLock::new(buffer);
        let full = Printer::new((bw, bh), &theme, &buffer);
        for y in 0..bh {
            full.print_hline((0, y), bw, ".");
        }
        draw(&Printer::new(size, &theme, &buffer));
        let b = buffer.read();
        for y in 0..bh {
            for x in 0..bw {
                if x >= size.0 || y >= size.1 {
                    assert_eq!(
                        b.cell_text(Vec2::new(x, y)),
                        Some("."),
                        "{ctx}: cell ({x},{y}) outside of {size:?} changed"
                    );
                }
            }
        }
    }

    #[test]
    fn chart_draws_inside_its_area_for_every_size_and_setting() {
        let datasets = odd_datasets();
        let ranges = odd_ranges();
        // Widths around the labels' (`1.0k` takes 5 columns), and heights from none to many.
        for w in [0, 1, 2, 3, 4, 5, 6, 8, 12usize] {
            for h in [0, 1, 3, 6usize] {
                for (di, data) in datasets.iter().enumerate() {
                    for (ri, range) in ranges.iter().enumerate() {
                        for (axis, legend) in [(false, false), (true, false), (true, true)] {
                            for lines in [0usize, 3] {
                                let chart = Chart::new()
                                    .axis(axis)
                                    .legend(legend)
                                    .grid_lines(lines)
                                    .range(*range)
                                    .plot(Plot::area(data.clone()).label("area"))
                                    .plot(Plot::line(data.clone()).label("line"))
                                    .plot(Plot::line(vec![1.0, 2.0]).label("日本語"))
                                    .plot(Plot::area(vec![]));
                                let ctx = format!(
                                    "{w}x{h} data {di} range {ri} axis {axis} legend {legend} lines {lines}"
                                );
                                assert_draws_inside((w, h), |p| chart.draw(p), &ctx);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn chart_with_odd_labels_draws_inside_its_area() {
        for label in [
            "",
            "x",
            "a very long label indeed",
            "日本語日本語",
            "e\u{301}e\u{301}",
        ] {
            let label = label.to_string();
            for w in 0..=14usize {
                for h in 0..=6usize {
                    let l = label.clone();
                    let chart = Chart::new()
                        .y_labels(move |_| l.clone())
                        .plot(Plot::area(vec![1.0, 2.0, 3.0]))
                        .plot(Plot::line(vec![1.0, 2.0, 3.0]));
                    assert_draws_inside((w, h), |p| chart.draw(p), &format!("{label:?} {w}x{h}"));
                }
            }
        }
        // A formatter whose width depends on the bounds (and so on the label column).
        for w in 0..=14usize {
            let chart = Chart::new()
                .y_labels(|v| "9".repeat(v.abs() as usize % 7 + 1))
                .plot(Plot::area(
                    (0..40).map(|i| (i * i) as f64).collect::<Vec<_>>(),
                ));
            assert_draws_inside((w, 5), |p| chart.draw(p), &format!("dependent labels {w}"));
        }
    }

    #[test]
    fn sparkline_draws_inside_its_area_for_every_size_and_data() {
        let datasets = odd_datasets();
        let ranges = odd_ranges();
        for w in 0..=12usize {
            for h in 0..=6usize {
                for data in &datasets {
                    for range in &ranges {
                        let s = Sparkline::new(data.clone()).range(*range);
                        assert_draws_inside((w, h), |p| s.draw(p), "sparkline");
                    }
                }
            }
        }
    }

    #[test]
    fn heatmap_draws_inside_its_area_for_every_size_and_setting() {
        let datasets = odd_datasets();
        let ranges = odd_ranges();
        for w in [0, 1, 3, 6, 14usize] {
            for h in [0, 1, 2, 4usize] {
                for nrows in [0, 1, 3, 5usize] {
                    for (di, data) in datasets.iter().enumerate() {
                        for range in &ranges {
                            for (labels, legend) in [(false, false), (true, false), (true, true)] {
                                let mut hm = Heatmap::new().range(*range);
                                for r in 0..nrows {
                                    let label = if labels {
                                        ["cpu", "日本", "", "e\u{301}", "long label"][r % 5]
                                    } else {
                                        ""
                                    };
                                    hm.add_row(label, data.clone());
                                }
                                if legend {
                                    hm = hm.legend("low", "日本高");
                                }
                                assert_draws_inside(
                                    (w, h),
                                    |p| hm.draw(p),
                                    &format!(
                                        "heatmap {w}x{h} rows {nrows} data {di} labels {labels} legend {legend}"
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
