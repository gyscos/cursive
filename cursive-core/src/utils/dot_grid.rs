//! Drawing with a resolution finer than terminal cells.

use crate::Printer;
use crate::Vec2;
use crate::style::{ColorStyle, ColorType};

/// How a [`DotGrid`] splits each terminal cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DotMode {
    /// Braille characters: 2×4 dots per cell.
    ///
    /// All the dots in a cell share one color: the last one given for that cell.
    Braille,

    /// Half-block characters: 1×2 dots per cell.
    ///
    /// Each dot has its own color, and entirely fills its half of the cell.
    HalfBlocks,
}

impl DotMode {
    /// Returns the number of dots in each cell.
    pub const fn dots_per_cell(self) -> Vec2 {
        match self {
            DotMode::Braille => Vec2::new(2, 4),
            DotMode::HalfBlocks => Vec2::new(1, 2),
        }
    }
}

/// What we know about one terminal cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    /// Which dots are set, and their color.
    Braille(u8, ColorType),
    /// The top and bottom halves.
    Halves([Option<ColorType>; 2]),
}

/// Returns `floor((2 * share * k + whole) / (2 * whole))`: `k * share / whole`, rounded to the
/// nearest integer (halves up), for `share <= whole` and `k <= whole`, without overflowing.
fn rounded_share(share: u128, whole: u128, k: u128) -> u128 {
    // With `k = t * whole + r`, the result is `t * share + (share * r + whole / 2) / whole`.
    // This does not need the double of anything as large as `share * r`.
    let (t, r) = (k / whole, k % whole);
    t * share + (share * r + whole / 2) / whole
}

/// A grid of dots smaller than terminal cells, to draw lines, curves or small pictures.
///
/// The grid is given a size in cells, and exposes a larger [resolution](Self::resolution) in
/// dots. Dots are set with [`set`](Self::set) or [`line`](Self::line), and the result is printed
/// with [`draw`](Self::draw). Cells without any dot are left untouched when drawing.
///
/// A view will usually create a new grid in its `draw` method, using the size of the printer.
///
/// # Examples
///
/// ```rust
/// use cursive_core::Printer;
/// use cursive_core::style::PaletteColor;
/// use cursive_core::utils::DotGrid;
///
/// fn draw(printer: &Printer) {
///     let mut grid = DotGrid::braille(printer.size);
///     let dots = grid.resolution();
///
///     // A diagonal across the entire area.
///     grid.line((0, 0), dots.saturating_sub((1, 1)), PaletteColor::Highlight);
///     grid.draw(printer);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct DotGrid {
    mode: DotMode,
    // Size in cells.
    size: Vec2,
    cells: Vec<Cell>,
}

impl DotGrid {
    /// Creates a new empty grid covering `size` cells.
    pub fn new(mode: DotMode, size: impl Into<Vec2>) -> Self {
        let size = size.into();
        let empty = match mode {
            DotMode::Braille => Cell::Braille(0, ColorType::InheritParent),
            DotMode::HalfBlocks => Cell::Halves([None; 2]),
        };
        DotGrid {
            mode,
            size,
            cells: vec![empty; size.x * size.y],
        }
    }

    /// Creates a new empty grid of braille dots covering `size` cells.
    ///
    /// Its resolution is `(2 * size.x, 4 * size.y)`.
    pub fn braille(size: impl Into<Vec2>) -> Self {
        Self::new(DotMode::Braille, size)
    }

    /// Creates a new empty grid of half-blocks covering `size` cells.
    ///
    /// Its resolution is `(size.x, 2 * size.y)`.
    pub fn half_blocks(size: impl Into<Vec2>) -> Self {
        Self::new(DotMode::HalfBlocks, size)
    }

    /// Returns how this grid splits each cell.
    pub fn mode(&self) -> DotMode {
        self.mode
    }

    /// Returns the size of this grid, in terminal cells.
    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// Returns the size of this grid, in dots.
    ///
    /// Valid dot positions go from `(0, 0)` (top-left) to `resolution() - (1, 1)`.
    pub fn resolution(&self) -> Vec2 {
        self.size * self.mode.dots_per_cell()
    }

    /// Removes all dots.
    pub fn clear(&mut self) {
        *self = Self::new(self.mode, self.size);
    }

