use crate::Printer;
use crate::Vec2;
use crate::style::chart::{AXIS, Fill, FillPos, series_color};
use crate::style::{ColorStyle, ColorType, PaletteColor};
use crate::utils::DotGrid;
use crate::view::View;
use crate::views::{Series, ValueRange};
use unicode_width::UnicodeWidthStr;

/// Number of cells in the color run of the legend.
const LEGEND_RUN: usize = 24;
/// The legend is dropped if its color run would be shorter than this.
const LEGEND_MIN_RUN: usize = 4;

/// Several labelled [`Series`], with each value shown as a color.
///
/// Time goes from left to right: each sample takes one column, and the newest
/// one is in the rightmost column. Each row of the heatmap is a series, and two
/// series share each line of the terminal, using half-block characters.
///
/// A label column on the left names the series: line `i` shows the labels of
/// rows `2i` and `2i+1`, as `top/bottom`. The column is left out when all the
/// labels are empty.
///
/// The colors come from a scale, which maps values to colors. The rows that
/// are shown share the same bounds, so they can be compared with each other:
/// if the view is too short for all the rows, the ones left out do not take
/// part in the bounds. An optional
/// legend takes an extra line below the plot, and shows the scale from low to
/// high values.
///
/// This view is not focusable and ignores events.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::Color;
/// use cursive_core::views::{Heatmap, Series};
///
/// let cpu0 = Series::with_capacity(60);
/// let cpu1 = Series::with_capacity(60);
///
/// let heatmap = Heatmap::new()
///     .row("cpu0", &cpu0)
///     .row("cpu1", &cpu1)
///     .range(0.0..=1.0)
///     .scale([Color::Rgb(0, 0, 60), Color::Rgb(255, 160, 0)])
///     .legend("idle", "busy");
///
/// // The heatmap shows the new data as soon as it is pushed.
/// cpu0.push(0.2);
/// cpu1.push(0.9);
/// assert_eq!(heatmap.len(), 2);
/// ```
pub struct Heatmap {
    rows: Vec<(String, Series)>,
    range: ValueRange,
    scale: Fill,
    legend: Option<(String, String)>,
    // The size we asked for the last time we were laid out.
    laid_out: Option<(usize, usize)>,
}

impl Default for Heatmap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heatmap {
    /// Creates a new, empty heatmap.
    ///
    /// By default, the scale goes from the background of views to the first
    /// series color, and there is no legend.
    pub fn new() -> Self {
        Heatmap {
            rows: Vec::new(),
            range: ValueRange::default(),
            scale: Fill::by_value([
                ColorType::from(PaletteColor::View),
                ColorType::from(series_color(0)),
            ]),
            legend: None,
            laid_out: None,
        }
    }

    /// Adds a row at the bottom of the heatmap.
    ///
    /// The label can be empty. Pass a `&Series` to keep sharing the data with the caller.
    pub fn add_row(&mut self, label: impl Into<String>, series: impl Into<Series>) {
        self.rows.push((label.into(), series.into()));
    }

