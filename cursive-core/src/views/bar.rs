//! Drawing code shared by [`ProgressBar`](crate::views::ProgressBar) and
//! [`Gauge`](crate::views::Gauge).

use crate::Printer;
use crate::printer::H_BLOCKS;
use crate::style::chart::{Fill, FillPos};
use crate::style::{ColorStyle, ColorType, PaletteColor};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// How a bar is drawn.
///
/// This is used by [`ProgressBar`](crate::views::ProgressBar) and
/// [`Gauge`](crate::views::Gauge).
///
/// More styles may be added in the future.
///
/// # Examples
///
/// ```rust
/// use cursive_core::views::{BarStyle, ProgressBar};
///
/// let bar = ProgressBar::new().style(BarStyle::Block);
///
/// assert_eq!("line".parse(), Ok(BarStyle::Line));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BarStyle {
    /// A solid colored bar. A label, if any, is drawn inside it.
    ///
    /// The precision is an eighth of a cell. The unfilled part is left as it
    /// is, so the bar uses no track.
    Solid,
    /// A thin line: `━` for both the track and the filled part.
    ///
    /// The precision is one cell.
    Line,
    /// A bar of block characters over a track.
    ///
    /// The precision is an eighth of a cell (see [`Printer::print_hbar_with`]).
    /// The track is drawn as a background color, so a partially filled cell
    /// shows the track behind it.
    Block,
}

impl std::str::FromStr for BarStyle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "solid" => Ok(BarStyle::Solid),
            "line" => Ok(BarStyle::Line),
            "block" => Ok(BarStyle::Block),
            _ => Err(()),
        }
    }
}

impl BarStyle {
    /// Returns the filled length, in eighths of a cell, for a ratio from 0 to 1.
    ///
    /// The result is rounded to what the style can show: a whole number of
    /// cells for `Line`.
    pub(crate) fn eighths(self, ratio: f32, width: usize) -> usize {
        let ratio = ratio.clamp(0.0, 1.0);
        match self {
            BarStyle::Line => (ratio * width as f32).round() as usize * 8,
            BarStyle::Solid | BarStyle::Block => (ratio * (width * 8) as f32).round() as usize,
        }
    }
}

/// Where the label of a [`ProgressBar`](crate::views::ProgressBar) goes.
///
/// More positions may be added in the future.
///
/// # Examples
///
/// ```rust
/// use cursive_core::views::{BarStyle, LabelPosition, ProgressBar};
///
/// let bar = ProgressBar::new()
///     .style(BarStyle::Line)
///     .label_position(LabelPosition::Hidden);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LabelPosition {
    /// Centered over the bar.
    ///
    /// This is the default for [`BarStyle::Solid`], where the label changes
    /// color with the bar. With other styles it is drawn over the bar without
    /// changing colors, which is rarely what you want.
    Inside,
    /// To the right of the bar.
    ///
    /// The bar is made shorter by the width of the label plus one space.
    /// This is the default for [`BarStyle::Line`] and [`BarStyle::Block`].
    ///
    /// The room for the label does not change with the value: it is the width of the widest of
    /// the labels for the minimum, the maximum and the current value (so the bar keeps its size
    /// while the label gains a digit), and the label is aligned to the right of that room.
    /// A label that is wider than the view is cropped.
    Right,
    /// No label.
    Hidden,
}

impl std::str::FromStr for LabelPosition {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inside" => Ok(LabelPosition::Inside),
            "right" => Ok(LabelPosition::Right),
            "hidden" => Ok(LabelPosition::Hidden),
            _ => Err(()),
        }
    }
}

/// The part of the drawing that is the same for all bars.
///
/// This draws a single line, over the full width of the printer.
pub(crate) struct Bar<'a> {
    pub style: BarStyle,
    /// Length of the filled part, in eighths of a cell.
    ///
    /// This can be more than the width of the bar, in which case the bar is full.
    pub eighths: usize,
    /// How complete the bar is, from 0 to 1: this is the value seen by the fill.
    pub ratio: f32,
    /// Colors the filled part. It sees the whole bar as its area.
    pub fill: &'a Fill,
    /// Color of the unfilled part, for `Line` and `Block`.
    pub track: ColorType,
    /// Label to draw inside the bar, centered. Only used by `Solid`.
    pub label: Option<&'a str>,
}

/// Block characters for the last, partially filled cell of a solid bar.
fn sub_block(extra: usize) -> &'static str {
    match extra {
        0 => " ",
        n => H_BLOCKS[(n - 1).min(7)],
    }
}

impl Bar<'_> {
    pub(crate) fn draw(&self, printer: &Printer) {
        let width = printer.size.x;
        if width == 0 || printer.size.y == 0 {
            return;
        }

        let fill = self.fill.resolve(printer);
        let color_at = |column: usize| {
            fill.color_at(FillPos::cell(
                column.min(width - 1),
                0,
                (width, 1),
                self.ratio,
            ))
        };

        match self.style {
            BarStyle::Solid => {
                let length = self.eighths / 8;
                let extra = self.eighths % 8;

                // The partial cell uses the bar color on the existing background.
                if length < width {
                    printer.with_color(ColorStyle::front(color_at(length)), |printer| {
                        printer.print((length, 0), sub_block(extra));
                    });
                }

                for column in 0..length.min(width) {
                    let color = ColorStyle::new(PaletteColor::HighlightText, color_at(column));
                    printer.with_color(color, |printer| {
                        printer.print((column, 0), " ");
                    });
                }

                if let Some(label) = self.label {
                    let offset = crate::align::HAlign::Center.get_offset(label.width(), width);
                    let mut column = offset;
                    for grapheme in label.graphemes(true) {
                        let grapheme_width = grapheme.width();
                        if grapheme_width == 0 {
                            continue;
                        }
                        if column >= width {
                            break;
                        }
                        // The label changes color with the bar. A character that would
                        // cross the edge of the filled part is not drawn: it would leave a
                        // hole in the bar (or cut the character in two), so its cell on
                        // the bar stays a filled space.
                        if column < length && column + grapheme_width > length {
                            column += grapheme_width;
                            continue;
                        }
                        let color = if column < length {
                            ColorStyle::new(PaletteColor::HighlightText, color_at(column))
                        } else {
                            ColorStyle::front(color_at(column))
                        };
                        printer.with_color(color, |printer| {
                            printer.print((column, 0), grapheme);
                        });
                        column += grapheme_width;
                    }
                }
            }
            BarStyle::Line => {
                printer.with_color(ColorStyle::front(self.track), |printer| {
                    printer.print_hline((0, 0), width, "━");
                });
                // Rounded to the nearest cell.
                let filled = ((self.eighths + 4) / 8).min(width);
                for column in 0..filled {
                    printer.with_color(ColorStyle::front(color_at(column)), |printer| {
                        printer.print((column, 0), "━");
                    });
                }
            }
            BarStyle::Block => {
                // The track is drawn first, as the bar leaves empty cells alone.
                printer.with_color(ColorStyle::back(self.track), |printer| {
                    printer.print_hline((0, 0), width, " ");
                });
                printer.print_hbar_eighths((0, 0), width, self.eighths, |column| {
                    ColorStyle::new(color_at(column), self.track)
                });
            }
        }
    }
}