    /// Sets the dot at the given position.
    ///
    /// Positions outside of the grid are ignored.
    pub fn set(&mut self, pos: impl Into<Vec2>, color: impl Into<ColorType>) {
        let pos = pos.into();
        let color = color.into();
        let per_cell = self.mode.dots_per_cell();

        if !pos.fits_in(self.resolution().saturating_sub((1, 1))) || self.cells.is_empty() {
            return;
        }

        let cell = &mut self.cells[(pos.y / per_cell.y) * self.size.x + pos.x / per_cell.x];
        let (x, y) = (pos.x % per_cell.x, pos.y % per_cell.y);
        match cell {
            Cell::Braille(bits, cell_color) => {
                // Dots are numbered column by column, except for the last row.
                const BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
                *bits |= BITS[x][y];
                *cell_color = color;
            }
            Cell::Halves(halves) => halves[y] = Some(color),
        }
    }

    /// Draws a straight line between two dots (both included).
    ///
    /// Parts of the line outside of the grid are ignored, and cost nothing: the endpoints can be
    /// anywhere (even `usize::MAX`), and the time spent only depends on the size of the grid.
    ///
    /// The line is the one Bresenham's algorithm gives between the two endpoints: the dots set
    /// inside the grid are exactly the ones that drawing the whole line would set, whatever the
    /// grid does with the rest.
    pub fn line(
        &mut self,
        from: impl Into<Vec2>,
        to: impl Into<Vec2>,
        color: impl Into<ColorType>,
    ) {
        let color = color.into();
        let (from, to) = (from.into(), to.into());
        let resolution = self.resolution();
        if resolution.x == 0 || resolution.y == 0 {
            return;
        }

        // Wide integers: nothing below can overflow, whatever the endpoints.
        let (x0, y0) = (from.x as i128, from.y as i128);
        let (x1, y1) = (to.x as i128, to.y as i128);
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        let (step_x, step_y) = ((x1 - x0).signum(), (y1 - y0).signum());

        // Bresenham's algorithm walks along the longest axis, one dot at a time, and moves along
        // the other axis by the rounded (halves up) share of the line done so far.
        // Instead of walking from the start, we skip to the part inside the grid: the position on
        // the shorter axis is computed directly for each step.
        // `major` is the position along the longest axis, `minor` the one along the other.
        let (major_start, major_step, major_len, major_size) = if dx >= dy {
            (x0, step_x, dx, resolution.x)
        } else {
            (y0, step_y, dy, resolution.y)
        };
        let (minor_start, minor_step, minor_len, minor_size) = if dx >= dy {
            (y0, step_y, dy, resolution.y)
        } else {
            (x0, step_x, dx, resolution.x)
        };

        // The steps `k` in `0..=major_len` that are inside the grid on the longest axis.
        let last = major_size as i128 - 1;
        let (first_step, last_step) = if major_step >= 0 {
            (0, major_len.min(last - major_start))
        } else {
            ((major_start - last).max(0), major_len.min(major_start))
        };

        for k in first_step..=last_step {
            let major = major_start + major_step * k;
            // The rounded share of the shorter axis (`major_len` is 0 for a single dot).
            let advance = if major_len == 0 {
                0
            } else {
                rounded_share(minor_len as u128, major_len as u128, k as u128) as i128
            };
            let minor = minor_start + minor_step * advance;
            if !(0..minor_size as i128).contains(&minor) {
                continue;
            }
            let (x, y) = if dx >= dy {
                (major, minor)
            } else {
                (minor, major)
            };
            self.set((x as usize, y as usize), color);
        }
    }

