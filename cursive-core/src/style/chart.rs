//! Colors for charts and other data views.
//!
//! * [`AXIS`] and [`SERIES`] are the default colors used by
//!   [`Chart`](crate::views::Chart), [`Sparkline`](crate::views::Sparkline) and
//!   [`Heatmap`](crate::views::Heatmap). They are regular custom palette entries, so a theme can
//!   set them.
//! * [`Fill`] describes how to color something drawn from data: with a single color, or with a
//!   gradient.

use super::gradient::Linear;
use super::{Color, ColorKey, ColorType, Palette, PaletteColor, Rgb};
use crate::Printer;

/// Color used for the axis, grid lines and labels of charts.
///
/// This is the `chart.axis` palette entry. If the theme does not define it, this is the same as
/// [`PaletteColor::Tertiary`].
///
/// ```toml
/// [colors.chart]
/// axis = "#555555"
/// ```
pub static AXIS: ColorKey = ColorKey::from_palette("chart.axis", PaletteColor::Tertiary);

/// Default colors for the successive data series of a chart.
///
/// These are the `chart.series.0`, `chart.series.1`, ... palette entries. If the theme does not
/// define them, they use title and highlight colors from the palette.
///
/// Charts with more series start again from the first color: see [`series_color`].
///
/// ```toml
/// [colors.chart.series]
/// 0 = "#4fd6be"
/// 1 = "#c099ff"
/// ```
pub static SERIES: [ColorKey; 6] = [
    ColorKey::from_palette("chart.series.0", PaletteColor::TitlePrimary),
    ColorKey::from_palette("chart.series.1", PaletteColor::TitleSecondary),
    ColorKey::from_palette("chart.series.2", PaletteColor::Highlight),
    ColorKey::from_palette("chart.series.3", PaletteColor::Secondary),
    ColorKey::from_palette("chart.series.4", PaletteColor::HighlightInactive),
    ColorKey::from_palette("chart.series.5", PaletteColor::Primary),
];

/// Returns the default color for the `n`-th series of a chart.
///
/// This cycles through [`SERIES`].
pub fn series_color(n: usize) -> &'static ColorKey {
    &SERIES[n % SERIES.len()]
}

/// What a gradient follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Along {
    /// From the bottom to the top of the area.
    Vertical,
    /// From the left to the right of the area.
    Horizontal,
    /// From the lowest to the highest value.
    Value,
    /// Constant colors between thresholds on the value: see `Fill::thresholds`.
    Steps,
}

/// How to color something drawn from data.
///
/// This is either a single color, or a gradient between several colors. Colors can be anything
/// that makes a [`ColorType`]: concrete colors, palette colors, or custom [`ColorKey`]s. They are
/// resolved when drawing, so a fill follows the current theme.
///
/// # Examples
///
/// ```rust
/// use cursive_core::style::chart::Fill;
/// use cursive_core::style::{BaseColor, PaletteColor, Rgb};
///
/// // A single color. Anything that can be a color can directly be used as a `Fill`.
/// let fill = Fill::solid(PaletteColor::Highlight);
/// let fill: Fill = BaseColor::Red.dark().into();
///
/// // Darker at the bottom, brighter at the top.
/// let fill = Fill::vertical([Rgb::new(0, 60, 60), Rgb::new(0, 255, 255)]);
///
/// // Green for low values, red for high ones.
/// let fill = Fill::by_value([BaseColor::Green.dark(), BaseColor::Red.dark()]);
///
/// // Green up to 70%, then yellow up to 85%, then red.
/// let fill = Fill::steps([
///     (0.0, BaseColor::Green.dark()),
///     (0.7, BaseColor::Yellow.dark()),
///     (0.85, BaseColor::Red.dark()),
/// ]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    // Never empty.
    stops: Vec<ColorType>,
    // Only for `Along::Steps`: the threshold of each stop, in increasing order.
    thresholds: Vec<f32>,
    along: Along,
}

impl Fill {
    /// A single color.
    pub fn solid(color: impl Into<ColorType>) -> Self {
        Fill {
            stops: vec![color.into()],
            thresholds: Vec::new(),
            along: Along::Value,
        }
    }

    /// A gradient going through the given colors, from the bottom to the top of the area.
    ///
    /// # Panics
    ///
    /// If `stops` is empty.
    pub fn vertical<C: Into<ColorType>>(stops: impl IntoIterator<Item = C>) -> Self {
        Self::gradient(stops, Along::Vertical)
    }