    /// Adds a row at the bottom of the heatmap.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn row(self, label: impl Into<String>, series: impl Into<Series>) -> Self {
        crate::With::with(self, |s| s.add_row(label, series))
    }

    /// Removes all the rows.
    pub fn clear_rows(&mut self) {
        self.rows.clear();
    }

    /// Returns the number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns `true` if there is no row.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
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

    /// Sets the colors of the scale, from the lowest value to the highest.
    ///
    /// Colors between the stops are interpolated.
    ///
    /// # Panics
    ///
    /// Panics if `stops` is empty.
    pub fn set_scale<C: Into<ColorType>>(&mut self, stops: impl IntoIterator<Item = C>) {
        self.scale = Fill::by_value(stops);
    }

    /// Sets the colors of the scale, from the lowest value to the highest.
    ///
    /// Chainable variant.
    ///
    /// # Panics
    ///
    /// Panics if `stops` is empty.
    #[must_use]
    pub fn scale<C: Into<ColorType>>(self, stops: impl IntoIterator<Item = C>) -> Self {
        crate::With::with(self, |s| s.set_scale(stops))
    }

    /// Sets the legend: the texts at both ends of the scale, or `None` to remove it.
    pub fn set_legend(&mut self, legend: Option<(String, String)>) {
        self.legend = legend;
    }

    /// Shows a legend under the plot, with the texts at both ends of the scale.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn legend(self, low: impl Into<String>, high: impl Into<String>) -> Self {
        crate::With::with(self, |s| s.set_legend(Some((low.into(), high.into()))))
    }

    /// Returns the number of terminal lines used by the rows.
    fn lines(&self) -> usize {
        self.rows.len().div_ceil(2)
    }

    /// Returns the label of a line.
    fn line_label(&self, line: usize) -> String {
        let top = self.rows[2 * line].0.as_str();
        let bottom = self.rows.get(2 * line + 1).map_or("", |row| row.0.as_str());
        match (top.is_empty(), bottom.is_empty()) {
            (false, false) => format!("{top}/{bottom}"),
            (false, true) => top.to_string(),
            (true, false) => bottom.to_string(),
            (true, true) => String::new(),
        }
    }

    /// Returns the width of the label column, including the space after the labels.
    fn label_width(&self) -> usize {
        let widest = (0..self.lines())
            .map(|line| self.line_label(line).width())
            .max()
            .unwrap_or(0);
        if widest == 0 { 0 } else { widest + 1 }
    }

    /// Returns the size we ask for when there is room for everything.
    ///
    /// This is all that decides the size: the key and `needs_relayout` follow it.
    fn wanted_size(&self) -> (usize, usize) {
        let longest = self
            .rows
            .iter()
            .map(|(_, series)| series.len())
            .max()
            .unwrap_or(0);
        let width = self.label_width() + longest.max(1);
        let height = self.lines() + usize::from(self.legend.is_some());
        (width, height)
    }
}

impl View for Heatmap {
    fn layout_key(&self) -> u64 {
        // The size depends on the labels (through the width of the column), on the longest
        // series, and on the number of lines: all in `wanted_size`.
        crate::view::combine_layout_key(crate::view::layout_key_seed::<Self>(), &self.wanted_size())
    }

    fn needs_relayout(&self) -> bool {
        self.laid_out != Some(self.wanted_size())
    }

    fn layout(&mut self, _: Vec2) {
        self.laid_out = Some(self.wanted_size());
    }