    /// Prints this grid, with its top-left corner at the origin of the printer.
    ///
    /// Cells without any dot are not printed.
    pub fn draw(&self, printer: &Printer) {
        for (i, cell) in self.cells.iter().enumerate() {
            let pos = Vec2::new(i % self.size.x, i / self.size.x);
            let (style, glyph) = match *cell {
                Cell::Braille(0, _) | Cell::Halves([None, None]) => continue,
                Cell::Braille(bits, color) => (
                    ColorStyle::front(color),
                    char::from_u32(0x2800 + bits as u32).unwrap_or(' '),
                ),
                Cell::Halves([Some(top), Some(bottom)]) => (ColorStyle::new(top, bottom), '▀'),
                Cell::Halves([Some(top), None]) => (ColorStyle::front(top), '▀'),
                Cell::Halves([None, Some(bottom)]) => (ColorStyle::front(bottom), '▄'),
            };
            printer.with_color(style, |printer| {
                printer.print(pos, glyph.encode_utf8(&mut [0; 4]));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::PrintBuffer;
    use crate::style::{Color, ColorKey, PaletteColor};
    use crate::theme::Theme;
    use parking_lot::RwLock;

    const RED: Color = Color::Rgb(255, 0, 0);
    const BLUE: Color = Color::Rgb(0, 0, 255);

    /// Draws the grid, and returns the text, front and back color of each cell.
    fn render(grid: &DotGrid, theme: &Theme) -> Vec<Vec<(String, Color, Color)>> {
        let size = grid.size();
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        let buffer = RwLock::new(buffer);
        grid.draw(&Printer::new(size, theme, &buffer));

        let buffer = buffer.read();
        (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| {
                        let pos = Vec2::new(x, y);
                        // Cells that were never printed have no style.
                        let unset = Color::TerminalDefault;
                        let (front, back) =
                            buffer.cell_style(pos).map_or((unset, unset), |style| {
                                (style.color.front, style.color.back)
                            });
                        (buffer.cell_text(pos).unwrap_or("").to_string(), front, back)
                    })
                    .collect()
            })
            .collect()
    }

    fn text(grid: &DotGrid) -> Vec<String> {
        render(grid, &Theme::retro())
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(text, _, _)| if text.is_empty() { ".".into() } else { text })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn resolution() {
        assert_eq!(DotGrid::braille((3, 2)).resolution(), Vec2::new(6, 8));
        assert_eq!(DotGrid::half_blocks((3, 2)).resolution(), Vec2::new(3, 4));
        assert_eq!(DotGrid::braille((0, 0)).resolution(), Vec2::zero());
    }

    #[test]
    fn braille_dots() {
        let mut grid = DotGrid::braille((2, 1));
        // Each corner of the first cell, one after the other.
        for (pos, expected) in [((0, 0), "⠁"), ((1, 0), "⠉"), ((0, 3), "⡉"), ((1, 3), "⣉")]
        {
            grid.set(pos, RED);
            assert_eq!(text(&grid), [format!("{expected}.")]);
        }

        // The second cell is separate.
        grid.set((2, 1), RED);
        assert_eq!(text(&grid), ["⣉⠂"]);

        grid.clear();
        assert_eq!(text(&grid), [".."]);
    }

    #[test]
    fn braille_color_is_per_cell() {
        let mut grid = DotGrid::braille((2, 1));
        grid.set((0, 0), RED);
        grid.set((1, 1), BLUE);
        grid.set((2, 0), RED);

        let theme = Theme::retro();
        let row = &render(&grid, &theme)[0];
        // The last color wins in the first cell.
        assert_eq!((row[0].0.as_str(), row[0].1), ("⠑", BLUE));
        assert_eq!((row[1].0.as_str(), row[1].1), ("⠁", RED));
    }

    #[test]
    fn half_blocks() {
        let mut grid = DotGrid::half_blocks((3, 1));
        grid.set((0, 0), RED);
        grid.set((1, 1), BLUE);
        grid.set((2, 0), RED);
        grid.set((2, 1), BLUE);
        assert_eq!(text(&grid), ["▀▄▀"]);

        let theme = Theme::retro();
        // Halves without a dot keep the current background.
        let background = Printer::new((1, 1), &theme, &RwLock::new(PrintBuffer::new()))
            .current_color()
            .back;
        let row = &render(&grid, &theme)[0];
        assert_eq!((row[0].1, row[0].2), (RED, background));
        assert_eq!((row[1].1, row[1].2), (BLUE, background));
        assert_eq!((row[2].1, row[2].2), (RED, BLUE));
    }

    #[test]
    fn out_of_bounds_is_ignored() {
        let mut grid = DotGrid::braille((1, 1));
        grid.set((2, 0), RED);
        grid.set((0, 4), RED);
        grid.line((5, 5), (9, 9), RED);
        assert_eq!(text(&grid), ["."]);

        let mut empty = DotGrid::half_blocks((0, 0));
        empty.set((0, 0), RED);
        empty.line((0, 0), (3, 3), RED);
        assert!(text(&empty).is_empty());
    }

    #[test]
    fn lines() {
        // Horizontal, on the second row of dots.
        let mut grid = DotGrid::braille((2, 1));
        grid.line((0, 1), (3, 1), RED);
        assert_eq!(text(&grid), ["⠒⠒"]);

        // Vertical, bottom to top.
        let mut grid = DotGrid::braille((1, 2));
        grid.line((1, 7), (1, 0), RED);
        assert_eq!(text(&grid), ["⢸", "⢸"]);

        // Diagonals, in both directions.
        let mut grid = DotGrid::braille((2, 1));
        grid.line((0, 0), (3, 3), RED);
        assert_eq!(text(&grid), ["⠑⢄"]);
        let mut grid = DotGrid::braille((2, 1));
        grid.line((3, 0), (0, 3), RED);
        assert_eq!(text(&grid), ["⡠⠊"]);

        // A single dot.
        let mut grid = DotGrid::half_blocks((1, 1));
        grid.line((0, 1), (0, 1), RED);
        assert_eq!(text(&grid), ["▄"]);

        // A line leaving the grid draws the part inside.
        let mut grid = DotGrid::half_blocks((2, 1));
        grid.line((0, 0), (5, 0), RED);
        assert_eq!(text(&grid), ["▀▀"]);
    }

    #[test]
    fn colors_follow_the_theme() {
        static KEY: ColorKey = ColorKey::from_palette("dot_grid_test.key", PaletteColor::Secondary);

        let mut grid = DotGrid::braille((2, 1));
        grid.set((0, 0), PaletteColor::Highlight);
        grid.set((2, 0), &KEY);

        let mut theme = Theme::retro();
        let row = &render(&grid, &theme)[0];
        assert_eq!(row[0].1, theme.palette[PaletteColor::Highlight]);
        assert_eq!(row[1].1, theme.palette[PaletteColor::Secondary]);

        theme.palette.set_custom_color(&KEY, BLUE);
        assert_eq!(render(&grid, &theme)[0][1].1, BLUE);
    }

    /// Runs `f` on another thread, and fails if it takes more than 3 seconds.
    fn within_3s(f: impl FnOnce() + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            f();
            let _ = tx.send(());
        });
        if let Err(e) = rx.recv_timeout(std::time::Duration::from_secs(3)) {
            panic!("did not finish: {e:?}");
        }
    }