    /// A gradient going through the given colors, from the left to the right of the area.
    ///
    /// # Panics
    ///
    /// If `stops` is empty.
    pub fn horizontal<C: Into<ColorType>>(stops: impl IntoIterator<Item = C>) -> Self {
        Self::gradient(stops, Along::Horizontal)
    }

    /// A gradient going through the given colors, from the lowest to the highest value.
    ///
    /// What "lowest" and "highest" mean depends on the view: usually the range of its axis.
    ///
    /// # Panics
    ///
    /// If `stops` is empty.
    pub fn by_value<C: Into<ColorType>>(stops: impl IntoIterator<Item = C>) -> Self {
        Self::gradient(stops, Along::Value)
    }

    /// Colors by threshold: each color applies from its threshold up to the next one.
    ///
    /// Thresholds are positions between 0 and 1 along the value: the same normalized value as
    /// [`Fill::by_value`], which is a fraction of the range of the view. For a view showing a
    /// [`ValueRange`](crate::views::ValueRange) from 0 to 200, a threshold of `0.7` is at the
    /// value 140.
    ///
    /// Steps are sorted by threshold. Values below the first threshold use the first color. There
    /// is no blending: the color is exactly the one given, so colors that cannot be converted to
    /// RGB (like the terminal default) are preserved.
    ///
    /// Only the value matters: the position in the area is ignored.
    ///
    /// # Panics
    ///
    /// If `steps` is empty, or if a threshold is NaN.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::style::BaseColor;
    /// use cursive_core::style::chart::Fill;
    ///
    /// // Green below 70%, yellow below 85%, red above.
    /// let fill = Fill::steps([
    ///     (0.0, BaseColor::Green.dark()),
    ///     (0.7, BaseColor::Yellow.dark()),
    ///     (0.85, BaseColor::Red.dark()),
    /// ]);
    /// ```
    pub fn steps<C: Into<ColorType>>(steps: impl IntoIterator<Item = (f32, C)>) -> Self {
        let mut steps: Vec<(f32, ColorType)> = steps
            .into_iter()
            .map(|(threshold, color)| (threshold, color.into()))
            .collect();
        assert!(!steps.is_empty(), "a fill needs at least one color");
        assert!(
            steps.iter().all(|(threshold, _)| !threshold.is_nan()),
            "step thresholds cannot be NaN"
        );
        // Stable, so equal thresholds keep their order (and the last one wins).
        steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (thresholds, stops) = steps.into_iter().unzip();
        Fill {
            stops,
            thresholds,
            along: Along::Steps,
        }
    }

    fn gradient<C: Into<ColorType>>(stops: impl IntoIterator<Item = C>, along: Along) -> Self {
        let stops: Vec<_> = stops.into_iter().map(Into::into).collect();
        assert!(!stops.is_empty(), "a fill needs at least one color");
        Fill {
            stops,
            thresholds: Vec::new(),
            along,
        }
    }

    /// Returns the colors this fill goes through.
    pub fn stops(&self) -> &[ColorType] {
        &self.stops
    }

    /// Returns the color for the given value, from 0 to 1.
    ///
    /// This works when the color does not depend on the RGB values of the theme: solid fills,
    /// and fills made with [`Fill::steps`]. Gradients give `None`: they blend resolved colors,
    /// so they need [`Fill::resolve`] or [`Fill::resolve_with`].
    ///
    /// The color is not resolved: a palette color or a custom [`ColorKey`] is returned as it
    /// is. It can be used right away in a [`ColorStyle`](super::ColorStyle), for example to
    /// color some text, and it still follows the theme.
    ///
    /// For a stepped fill, this is the color of the last threshold that the value reached. A
    /// value below the first threshold, or NaN, gives the first color.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::style::chart::Fill;
    /// use cursive_core::style::{BaseColor, ColorStyle, ColorType, PaletteColor};
    /// use cursive_core::utils::markup::StyledString;
    ///
    /// let fill = Fill::steps([
    ///     (0.0, ColorType::from(BaseColor::Green.dark())),
    ///     (0.7, PaletteColor::Highlight.into()),
    ///     (0.85, BaseColor::Red.dark().into()),
    /// ]);
    ///
    /// let usage = 0.75;
    /// let color = fill.color_for(usage).unwrap();
    /// assert_eq!(color, ColorType::from(PaletteColor::Highlight));
    ///
    /// let text = StyledString::styled("75%", ColorStyle::front(color));
    ///
    /// // A gradient has no single color.
    /// assert_eq!(
    ///     Fill::by_value([BaseColor::Red.dark(), BaseColor::Blue.dark()]).color_for(0.5),
    ///     None
    /// );
    /// ```
    pub fn color_for(&self, value: f32) -> Option<ColorType> {
        match self.along {
            Along::Steps => Some(
                self.thresholds
                    .iter()
                    .rposition(|&threshold| value >= threshold)
                    .map_or(self.stops[0], |i| self.stops[i]),
            ),
            _ => match self.stops[..] {
                [color] => Some(color),
                _ => None,
            },
        }
    }