    fn draw(&self, printer: &Printer) {
        let size = printer.size;
        if size.y == 0 {
            return;
        }

        let legend = usize::from(self.legend.is_some());
        let lines = self.lines().min(size.y - legend);
        let label_width = self.label_width();
        let plot_width = size.x.saturating_sub(label_width);

        for line in 0..lines {
            printer.with_color(ColorStyle::front(&AXIS), |printer| {
                printer.print((0, line), &self.line_label(line));
            });
        }

        let needs_scale = (lines > 0 || legend > 0) && plot_width > 0;
        if !needs_scale {
            return;
        }
        let scale = self.scale.resolve(printer);
        let plot = printer.offset((label_width, 0));

        if lines > 0 {
            // Lock each series once.
            // Only the rows that are drawn count for the bounds, like the visible samples
            // do in a chart.
            let samples: Vec<Vec<f64>> = self
                .rows
                .iter()
                .take(2 * lines)
                .map(|(_, series)| series.tail(plot_width))
                .collect();
            let bounds = self.range.resolve(samples.iter().flatten().copied());

            // Two rows per line: row `y` is the dot at height `y`.
            let mut grid = DotGrid::half_blocks((plot_width, lines));
            for (y, samples) in samples.iter().enumerate() {
                let start = plot_width - samples.len();
                for (x, &sample) in samples.iter().enumerate() {
                    let pos = FillPos {
                        value: bounds.ratio(sample),
                        ..FillPos::default()
                    };
                    grid.set((start + x, y), scale.color_at(pos));
                }
            }
            grid.draw(&plot);
        }

        if let Some((low, high)) = &self.legend {
            let (low_width, high_width) = (low.width(), high.width());
            let run = plot_width
                .saturating_sub(low_width + high_width + 2)
                .min(LEGEND_RUN);
            if run >= LEGEND_MIN_RUN {
                let y = size.y - 1;
                let axis = ColorStyle::front(&AXIS);
                plot.with_color(axis, |printer| printer.print((0, y), &format!("{low} ")));
                for i in 0..run {
                    let pos = FillPos {
                        value: i as f32 / (run - 1) as f32,
                        ..FillPos::default()
                    };
                    plot.with_color(ColorStyle::front(scale.color_at(pos)), |printer| {
                        printer.print((low_width + 1 + i, y), "▆");
                    });
                }
                plot.with_color(axis, |printer| {
                    printer.print((low_width + 1 + run, y), &format!(" {high}"));
                });
            }
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        let (width, height) = self.wanted_size();
        Vec2::new(width.min(constraint.x), height)
    }
}

crate::manual_blueprint!(Heatmap, |config, context| {
    use crate::builder::Error;

    let mut heatmap = Heatmap::new();

    let rows = &config["rows"];
    if !rows.is_null() {
        let rows = rows
            .as_array()
            .ok_or_else(|| Error::invalid_config("Expected a list of rows", rows))?;
        for row in rows {
            let label: Option<String> = context.resolve(&row["label"])?;
            let series: Series = context.resolve(&row["series"])?;
            heatmap.add_row(label.unwrap_or_default(), series);
        }
    }

    if let Some(range) = context.resolve::<Option<ValueRange>>(&config["range"])? {
        heatmap.set_range(range);
    }

    if let Some(stops) = context.resolve::<Option<Vec<ColorType>>>(&config["scale"])? {
        if stops.is_empty() {
            return Err(Error::invalid_config(
                "A scale needs at least one color",
                &config["scale"],
            ));
        }
        heatmap.set_scale(stops);
    }

    let legend = &config["legend"];
    if !legend.is_null() {
        let low: String = context.resolve(&legend["low"])?;
        let high: String = context.resolve(&legend["high"])?;
        heatmap.set_legend(Some((low, high)));
    }

    Ok(heatmap)
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::{Color, Rgb};
    use crate::theme::Theme;
    use parking_lot::RwLock;

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);
    const RED: Color = Color::Rgb(255, 0, 0);

    /// One drawn cell: its text, front color and back color.
    type Cell = (String, Color, Color);

    /// Draws the view, and returns the cells and the default background color.
    fn draw_in(heatmap: &Heatmap, size: Vec2) -> (Vec<Vec<Cell>>, Color) {
        let theme = Theme::retro();
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        let printer = Printer::new(size, &theme, &buffer);
        let background = printer.current_color().back;
        heatmap.draw(&printer);

        let buffer = buffer.read();
        let cells = (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| {
                        let pos = Vec2::new(x, y);
                        let unset = Color::TerminalDefault;
                        let (front, back) =
                            buffer.cell_style(pos).map_or((unset, unset), |style| {
                                (style.color.front, style.color.back)
                            });
                        (buffer.cell_text(pos).unwrap_or("").to_string(), front, back)
                    })
                    .collect()
            })
            .collect();
        (cells, background)
    }

    fn cells(heatmap: &Heatmap, size: impl Into<Vec2>) -> Vec<Vec<Cell>> {
        draw_in(heatmap, size.into()).0
    }

    /// The lines of text, with `.` for the cells that were not printed.
    fn text(heatmap: &Heatmap, size: impl Into<Vec2>) -> Vec<String> {
        cells(heatmap, size)
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(text, _, _)| if text.is_empty() { ".".into() } else { text })
                    .collect()
            })
            .collect()
    }

    fn axis_color() -> Color {
        Theme::retro().palette[PaletteColor::Tertiary]
    }

    /// A heatmap with a black to white scale between 0 and 1.
    fn plain() -> Heatmap {
        Heatmap::new().range(0.0..=1.0).scale([BLACK, WHITE])
    }

    #[test]
    fn rows_share_lines() {
        let heatmap = plain().row("", [1.0, 0.0, 1.0]).row("", [1.0, 1.0, 0.0]);
        assert_eq!(text(&heatmap, (3, 1)), ["▀▀▀"]);

        // The top row is the front color, the bottom row the back color.
        let line = &cells(&heatmap, (3, 1))[0];
        assert_eq!((line[0].1, line[0].2), (WHITE, WHITE));
        assert_eq!((line[1].1, line[1].2), (BLACK, WHITE));
        assert_eq!((line[2].1, line[2].2), (WHITE, BLACK));
    }

    #[test]
    fn newest_samples_are_on_the_right() {
        let heatmap = plain().row("", [1.0]).row("", [1.0, 1.0]);
        assert_eq!(text(&heatmap, (4, 1)), ["..▄▀"]);
        // Only the bottom row has a sample in the third column.
        let line = &cells(&heatmap, (4, 1))[0];
        assert_eq!(line[2].1, WHITE);
        assert_eq!((line[3].1, line[3].2), (WHITE, WHITE));

        // Only the newest samples are visible.
        let heatmap = plain().row("", [1.0, 0.0, 1.0]);
        let line = &cells(&heatmap, (2, 1))[0];
        assert_eq!((line[0].1, line[1].1), (BLACK, WHITE));
    }

    #[test]
    fn odd_row_only_sets_the_top_half() {
        let heatmap = plain()
            .row("", [1.0, 0.0])
            .row("", [0.0, 1.0])
            .row("", [1.0, 0.0]);
        assert_eq!(text(&heatmap, (2, 2)), ["▀▀", "▀▀"]);

        let (cells, background) = draw_in(&heatmap, Vec2::new(2, 2));
        // The second line only has a top half: the background shows below.
        assert_eq!((cells[1][0].1, cells[1][0].2), (WHITE, background));
        assert_eq!((cells[1][1].1, cells[1][1].2), (BLACK, background));
    }

    #[test]
    fn bounds_are_shared_by_all_rows() {
        // The default range starts at 0 and ends at the largest value.
        let heatmap = Heatmap::new()
            .scale([BLACK, WHITE])
            .row("", [1.0])
            .row("", [4.0]);
        let cell = &cells(&heatmap, (1, 1))[0][0];
        let Color::Rgb(top, _, _) = cell.1 else {
            panic!("gradients give RGB colors");
        };
        // 1 out of 4.
        assert!((63..=65).contains(&top));
        assert_eq!(cell.2, WHITE);
    }

    #[test]
    fn bounds_only_use_the_rows_that_are_shown() {
        let front = |heatmap: &Heatmap| {
            let cell = &cells(heatmap, (1, 1))[0][0];
            assert_eq!(cell.0, "▀");
            cell.1
        };
        let two = Heatmap::new()
            .scale([BLACK, WHITE])
            .row("", [1.0])
            .row("", [1.0]);
        // A single line only shows two rows: the others do not take part in the bounds.
        let four = Heatmap::new()
            .scale([BLACK, WHITE])
            .row("", [1.0])
            .row("", [1.0])
            .row("", [100.0])
            .row("", [100.0]);
        assert_eq!(front(&two), front(&four));
        assert_eq!(front(&four), WHITE);
        // With room for the other rows, they do count.
        let cells = cells(&four, (1, 2));
        assert_ne!(cells[0][0].1, WHITE);
        assert_eq!(cells[1][0].1, WHITE);
    }

    #[test]
    fn scale_is_followed() {
        let heatmap = Heatmap::new()
            .range(0.0..=2.0)
            .scale([BLACK, RED, WHITE])
            .row("", [0.0, 1.0, 2.0]);
        let line = &cells(&heatmap, (3, 1))[0];
        assert_eq!(line[0].1, BLACK);
        assert_eq!(line[1].1, RED);
        assert_eq!(line[2].1, WHITE);
    }

    #[test]
    fn default_scale_goes_from_the_view_color() {
        let heatmap = Heatmap::new().range(0.0..=1.0).row("", [0.0, 1.0]);
        let theme = Theme::retro();
        let line = &cells(&heatmap, (2, 1))[0];
        let rgb = |color: PaletteColor| theme.palette[color].to_rgb().unwrap();
        for (cell, expected) in [
            (&line[0], rgb(PaletteColor::View)),
            (&line[1], rgb(PaletteColor::TitlePrimary)),
        ] {
            let Color::Rgb(r, g, b) = cell.1 else {
                panic!("gradients give RGB colors");
            };
            assert_eq!(Rgb::new(r, g, b), expected);
        }
    }

    #[test]
    fn labels() {
        let heatmap = plain()
            .row("cpu0", [1.0])
            .row("cpu1", [1.0])
            .row("mem", [1.0])
            .row("", [1.0])
            .row("", [1.0])
            .row("net", [1.0]);
        // The widest label is "cpu0/cpu1": 9 cells, and a space.
        assert_eq!(heatmap.label_width(), 10);
        // A line with a single label only shows that one.
        assert_eq!(
            text(&heatmap, (11, 3)),
            ["cpu0/cpu1.▀", "mem.......▀", "net.......▀"]
        );

        let cells = cells(&heatmap, (11, 3));
        assert_eq!((cells[0][0].0.as_str(), cells[0][0].1), ("c", axis_color()));
        assert_eq!(cells[2][0].0, "n");
    }

    #[test]
    fn single_row_on_a_line() {
        let heatmap = plain().row("a", [1.0]).row("b", [1.0]).row("c", [1.0]);
        assert_eq!(heatmap.label_width(), 4);
        assert_eq!(text(&heatmap, (5, 2)), ["a/b.▀", "c...▀"]);
    }

    #[test]
    fn label_column_width() {
        let mut heatmap = plain().row("ab", [1.0, 1.0, 1.0]);
        assert_eq!(heatmap.label_width(), 3);
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(6, 1));

        // Labels are measured in cells.
        let heatmap = plain().row("日本", [1.0]);
        assert_eq!(heatmap.label_width(), 5);
    }

    #[test]
    fn no_label_column_without_labels() {
        let mut heatmap = plain().row("", [1.0, 1.0]).row("", [1.0]);
        assert_eq!(heatmap.label_width(), 0);
        assert_eq!(text(&heatmap, (2, 1)), ["▀▀"]);
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(2, 1));
    }

    #[test]
    fn legend_line() {
        let heatmap = plain().row("", [1.0]).legend("lo", "hi");
        let size = Vec2::new(40, 2);

        // Room for both texts, 2 spaces and 24 cells: the whole legend is shown.
        let lines = text(&heatmap, size);
        assert_eq!(lines[0], format!("{}▀", ".".repeat(39)));
        assert_eq!(
            lines[1],
            format!("lo {} hi{}", "▆".repeat(24), ".".repeat(10))
        );

        let line = &cells(&heatmap, size)[1];
        for x in [0, 1, 28, 29] {
            assert_eq!(line[x].1, axis_color(), "x={x}");
        }
        // The run goes from the low end of the scale to the high end.
        assert_eq!(line[3].1, BLACK);
        assert_eq!(line[26].1, WHITE);

        // With less room, the run is shorter.
        assert_eq!(text(&heatmap, (12, 2))[1], "lo ▆▆▆▆▆▆ hi");
    }

    #[test]
    fn legend_is_dropped_when_too_narrow() {
        let heatmap = plain().row("", [1.0]).legend("lo", "hi");
        // The texts, 2 spaces and 4 cells are 10 cells.
        assert_eq!(text(&heatmap, (10, 2))[1], "lo ▆▆▆▆ hi");
        assert_eq!(text(&heatmap, (9, 2))[1], ".........");
    }

    #[test]
    fn legend_is_below_the_plot() {
        let heatmap = plain().row("ab", [1.0]).legend("0", "1");
        assert_eq!(
            text(&heatmap, (11, 2)),
            [format!("ab{}▀", ".".repeat(8)), "...0 ▆▆▆▆ 1".to_string()]
        );
    }

    #[test]
    fn too_small_area() {
        let heatmap = plain()
            .row("a", [1.0])
            .row("b", [1.0])
            .row("c", [1.0])
            .row("d", [1.0])
            .legend("lo", "hi");

        // The rows and the legend need 3 lines: the last line of rows is dropped.
        assert_eq!(
            text(&heatmap, (14, 2)),
            [
                format!("a/b{}▀", ".".repeat(10)),
                "....lo ▆▆▆▆ hi".to_string()
            ]
        );
        // With a single line, the legend wins.
        assert_eq!(text(&heatmap, (14, 1)), ["....lo ▆▆▆▆ hi"]);

        // No room at all.
        assert!(text(&heatmap, (14, 0)).is_empty());
        // No room for the plot: only the labels.
        assert_eq!(text(&heatmap, (3, 3)), ["a/b", "c/d", "..."]);
        assert_eq!(text(&heatmap, (0, 3)), ["", "", ""]);
    }

    #[test]
    fn empty_heatmap() {
        let mut heatmap = plain();
        assert!(heatmap.is_empty());
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(1, 0));
        assert_eq!(text(&heatmap, (3, 2)), ["...", "..."]);

        heatmap.add_row("x", Series::new());
        assert_eq!(heatmap.len(), 1);
        assert_eq!(text(&heatmap, (4, 1)), ["x..."]);

        heatmap.clear_rows();
        assert!(heatmap.is_empty());
    }

    #[test]
    fn required_size() {
        let mut heatmap = plain()
            .row("", [1.0, 2.0, 3.0])
            .row("", [1.0])
            .row("", [1.0]);
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(3, 2));
        assert_eq!(heatmap.required_size(Vec2::new(2, 20)), Vec2::new(2, 2));

        heatmap.set_legend(Some(("a".into(), "b".into())));
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(3, 3));
        heatmap.set_legend(None);
        assert_eq!(heatmap.required_size(Vec2::new(20, 20)), Vec2::new(3, 2));
    }

    #[test]
    fn layout_is_cached() {
        crate::views::layout_key_tests::assert_layout_is_cached(|| {
            plain()
                .row("cpu", [1.0, 2.0])
                .row("", [1.0])
                .legend("a", "b")
        });
        crate::views::layout_key_tests::assert_layout_is_cached(Heatmap::new);
    }

    #[test]
    fn layout_key_follows_the_size() {
        let series = Series::new();
        let mut heatmap = plain().row("a", &series);
        heatmap.layout(Vec2::new(5, 1));
        let key = heatmap.layout_key();
        assert!(!heatmap.needs_relayout());

        // Data that does not change the longest series.
        series.push(0.5);
        assert_eq!(heatmap.layout_key(), key);
        assert!(!heatmap.needs_relayout());
        heatmap.set_range(0.0..=9.0);
        heatmap.set_scale([RED, WHITE]);
        assert_eq!(heatmap.layout_key(), key);

        // Each of these changes the size.
        let changes: [(&str, fn(&mut Heatmap)); 6] = [
            ("longer series", |h| h.rows[0].1.push(0.5)),
            ("wider label", |h| h.rows[0].0 = "abc".into()),
            ("no label", |h| h.rows[0].0.clear()),
            ("legend", |h| h.set_legend(Some(("a".into(), "b".into())))),
            ("two rows more", |h| {
                h.add_row("", [0.0]);
                h.add_row("", [0.0]);
            }),
            ("no rows", Heatmap::clear_rows),
        ];
        for (name, change) in changes {
            let mut heatmap = plain().row("a", [0.5]);
            heatmap.layout(Vec2::new(5, 1));
            let key = heatmap.layout_key();
            let size = heatmap.required_size(Vec2::new(50, 50));
            change(&mut heatmap);
            let changed = heatmap.required_size(Vec2::new(50, 50));
            assert_ne!(size, changed, "{name}");
            assert_ne!(heatmap.layout_key(), key, "{name}");
            assert!(heatmap.needs_relayout(), "{name}");
            heatmap.layout(changed);
            assert!(!heatmap.needs_relayout(), "{name}");
        }

        // Same size, same key: a label of the same width, or a row added to a line with room.
        // (The key follows the size, not the labels themselves.)
        let mut same = plain().row("a", [0.5]);
        let key = same.layout_key();
        same.rows[0].0 = "b".into();
        assert_eq!(same.layout_key(), key);
        same.add_row("", [0.0]);
        assert_eq!(same.layout_key(), key);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config() {
        let config = r#"
            Heatmap:
                rows:
                    - label: a
                      series: [0, 1]
                    - label: b
                      series: $shared
                    - series: [1]
                range: [0, 1]
                scale: [black, white]
                legend:
                    low: lo
                    high: hi
        "#;
        let shared = Series::new();
        shared.push(1.0);
        let mut context = crate::builder::Context::new();
        context.store("shared", shared.clone());

        let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
        let view = context.build(&config).unwrap();
        let heatmap = view
            .downcast_ref::<Heatmap>()
            .expect("the view should be a Heatmap");

        assert_eq!(heatmap.len(), 3);
        assert_eq!(heatmap.line_label(0), "a/b");
        assert_eq!(heatmap.line_label(1), "");
        assert_eq!(heatmap.legend, Some(("lo".to_string(), "hi".to_string())));
        assert_eq!(heatmap.range, ValueRange::fixed(0.0, 1.0));
        assert_eq!(heatmap.scale.stops().len(), 2);

        // The variable is shared, not copied.
        shared.push(2.0);
        assert_eq!(heatmap.rows[1].1.to_vec(), [1.0, 2.0]);
    }

    #[cfg(feature = "builder")]
    #[test]
    fn from_config_errors() {
        let build = |config: &str| {
            let config: crate::builder::Config = yaml_serde::from_str(config).unwrap();
            crate::builder::Context::new().build(&config).map(|_| ())
        };
        assert!(build("Heatmap: {}").is_ok());
        assert!(build("Heatmap: {rows: 3}").is_err());
        assert!(build("Heatmap: {rows: [{label: a, series: nope}]}").is_err());
        assert!(build("Heatmap: {scale: []}").is_err());
        assert!(build("Heatmap: {legend: {low: a}}").is_err());
    }
}