    #[test]
    fn line_with_huge_coordinates_is_instant() {
        // The grid is 1x1 cells: only the part inside matters.
        within_3s(|| {
            let mut grid = DotGrid::braille((1, 1));
            grid.line((0, 0), (1usize << 40, 0), RED);
            assert_eq!(text(&grid), ["⠉"]);
            // Both ends far away, passing through the grid.
            let mut grid = DotGrid::braille((1, 1));
            grid.line((0, 1usize << 50), (1usize << 50, 0), RED);
            assert_eq!(text(&grid), ["."]);
        });
    }

    #[test]
    fn line_with_far_endpoints_does_not_overflow() {
        for far in [
            1usize << 31,
            1 << 62,
            (1 << 63) - 1,
            1 << 63,
            usize::MAX - 1,
            usize::MAX,
        ] {
            for (from, to) in [
                ((0, 0), (far, 0)),
                ((0, 0), (0, far)),
                ((0, 0), (far, far)),
                ((far, far), (0, 0)),
                ((far, 0), (0, far)),
                ((far, far), (far - 1, far)),
                ((1, 1), (far, far)),
                ((3, 2), (far, far / 2)),
            ] {
                let mut grid = DotGrid::braille((2, 2));
                within_3s(move || grid.line(from, to, RED));
            }
        }

        // The visible part is still the right one.
        let mut grid = DotGrid::braille((2, 1));
        grid.line((0, 0), (usize::MAX, 0), RED);
        assert_eq!(text(&grid), ["⠉⠉"]);
        let mut grid = DotGrid::braille((2, 1));
        grid.line((usize::MAX, 0), (0, 0), RED);
        assert_eq!(text(&grid), ["⠉⠉"]);
        let mut grid = DotGrid::braille((1, 1));
        grid.line((0, 0), (usize::MAX, usize::MAX), RED);
        assert_eq!(text(&grid), ["⠑"]);
    }