    /// Resolves the colors with the theme of the given printer.
    ///
    /// The result can then give the color of each point.
    ///
    /// Gradients need RGB values: see [`Color::to_rgb`] for how other colors are converted.
    /// The terminal default color, which has no known value, is replaced with a medium gray.
    pub fn resolve(&self, printer: &Printer) -> ResolvedFill {
        self.resolve_with(&printer.theme.palette, printer.current_color().front)
    }

    /// Resolves the colors with the given palette.
    ///
    /// This is [`Fill::resolve`] for when there is no [`Printer`] at hand. `parent` is the
    /// color used for [`ColorType::InheritParent`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::style::chart::{Fill, FillPos};
    /// use cursive_core::style::{BaseColor, Color, Palette};
    ///
    /// let fill = Fill::by_value([Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255)]);
    /// let resolved = fill.resolve_with(&Palette::terminal_default(), BaseColor::White.dark());
    ///
    /// let pos = FillPos {
    ///     value: 1.0,
    ///     ..FillPos::default()
    /// };
    /// assert_eq!(resolved.color_at(pos), Color::Rgb(255, 255, 255));
    /// ```
    pub fn resolve_with(&self, palette: &Palette, parent: Color) -> ResolvedFill {
        let colors: Vec<Color> = self
            .stops
            .iter()
            .map(|&stop| stop.resolve(palette, parent))
            .collect();

        let kind = match colors[..] {
            _ if self.along == Along::Steps => {
                ResolvedKind::Steps(self.thresholds.iter().copied().zip(colors).collect())
            }
            // Keep single colors as they are: no need to approximate them.
            [color] => ResolvedKind::Solid(color),
            _ => {
                let colors: Vec<Rgb<u8>> = colors
                    .iter()
                    .map(|color| color.to_rgb().unwrap_or(Rgb::new(128, 128, 128)))
                    .collect();
                ResolvedKind::Gradient(Linear::evenly_spaced(&colors), self.along)
            }
        };
        ResolvedFill { kind }
    }
}

impl<C: Into<ColorType>> From<C> for Fill {
    fn from(color: C) -> Self {
        Fill::solid(color)
    }
}

/// Where a point is, to pick its color in a [`ResolvedFill`].
///
/// Each field goes from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FillPos {
    /// Horizontal position in the area: 0 on the left, 1 on the right.
    pub x: f32,
    /// Vertical position in the area: 0 at the bottom, 1 at the top.
    pub y: f32,
    /// The value being drawn: 0 for the lowest, 1 for the highest.
    pub value: f32,
}

impl FillPos {
    /// Position of the given cell in an area of the given size.
    ///
    /// Here `row` is counted from the bottom of the area.
    pub fn cell(column: usize, row: usize, size: impl Into<crate::Vec2>, value: f32) -> Self {
        let size = size.into();
        // The first and last cells get the first and last colors.
        FillPos {
            x: fraction(column, size.x),
            y: fraction(row, size.y),
            value,
        }
    }
}

/// Position of `i` in `0..len`, as a fraction between 0 and 1.
///
/// The first and last elements are at 0 and 1; a single element is at 0.
pub(crate) fn fraction(i: usize, len: usize) -> f32 {
    if len > 1 {
        i as f32 / (len - 1) as f32
    } else {
        0.0
    }
}

#[derive(Clone, Debug)]
enum ResolvedKind {
    Solid(Color),
    Gradient(Linear, Along),
    // Sorted by threshold, never empty.
    Steps(Vec<(f32, Color)>),
}

/// A [`Fill`] with its colors resolved for a theme.
///
/// Created with [`Fill::resolve`].
#[derive(Clone, Debug)]
pub struct ResolvedFill {
    kind: ResolvedKind,
}