    /// What `line` did before it clipped: the Bresenham algorithm on the whole line.
    fn unclipped_line(grid: &mut DotGrid, from: (usize, usize), to: (usize, usize), color: Color) {
        let (mut x, mut y) = (from.0 as isize, from.1 as isize);
        let (to_x, to_y) = (to.0 as isize, to.1 as isize);
        let (dx, dy) = ((to_x - x).abs(), -(to_y - y).abs());
        let (step_x, step_y) = ((to_x - x).signum(), (to_y - y).signum());
        let mut error = dx + dy;

        loop {
            grid.set((x as usize, y as usize), color);
            if (x, y) == (to_x, to_y) {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x += step_x;
            }
            if doubled <= dx {
                error += dx;
                y += step_y;
            }
        }
    }

    fn assert_line_matches_unclipped(
        mode: DotMode,
        size: (usize, usize),
        from: (usize, usize),
        to: (usize, usize),
    ) {
        let mut clipped = DotGrid::new(mode, size);
        clipped.line(from, to, RED);
        let mut full = DotGrid::new(mode, size);
        unclipped_line(&mut full, from, to, RED);
        assert!(
            clipped.cells == full.cells,
            "{mode:?} {size:?}: {from:?} -> {to:?}\n{:?}\n{:?}",
            text(&clipped),
            text(&full)
        );
    }

    #[test]
    fn clipped_lines_are_the_visible_part_of_the_whole_line() {
        // Every line between nearby dots, inside the grid or just around it.
        for mode in [DotMode::Braille, DotMode::HalfBlocks] {
            for size in [(1, 1), (2, 1), (3, 2)] {
                let res = DotGrid::new(mode, size).resolution();
                for x0 in 0..res.x + 3 {
                    for y0 in 0..res.y + 3 {
                        for x1 in 0..res.x + 3 {
                            for y1 in 0..res.y + 3 {
                                assert_line_matches_unclipped(mode, size, (x0, y0), (x1, y1));
                            }
                        }
                    }
                }
            }
        }

        // Endpoints farther away, where the old algorithm still ends.
        let coordinates = [0, 2, 5, 8, 13, 99, 100, 301];
        for mode in [DotMode::Braille, DotMode::HalfBlocks] {
            for size in [(1, 1), (3, 2)] {
                for x0 in coordinates {
                    for y0 in coordinates {
                        for x1 in coordinates {
                            for y1 in coordinates {
                                assert_line_matches_unclipped(mode, size, (x0, y0), (x1, y1));
                            }
                        }
                    }
                }
            }
        }

        // Many other slopes.
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut random = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..4000 {
            let mode = [DotMode::Braille, DotMode::HalfBlocks][random(2)];
            let size = (random(8), random(5));
            let reach = [10, 60, 600][random(3)];
            let from = (random(reach), random(reach));
            let to = (random(reach), random(reach));
            assert_line_matches_unclipped(mode, size, from, to);
        }
    }

    #[test]
    fn sweeping_every_size_and_position_does_not_panic() {
        for mode in [DotMode::Braille, DotMode::HalfBlocks] {
            for w in 0..=6 {
                for h in 0..=4 {
                    let mut grid = DotGrid::new(mode, (w, h));
                    let res = grid.resolution();
                    for x in 0..res.x + 3 {
                        for y in 0..res.y + 3 {
                            grid.set((x, y), RED);
                            grid.line((0, 0), (x, y), BLUE);
                            grid.line((x, y), (0, res.y), BLUE);
                        }
                    }
                    // Drawing in a smaller or larger printer never panics.
                    for pw in 0..=7 {
                        let mut buffer = PrintBuffer::new();
                        buffer.resize(Vec2::new(pw, 5));
                        let buffer = RwLock::new(buffer);
                        grid.draw(&Printer::new((pw, 5), &Theme::retro(), &buffer));
                    }
                }
            }
        }
    }

    #[test]
    fn rounded_share_is_the_rounded_ratio() {
        for whole in 1u128..40 {
            for share in 0..=whole {
                for k in 0..=whole {
                    assert_eq!(
                        rounded_share(share, whole, k),
                        (2 * share * k + whole) / (2 * whole),
                        "{share}/{whole} of {k}"
                    );
                }
            }
        }
        // Without overflowing: at the largest sizes a `usize` allows.
        let max = u64::MAX as u128;
        assert_eq!(rounded_share(max, max, max), max);
        assert_eq!(rounded_share(0, max, max), 0);
        assert_eq!(rounded_share(max - 1, max, max), max - 1);
        assert_eq!(rounded_share(max / 2, max, max), max / 2);
    }
}