impl ResolvedFill {
    /// Returns the color for the given point.
    pub fn color_at(&self, pos: FillPos) -> Color {
        match &self.kind {
            ResolvedKind::Solid(color) => *color,
            ResolvedKind::Steps(steps) => {
                // The last step reached. A NaN value reaches none, like values below the first.
                steps
                    .iter()
                    .rev()
                    .find(|(threshold, _)| pos.value >= *threshold)
                    .unwrap_or(&steps[0])
                    .1
            }
            ResolvedKind::Gradient(gradient, along) => {
                let x = match along {
                    Along::Vertical => pos.y,
                    Along::Horizontal => pos.x,
                    Along::Value | Along::Steps => pos.value,
                };
                let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
                gradient.interpolate(x).as_color()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::theme::Theme;
    use parking_lot::RwLock;

    fn with_printer<R>(theme: &Theme, f: impl FnOnce(&Printer) -> R) -> R {
        let mut buffer = PrintBuffer::new();
        buffer.resize(crate::Vec2::new(4, 4));
        let buffer = RwLock::new(buffer);
        f(&Printer::new((4, 4), theme, &buffer))
    }

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);

    #[test]
    fn solid_keeps_the_color() {
        let theme = Theme::retro();
        // Even colors without a known RGB value.
        let fill = Fill::solid(Color::TerminalDefault);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        let pos = FillPos {
            x: 0.3,
            y: 0.6,
            value: 0.9,
        };
        assert_eq!(resolved.color_at(pos), Color::TerminalDefault);

        let fill: Fill = PaletteColor::Highlight.into();
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(
            resolved.color_at(pos),
            theme.palette[PaletteColor::Highlight]
        );
    }

    #[test]
    fn gradients_follow_their_axis() {
        let theme = Theme::retro();
        let resolve = |fill: Fill| with_printer(&theme, |printer| fill.resolve(printer));
        let pos = FillPos {
            x: 0.0,
            y: 1.0,
            value: 0.5,
        };

        assert_eq!(
            resolve(Fill::horizontal([BLACK, WHITE])).color_at(pos),
            BLACK
        );
        assert_eq!(resolve(Fill::vertical([BLACK, WHITE])).color_at(pos), WHITE);
        let Color::Rgb(r, g, b) = resolve(Fill::by_value([BLACK, WHITE])).color_at(pos) else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&r) && r == g && g == b);

        // Out-of-range and invalid positions are fine.
        let wild = FillPos {
            x: -3.0,
            y: 7.0,
            value: f32::NAN,
        };
        assert_eq!(
            resolve(Fill::horizontal([BLACK, WHITE])).color_at(wild),
            BLACK
        );
        assert_eq!(
            resolve(Fill::vertical([BLACK, WHITE])).color_at(wild),
            WHITE
        );
        assert_eq!(
            resolve(Fill::by_value([BLACK, WHITE])).color_at(wild),
            BLACK
        );
    }

    #[test]
    fn gradients_follow_the_theme() {
        static LOW: ColorKey = ColorKey::new("fill_test.low", BLACK);

        let fill = Fill::by_value([ColorType::from(&LOW), WHITE.into()]);
        let low = FillPos::default();

        let mut theme = Theme::retro();
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(low), BLACK);

        theme.palette.set_custom_color(&LOW, Color::Rgb(9, 8, 7));
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(low), Color::Rgb(9, 8, 7));
    }

    fn value(v: f32) -> FillPos {
        FillPos {
            value: v,
            ..FillPos::default()
        }
    }

    const A: Color = Color::Rgb(1, 0, 0);
    const B: Color = Color::Rgb(0, 2, 0);
    const C: Color = Color::Rgb(0, 0, 3);

    #[test]
    fn steps_thresholds() {
        let theme = Theme::retro();
        let fill = Fill::steps([(0.0, A), (0.7, B), (0.85, C)]);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));

        assert_eq!(resolved.color_at(value(0.0)), A);
        assert_eq!(resolved.color_at(value(0.5)), A);
        assert_eq!(resolved.color_at(value(0.6999)), A);
        assert_eq!(resolved.color_at(value(0.7)), B);
        assert_eq!(resolved.color_at(value(0.8)), B);
        assert_eq!(resolved.color_at(value(0.8499)), B);
        assert_eq!(resolved.color_at(value(0.85)), C);
        assert_eq!(resolved.color_at(value(1.0)), C);
        assert_eq!(resolved.color_at(value(50.0)), C);
        // Other axes are ignored.
        let pos = FillPos {
            x: 1.0,
            y: 1.0,
            value: 0.1,
        };
        assert_eq!(resolved.color_at(pos), A);
    }

    #[test]
    fn steps_are_sorted() {
        let theme = Theme::retro();
        let fill = Fill::steps([(0.85, C), (0.0, A), (0.7, B)]);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(value(0.1)), A);
        assert_eq!(resolved.color_at(value(0.75)), B);
        assert_eq!(resolved.color_at(value(0.9)), C);
        // Every color is still listed.
        assert_eq!(fill.stops().len(), 3);
    }

    #[test]
    fn color_for_solid() {
        let fill = Fill::solid(A);
        assert_eq!(fill.color_for(0.0), Some(A.into()));
        assert_eq!(fill.color_for(0.7), Some(A.into()));
        assert_eq!(fill.color_for(f32::NAN), Some(A.into()));
        assert_eq!(fill.color_for(-3.0), Some(A.into()));

        // Colors that cannot be resolved without a theme stay as they are.
        let fill = Fill::solid(PaletteColor::Highlight);
        assert_eq!(fill.color_for(0.5), Some(PaletteColor::Highlight.into()));
        let fill = Fill::solid(Color::TerminalDefault);
        assert_eq!(fill.color_for(0.5), Some(Color::TerminalDefault.into()));

        // A gradient with a single color is a solid color.
        assert_eq!(Fill::by_value([B]).color_for(0.5), Some(B.into()));
    }

    #[test]
    fn color_for_steps() {
        let fill = Fill::steps([(0.0, A), (0.7, B), (0.85, C)]);
        let color = |value| fill.color_for(value);
        assert_eq!(color(0.0), Some(A.into()));
        assert_eq!(color(0.6999), Some(A.into()));
        assert_eq!(color(0.7), Some(B.into()));
        assert_eq!(color(0.8499), Some(B.into()));
        assert_eq!(color(0.85), Some(C.into()));
        assert_eq!(color(1.0), Some(C.into()));
        assert_eq!(color(50.0), Some(C.into()));
        // Below the first threshold, and NaN, give the first color.
        assert_eq!(color(-1.0), Some(A.into()));
        assert_eq!(color(f32::NAN), Some(A.into()));

        let fill = Fill::steps([(0.5, A), (0.9, B)]);
        assert_eq!(fill.color_for(0.1), Some(A.into()));
        assert_eq!(fill.color_for(f32::NAN), Some(A.into()));

        // Unsorted steps are sorted, and the last of equal thresholds wins.
        let fill = Fill::steps([(0.85, C), (0.5, A), (0.5, B)]);
        assert_eq!(fill.color_for(0.6), Some(B.into()));
        assert_eq!(fill.color_for(0.9), Some(C.into()));
    }

    #[test]
    fn color_for_gradients_is_none() {
        assert_eq!(Fill::vertical([A, B]).color_for(0.5), None);
        assert_eq!(Fill::horizontal([A, B]).color_for(0.5), None);
        assert_eq!(Fill::by_value([A, B]).color_for(0.5), None);
        assert_eq!(Fill::by_value([A, B]).color_for(f32::NAN), None);
    }

    #[test]
    fn color_for_keeps_custom_keys() {
        static KEY: ColorKey = ColorKey::new("fill_test.color_for", BLACK);
        let fill = Fill::steps([(0.0, ColorType::from(&KEY)), (0.5, A.into())]);
        let color = fill.color_for(0.2).unwrap();
        assert_eq!(color, ColorType::from(&KEY));

        // Resolved late, so it follows the theme.
        let mut theme = Theme::retro();
        assert_eq!(color.resolve(&theme.palette, WHITE), BLACK);
        theme.palette.set_custom_color(&KEY, Color::Rgb(9, 8, 7));
        assert_eq!(color.resolve(&theme.palette, WHITE), Color::Rgb(9, 8, 7));
    }

    #[test]
    fn resolve_with_matches_resolve() {
        static KEY: ColorKey = ColorKey::new("fill_test.resolve_with", BLACK);
        let mut theme = Theme::retro();
        theme.palette.set_custom_color(&KEY, Color::Rgb(9, 8, 7));

        let fills = [
            Fill::solid(PaletteColor::Highlight),
            Fill::solid(&KEY),
            Fill::solid(ColorType::InheritParent),
            Fill::solid(Color::TerminalDefault),
            Fill::steps([
                (0.0, ColorType::from(&KEY)),
                (0.5, ColorType::InheritParent),
            ]),
            Fill::by_value([ColorType::from(&KEY), WHITE.into()]),
            Fill::horizontal([ColorType::from(PaletteColor::Highlight), BLACK.into()]),
            Fill::vertical([ColorType::InheritParent, WHITE.into()]),
        ];
        let (parent, with_printer) = with_printer(&theme, |printer| {
            let resolved: Vec<_> = fills.iter().map(|fill| fill.resolve(printer)).collect();
            (printer.current_color().front, resolved)
        });

        for (fill, expected) in fills.iter().zip(with_printer) {
            let resolved = fill.resolve_with(&theme.palette, parent);
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let pos = FillPos {
                    x: t,
                    y: 1.0 - t,
                    value: t,
                };
                assert_eq!(resolved.color_at(pos), expected.color_at(pos), "{fill:?}");
            }
        }

        // The parent is the color of `InheritParent`.
        let fill = Fill::solid(ColorType::InheritParent);
        let resolved = fill.resolve_with(&theme.palette, A);
        assert_eq!(resolved.color_at(FillPos::default()), A);
    }

    #[test]
    fn steps_below_the_first_threshold() {
        let theme = Theme::retro();
        let fill = Fill::steps([(0.5, A), (0.9, B)]);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(value(0.0)), A);
        assert_eq!(resolved.color_at(value(-4.0)), A);
        assert_eq!(resolved.color_at(value(f32::NEG_INFINITY)), A);
        assert_eq!(resolved.color_at(value(f32::NAN)), A);
        assert_eq!(resolved.color_at(value(0.5)), A);
        assert_eq!(resolved.color_at(value(0.9)), B);
    }

    #[test]
    fn steps_single_step() {
        let theme = Theme::retro();
        let resolved = with_printer(&theme, |printer| Fill::steps([(0.5, A)]).resolve(printer));
        assert_eq!(resolved.color_at(value(0.0)), A);
        assert_eq!(resolved.color_at(value(1.0)), A);
    }

    #[test]
    fn steps_keep_non_rgb_colors() {
        let theme = Theme::retro();
        let fill = Fill::steps([
            (0.0, Color::TerminalDefault),
            (0.5, Color::Dark(crate::style::BaseColor::Red)),
        ]);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(value(0.1)), Color::TerminalDefault);
        assert_eq!(
            resolved.color_at(value(0.6)),
            Color::Dark(crate::style::BaseColor::Red)
        );
    }

    #[test]
    fn steps_follow_the_theme() {
        static OK: ColorKey = ColorKey::new("fill_test.steps.ok", A);
        static BAD: ColorKey = ColorKey::new("fill_test.steps.bad", B);

        let fill = Fill::steps([(0.0, ColorType::from(&OK)), (0.5, ColorType::from(&BAD))]);

        let mut theme = Theme::retro();
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(value(0.1)), A);
        assert_eq!(resolved.color_at(value(0.9)), B);

        theme.palette.set_custom_color(&BAD, C);
        theme.palette.set_custom_color(&OK, WHITE);
        let resolved = with_printer(&theme, |printer| fill.resolve(printer));
        assert_eq!(resolved.color_at(value(0.1)), WHITE);
        assert_eq!(resolved.color_at(value(0.9)), C);
    }

    #[test]
    #[should_panic(expected = "at least one color")]
    fn steps_cannot_be_empty() {
        Fill::steps(Vec::<(f32, Color)>::new());
    }

    #[test]
    #[should_panic(expected = "NaN")]
    fn steps_thresholds_cannot_be_nan() {
        Fill::steps([(f32::NAN, A)]);
    }

    #[test]
    fn cell_positions() {
        assert_eq!(
            FillPos::cell(0, 3, (5, 4), 0.5),
            FillPos {
                x: 0.0,
                y: 1.0,
                value: 0.5
            }
        );
        assert_eq!(FillPos::cell(4, 0, (5, 4), 0.0).x, 1.0);
        // A single cell is at the start.
        assert_eq!(FillPos::cell(0, 0, (1, 1), 0.0).x, 0.0);
    }

    #[test]
    fn default_series_colors_cycle() {
        assert_eq!(series_color(0).name(), "chart.series.0");
        assert_eq!(series_color(SERIES.len() + 1).name(), "chart.series.1");

        // They can be set from a theme.
        let mut theme = Theme::retro();
        theme.palette.set_color("chart.series.1", WHITE);
        with_printer(&theme, |printer| {
            assert_eq!(printer.resolve_color(series_color(1)), WHITE);
            assert_eq!(
                printer.resolve_color(&AXIS),
                theme.palette[PaletteColor::Tertiary]
            );
        });
    }
}
