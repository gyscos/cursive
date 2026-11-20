//! Provide higher-level abstraction to draw things on buffers.

use crate::Vec2;
use crate::align::HAlign;
use crate::buffer::{PrintBuffer, Window};
use crate::direction::Orientation;
use crate::rect::Rect;
use crate::style::chart::{Fill, FillPos};
use crate::style::{
    BORDER, BORDER_FOCUSED, BorderStyle, Color, ColorKey, ColorPair, ColorStyle, ColorType,
    ConcreteStyle, Effect, PaletteColor, PaletteStyle, Style, StyleType,
};
use crate::theme::Theme;
use crate::utils::lines::simple::{prefix, suffix};
use crate::utils::span::IndexedSpan;
use crate::with::With;

use enumset::EnumSet;
use parking_lot::RwLock;
use std::cell::Cell;
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Convenient interface to draw on a subset of the screen.
///
/// The printing area is defined by `offset` and `size`.\
/// The content that will be printed is defined by [`Self::content_offset`]
/// and [`Self::size`].
///
/// If the printer is asked to print outside of the printing area,
/// then the string to be printed shall be truncated without throwing errors.
/// Refer to the [`crate::traits::View`] to understand how to change its size.
#[derive(Clone)]
pub struct Printer<'a, 'b> {
    /// Offset into the window this printer should start drawing at.
    ///
    /// A print request at `x` will really print at `x + offset`.
    pub offset: Vec2,

    /// Size of the area we are allowed to draw on.
    ///
    /// Anything outside of this should be discarded.\
    /// The view being drawn can ignore this, but anything further than that
    /// will be ignored.
    pub output_size: Vec2,

    /// Size allocated to the view.
    ///
    /// This should be the same value as the one given in the last call to
    /// `View::layout`.
    pub size: Vec2,

    /// Offset into the view for this printer.
    ///
    /// The view being drawn can ignore this, but anything to the top-left of
    /// this will actually be ignored, so it can be used to skip this part.
    ///
    /// A print request `x`, will really print at `x - content_offset`.
    pub content_offset: Vec2,

    /// Whether the view to draw is currently focused or not.
    pub focused: bool,

    /// Whether the view to draw is currently enabled or not.
    pub enabled: bool,

    /// Currently used theme
    pub theme: &'a Theme,

    /// Current style used
    current_style: Cell<ConcreteStyle>,

    /// Backend used to actually draw things
    buffer: &'b RwLock<PrintBuffer>,
}

impl<'a, 'b> Printer<'a, 'b> {
    /// Creates a new printer on the given window.
    ///
    /// But nobody needs to know that.
    #[doc(hidden)]
    pub fn new<T: Into<Vec2>>(size: T, theme: &'a Theme, buffer: &'b RwLock<PrintBuffer>) -> Self {
        let size = size.into();
        let mut printer = Printer {
            offset: Vec2::zero(),
            content_offset: Vec2::zero(),
            output_size: size,
            size,
            focused: true,
            enabled: true,
            theme,
            buffer,
            current_style: Cell::new(ConcreteStyle {
                color: ColorPair::terminal_default(),
                effects: EnumSet::empty(),
            }),
        };
        printer.clamp_output_to_buffer();
        printer
    }

    /// Returns the region of the window where this printer can write.
    pub fn output_window(&self) -> Rect {
        Rect::from_size(self.offset, self.output_size)
    }

    /// Pulls the output window back inside the backend buffer.
    ///
    /// Prints outside the buffer are meaningless, so every time the offset
    /// or the output size changes, the window is clamped to the buffer.
    /// Only the output viewport is affected; the virtual [`Self::size`]
    /// reported to the view is left untouched.
    fn clamp_output_to_buffer(&mut self) {
        let buffer_size = self.buffer.read().size();
        // Clamp the origin into the buffer. A zero-size window sitting
        // exactly on the buffer edge still fits (`fits_in` is inclusive).
        self.offset = self.offset.or_min(buffer_size);
        // Shrink the output so the window ends inside the buffer.
        let end = (self.offset + self.output_size).or_min(buffer_size);
        self.output_size = end.saturating_sub(self.offset);
    }

    /// The range of coordinates along `orientation` that would be visible,
    /// in the view's own coordinates (empty if nothing is visible at all).
    ///
    /// Anything printed outside of it is discarded: containers can skip
    /// drawing children entirely outside of it.
    pub(crate) fn visible_range(&self, orientation: Orientation) -> std::ops::Range<usize> {
        if self.output_size.x == 0 || self.output_size.y == 0 {
            return 0..0;
        }
        let start = *self.content_offset.get(orientation);
        start..start + *self.output_size.get(orientation)
    }

    /// Returns the size of the entire buffer.
    ///
    /// This is the size of the entire terminal, not just the area this printer can write into.
    pub fn buffer_size(&self) -> Vec2 {
        self.buffer.read().size()
    }

    /// Clear the screen.
    ///
    /// It will discard anything drawn before.
    ///
    /// Users rarely need to call this directly.
    pub fn clear(&self) {
        let color = self.theme.palette[PaletteColor::Background];
        self.buffer.write().fill(
            " ",
            ColorPair {
                front: color,
                back: color,
            },
        );
    }

    /// Prints some styled text at the given position.
    pub fn print_styled<V, S>(&self, start: V, text: S)
    where
        V: Into<Vec2>,
        S: crate::utils::span::SpannedText<S = IndexedSpan<Style>>,
    {
        let Vec2 { mut x, y } = start.into();
        for span in text.spans() {
            let span = span.resolve(text.source());
            self.with_style(*span.attr, |printer| {
                printer.print_with_width((x, y), span.content, |_| span.width);
                x += span.width;
            });
        }
    }

    /// Prints some text at the given position.
    ///
    /// # Parameters
    /// * `start` is the offset used to print the text in the view.
    /// * `text` is a string to print on the screen. It must be a single line, no line wrapping
    ///   will be done.
    ///
    /// # Description
    /// Prints some text at the given position.
    /// The text could be truncated if it exceed the [drawing area size](Self::output_size).
    ///
    /// # Example
    /// ```rust
    /// # use cursive_core as cursive;
    /// use cursive::{Printer, Vec2, View, XY};
    ///
    /// pub struct CustomView {
    ///     word: String,
    /// }
    ///
    /// impl CustomView {
    ///     pub fn new() -> Self {
    ///         Self {
    ///             word: String::from("Eh, tu connais Rust?"),
    ///         }
    ///     }
    /// }
    ///
    /// impl View for CustomView {
    ///     fn draw(&self, printer: &Printer<'_, '_>) {
    ///         printer.print(XY::new(0, 0), &self.word);
    ///     }
    /// }
    /// ```
    // TODO: use &mut self? We don't *need* it, but it may make sense.
    // We don't want people to start calling prints in parallel?
    pub fn print<S: Into<Vec2>>(&self, start: S, text: &str) {
        self.print_with_width(start, text, UnicodeWidthStr::width);
    }

    /// Prints some text, using the given callback to compute width.
    ///
    /// Mostly used with [`UnicodeWidthStr::width`].
    /// If you already know the width, you can give it as a constant instead.
    fn print_with_width<S, F>(&self, start: S, text: &str, width: F)
    where
        S: Into<Vec2>,
        F: FnOnce(&str) -> usize,
    {
        // Where we are asked to start printing. Oh boy. It's not that simple.
        let start = start.into();

        // We accept requests between `content_offset` and
        // `content_offset + output_size`.
        if !start.strictly_lt(self.output_size + self.content_offset) {
            return;
        }

        // If start < content_offset, part of the text will not be visible.
        // This is the part of the text that's hidden:
        // (It should always be smaller than the content offset)
        let hidden_part = self.content_offset.saturating_sub(start);
        if hidden_part.y > 0 {
            // Since we are printing a single line, there's nothing we can do.
            return;
        }

        let mut text_width = width(text);

        // If we're waaaay too far left, just give up.
        if hidden_part.x > text_width {
            return;
        }

        let mut text = text;
        let mut start = start;

        if hidden_part.x > 0 {
            // We have to drop hidden_part.x width from the start of the string.
            // prefix() may be too short if there's a double-width character.
            // So instead, keep the suffix and drop the prefix.

            // TODO: use a different prefix method that is *at least* the width
            // (and not *at most*)
            let tail = suffix(text.graphemes(true), text_width - hidden_part.x, "");
            let skipped_len = text.len() - tail.length;
            let skipped_width = text_width - tail.width;
            assert_eq!(text[..skipped_len].width(), skipped_width);

            // This should be equal most of the time, except when there's a double
            // character preventing us from splitting perfectly.
            assert!(skipped_width >= hidden_part.x);

            // Drop part of the text, and move the cursor correspondingly.
            text = &text[skipped_len..];
            start = start + (skipped_width, 0);
            text_width -= skipped_width;
        }

        assert!(start.fits(self.content_offset));

        // What we did before should guarantee that this won't overflow.
        start = start - self.content_offset;

        // Do we have enough room for the entire line?
        let room = self.output_size.x - start.x;

        if room < text_width {
            // Drop the end of the text if it's too long
            // We want the number of CHARACTERS, not bytes.
            // (Actually we want the "width" of the string, see unicode-width)
            let prefix_len = prefix(text.graphemes(true), room, "").length;
            text = &text[..prefix_len];
            assert!(text.width() <= room);
        }

        let start = start + self.offset;
        self.buffer
            .write()
            .print_at(start, text, self.current_style());
    }

    /// Prints a vertical line using the given character.
    pub fn print_vline<T: Into<Vec2>>(&self, start: T, height: usize, c: &str) {
        let start = start.into();

        // Here again, we can abort if we're trying to print too far right or
        // too low.
        if !start.strictly_lt(self.output_size + self.content_offset) {
            return;
        }

        // hidden_part describes how far to the top left of the viewport we are.
        let hidden_part = self.content_offset.saturating_sub(start);
        if hidden_part.x > 0 || hidden_part.y >= height {
            // We're printing a single column, so we can't do much here.
            return;
        }

        // Skip `hidden_part`
        let start = start + hidden_part;
        assert!(start.fits(self.content_offset));

        let height = height - hidden_part.y;

        // What we did before ensures this won't overflow.
        let start = start - self.content_offset;

        // Don't go overboard
        let height = min(height, self.output_size.y - start.y);

        let start = start + self.offset;
        for y in 0..height {
            self.buffer
                .write()
                .print_at(start + (0, y), c, self.current_style());
        }
    }

    /// Calls a closure on the output window for this printer.
    ///
    /// The window is the part of [`Self::output_window`] that lies in the
    /// backend buffer. It can be empty, for instance for a view placed
    /// entirely off-screen, so `f` should rely on [`Window::size`] rather than
    /// assume any cell is available.
    pub fn on_window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Window<'_>) -> R,
    {
        let mut buffer = self.buffer.write();
        let mut window = buffer.clipped_window(self.offset, self.output_size);
        f(&mut window)
    }

    /// Prints a line using the given character.
    pub fn print_line<T: Into<Vec2>>(
        &self,
        orientation: Orientation,
        start: T,
        length: usize,
        c: &str,
    ) {
        match orientation {
            Orientation::Vertical => self.print_vline(start, length, c),
            Orientation::Horizontal => self.print_hline(start, length, c),
        }
    }

    /// Fills a rectangle using the given character.
    pub fn print_rect(&self, rect: Rect, c: &str) {
        for y in rect.top()..=rect.bottom() {
            self.print_hline((rect.left(), y), rect.width(), c);
        }
    }

    /// Prints a horizontal line using the given character.
    pub fn print_hline<T: Into<Vec2>>(&self, start: T, width: usize, c: &str) {
        let start = start.into();

        // Nothing to be done if the start if too far to the bottom/right
        if !start.strictly_lt(self.output_size + self.content_offset) {
            return;
        }

        let hidden_part = self.content_offset.saturating_sub(start);
        if hidden_part.y > 0 || hidden_part.x >= width {
            // We're printing a single line, so we can't do much here.
            return;
        }

        // Skip `hidden_part`
        let start = start + hidden_part;
        assert!(start.fits(self.content_offset));

        let width = width - hidden_part.x;

        // Don't go too far
        let start = start - self.content_offset;

        let c_width = c.width();

        // Don't write too much if we're close to the end
        let repetitions = min(width, self.output_size.x - start.x) / c_width;

        let mut start = start + self.offset;
        let mut buffer = self.buffer.write();
        let style = self.current_style();
        for _ in 0..repetitions {
            buffer.print_at(start, c, style);
            start.x += c_width;
        }
    }

    /// Prints `text` in the `width` cells starting at `start`, aligned as given.
    ///
    /// The width of the text is its display width (see [`UnicodeWidthStr`]).
    /// Cells of the span that the text does not cover are left untouched.
    ///
    /// Text wider than `width` is cropped to the span, and never spills out of
    /// it: with [`HAlign::Left`] and [`HAlign::Center`] the start of the text is
    /// kept, with [`HAlign::Right`] the end is (so a right-aligned number keeps
    /// its last digits). A double-width character is never split: if it does not
    /// fit, the leftover cell is left untouched.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::align::HAlign;
    /// # fn with_printer(printer: &Printer) {
    /// // A label on the left, and its value on the right of a 20-cell row.
    /// printer.print_aligned((0, 0), 20, HAlign::Left, "CPU");
    /// printer.print_aligned((0, 0), 20, HAlign::Right, "42%");
    /// # }
    /// ```
    pub fn print_aligned<T: Into<Vec2>>(&self, start: T, width: usize, align: HAlign, text: &str) {
        let start = start.into();
        let pieces = [(text, text.width(), None)];
        self.print_pieces_aligned(start, width, align, &pieces);
    }

    /// Prints styled `text` in the `width` cells starting at `start`, aligned as given.
    ///
    /// Like [`print_aligned`](Self::print_aligned), but each span of the text
    /// keeps its own style. When the text is too wide, the cropped parts are
    /// chosen the same way, across spans.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::align::HAlign;
    /// # use cursive_core::style::BaseColor;
    /// # use cursive_core::utils::markup::StyledString;
    /// # fn with_printer(printer: &Printer) {
    /// let mut text = StyledString::plain("42");
    /// text.append(StyledString::styled("%", BaseColor::Red.dark()));
    /// printer.print_styled_aligned((0, 0), 10, HAlign::Right, &text);
    /// # }
    /// ```
    pub fn print_styled_aligned<T, S>(&self, start: T, width: usize, align: HAlign, text: S)
    where
        T: Into<Vec2>,
        S: crate::utils::span::SpannedText<S = IndexedSpan<Style>>,
    {
        let start = start.into();
        let pieces: Vec<_> = text
            .spans()
            .iter()
            .map(|span| {
                let span = span.resolve(text.source());
                (span.content, span.width, Some(*span.attr))
            })
            .collect();
        self.print_pieces_aligned(start, width, align, &pieces);
    }

    /// Prints consecutive pieces of text (content, width, style) aligned in a span.
    ///
    /// A piece without a style uses the current one.
    fn print_pieces_aligned(
        &self,
        start: Vec2,
        width: usize,
        align: HAlign,
        pieces: &[(&str, usize, Option<Style>)],
    ) {
        let total: usize = pieces.iter().map(|&(_, w, _)| w).sum();
        let base = start.x + align.get_offset(total, width);

        // Cells of the text that are kept: `lo..hi`.
        // Right-aligned text keeps its end, other alignments keep the start.
        let lo = match align {
            HAlign::Right => total.saturating_sub(width),
            _ => 0,
        };
        let hi = lo + width;

        let mut at = 0;
        for &(content, piece_width, style) in pieces {
            let (piece_start, piece_end) = (at, at + piece_width);
            at = piece_end;
            if piece_end <= lo || piece_start >= hi {
                continue;
            }

            let mut content = content;
            let mut piece_start = piece_start;
            let mut piece_width = piece_width;
            if piece_start < lo {
                // Drop the cells before `lo`, but never split a character.
                let tail = suffix(content.graphemes(true), piece_end - lo, "");
                content = &content[content.len() - tail.length..];
                piece_start = piece_end - tail.width;
                piece_width = tail.width;
            }
            if piece_start + piece_width > hi {
                // Drop the cells after `hi`.
                let head = prefix(content.graphemes(true), hi - piece_start, "");
                content = &content[..head.length];
                piece_width = head.width;
            }
            if piece_width == 0 {
                continue;
            }

            let position = (base + piece_start - lo, start.y);
            match style {
                Some(style) => self.with_style(style, |printer| {
                    printer.print_with_width(position, content, |_| piece_width);
                }),
                None => self.print_with_width(position, content, |_| piece_width),
            }
        }
    }

    /// Prints a horizontal bar, filled from the left by a fraction.
    ///
    /// The bar occupies `width` cells starting at `start`. It is filled
    /// proportionally to `fill` (clamped to `0.0..=1.0`), with a precision of
    /// an eighth of a cell, using the characters `▏▎▍▌▋▊▉█`.
    ///
    /// Cells that are entirely empty are not printed at all, so you can draw a
    /// track underneath first. The bar uses the current style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # fn with_printer(printer: &Printer) {
    /// // A track, then a bar filled at 40%.
    /// printer.print_hline((0, 0), 10, "░");
    /// printer.print_hbar((0, 0), 10, 0.4);
    /// # }
    /// ```
    pub fn print_hbar<T: Into<Vec2>>(&self, start: T, width: usize, fill: f32) {
        self.print_hbar_with(start, width, fill, |_| ColorStyle::inherit_parent());
    }

    /// Prints a horizontal bar, with a style for each cell.
    ///
    /// Like [`print_hbar`](Self::print_hbar), but `f` gives the style of each
    /// filled cell. It is called with the index of the cell, counted from the
    /// left end of the bar.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::{BaseColor, ColorStyle};
    /// # fn with_printer(printer: &Printer) {
    /// // The first half is green, the rest is red.
    /// printer.print_hbar_with((0, 0), 10, 0.8, |i| {
    ///     if i < 5 {
    ///         BaseColor::Green
    ///     } else {
    ///         BaseColor::Red
    ///     }
    /// });
    /// # }
    /// ```
    pub fn print_hbar_with<T, S, F>(&self, start: T, width: usize, fill: f32, f: F)
    where
        T: Into<Vec2>,
        S: Into<ColorStyle>,
        F: Fn(usize) -> S,
    {
        self.print_hbar_eighths(start, width, bar_eighths(fill, width), f);
    }

    /// Prints a horizontal bar of `width` cells, filled for the given number of eighths of a cell.
    ///
    /// This is what [`print_hbar_with`](Self::print_hbar_with) uses, for callers that have an
    /// exact count: more than `width * 8` makes a full bar.
    pub(crate) fn print_hbar_eighths<T, S, F>(&self, start: T, width: usize, eighths: usize, f: F)
    where
        T: Into<Vec2>,
        S: Into<ColorStyle>,
        F: Fn(usize) -> S,
    {
        let start = start.into();
        for i in 0..width {
            let level = eighths.saturating_sub(i * 8).min(8);
            if level == 0 {
                break;
            }
            self.with_color(f(i).into(), |printer| {
                printer.print((start.x + i, start.y), H_BLOCKS[level - 1]);
            });
        }
    }

    /// Prints a vertical bar, filled from the bottom by a fraction.
    ///
    /// The bar occupies `height` cells, and `start` is its top cell (like
    /// [`print_vline`](Self::print_vline)). It is filled proportionally to
    /// `fill` (clamped to `0.0..=1.0`), with a precision of an eighth of a
    /// cell, using the characters `▁▂▃▄▅▆▇█`.
    ///
    /// Cells that are entirely empty are not printed at all, so you can draw a
    /// track underneath first. The bar uses the current style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # fn with_printer(printer: &Printer) {
    /// // A 5-cell tall bar, filled at 60%.
    /// printer.print_vbar((0, 0), 5, 0.6);
    /// # }
    /// ```
    pub fn print_vbar<T: Into<Vec2>>(&self, start: T, height: usize, fill: f32) {
        self.print_vbar_with(start, height, fill, |_| ColorStyle::inherit_parent());
    }

    /// Prints a vertical bar, with a style for each cell.
    ///
    /// Like [`print_vbar`](Self::print_vbar), but `f` gives the style of each
    /// filled cell. It is called with the index of the cell, counted from the
    /// bottom of the bar.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::Rgb;
    /// # fn with_printer(printer: &Printer) {
    /// // A gradient, getting brighter towards the top.
    /// printer.print_vbar_with((0, 0), 8, 1.0, |i| Rgb::new(i as u8 * 30, 0, 0));
    /// # }
    /// ```
    pub fn print_vbar_with<T, S, F>(&self, start: T, height: usize, fill: f32, f: F)
    where
        T: Into<Vec2>,
        S: Into<ColorStyle>,
        F: Fn(usize) -> S,
    {
        const BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        let start = start.into();
        let eighths = bar_eighths(fill, height);
        for i in 0..height {
            let level = eighths.saturating_sub(i * 8).min(8);
            if level == 0 {
                break;
            }
            self.with_color(f(i).into(), |printer| {
                printer.print((start.x, start.y + height - 1 - i), BLOCKS[level - 1]);
            });
        }
    }

    /// Prints a horizontal bar colored by a [`Fill`].
    ///
    /// Like [`print_hbar`](Self::print_hbar), but the front color of each
    /// cell comes from `fill`, which sees the bar as its area: `width` cells
    /// by one row, with `ratio` as its value. Only the front color is set;
    /// the background is left as it is.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::BaseColor;
    /// # use cursive_core::style::chart::Fill;
    /// # fn with_printer(printer: &Printer) {
    /// // Green, then yellow, then red as the bar fills up.
    /// let fill = Fill::steps([
    ///     (0.0, BaseColor::Green.dark()),
    ///     (0.7, BaseColor::Yellow.dark()),
    ///     (0.85, BaseColor::Red.dark()),
    /// ]);
    /// printer.print_hbar_filled((0, 0), 10, 0.8, &fill);
    /// # }
    /// ```
    pub fn print_hbar_filled<T: Into<Vec2>>(
        &self,
        start: T,
        width: usize,
        ratio: f32,
        fill: &Fill,
    ) {
        let fill = fill.resolve(self);
        self.print_hbar_with(start, width, ratio, |column| {
            ColorStyle::front(fill.color_at(FillPos::cell(column, 0, (width, 1), ratio)))
        });
    }

    /// Prints a vertical bar colored by a [`Fill`].
    ///
    /// Like [`print_vbar`](Self::print_vbar), but the front color of each
    /// cell comes from `fill`, which sees the bar as its area: one column by
    /// `height` cells, with `ratio` as its value. The rows are counted from
    /// the bottom. Only the front color is set; the background is left as it
    /// is.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::Rgb;
    /// # use cursive_core::style::chart::Fill;
    /// # fn with_printer(printer: &Printer) {
    /// // Darker at the bottom, brighter at the top.
    /// let fill = Fill::vertical([Rgb::new(0, 60, 0), Rgb::new(0, 255, 0)]);
    /// printer.print_vbar_filled((0, 0), 8, 1.0, &fill);
    /// # }
    /// ```
    pub fn print_vbar_filled<T: Into<Vec2>>(
        &self,
        start: T,
        height: usize,
        ratio: f32,
        fill: &Fill,
    ) {
        let fill = fill.resolve(self);
        self.print_vbar_with(start, height, ratio, |row| {
            ColorStyle::front(fill.color_at(FillPos::cell(0, row, (1, height), ratio)))
        });
    }

    /// Prints some text colored by a [`Fill`], going horizontally across the text.
    ///
    /// The front color of each grapheme comes from `fill`. The text is the
    /// fill's area: the position is 0 at the first cell and 1 at the last
    /// one, and the value is the same as the horizontal position. A wide
    /// character is colored from the position of its first cell. Other
    /// attributes of the current style, like effects and the background, are
    /// kept.
    ///
    /// The text must be a single line.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::Rgb;
    /// # use cursive_core::style::chart::Fill;
    /// # fn with_printer(printer: &Printer) {
    /// let fill = Fill::horizontal([Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)]);
    /// printer.print_filled((0, 0), "From red to blue", &fill);
    /// # }
    /// ```
    pub fn print_filled<T: Into<Vec2>>(&self, start: T, text: &str, fill: &Fill) {
        let start = start.into();
        let fill = fill.resolve(self);

        let width: usize = text.graphemes(true).map(|g| g.width()).sum();
        let mut x = 0;
        for grapheme in text.graphemes(true) {
            let w = grapheme.width();
            if w == 0 {
                continue;
            }
            let pos = FillPos::cell(x, 0, (width, 1), 0.0);
            let pos = FillPos {
                value: pos.x,
                ..pos
            };
            self.with_color(ColorStyle::front(fill.color_at(pos)), |printer| {
                printer.print((start.x + x, start.y), grapheme);
            });
            x += w;
        }
    }

    /// Resolves a color using the printer's theme.
    ///
    /// Palette colors and custom [`ColorKey`](crate::style::ColorKey)s are
    /// looked up in the theme's palette (custom keys use their fallback when
    /// the palette has no entry), and `ColorType::InheritParent` gives the
    /// printer's current front color.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style::PaletteColor;
    /// # fn with_printer(printer: &Printer) {
    /// let color = printer.resolve_color(PaletteColor::Highlight);
    /// # let _ = color;
    /// # }
    /// ```
    pub fn resolve_color(&self, color: impl Into<ColorType>) -> Color {
        color
            .into()
            .resolve(&self.theme.palette, self.current_color().front)
    }

    /// Returns the color currently used by the printer.
    pub fn current_color(&self) -> ColorPair {
        self.current_style().color
    }

    /// Returns the style currently used by the printer.
    pub fn current_style(&self) -> ConcreteStyle {
        self.current_style.get()
    }

    /// Sets the color used by this printer.
    pub fn set_color(&mut self, color: ColorStyle) {
        let style = self.current_style.get();
        let color = color.resolve(&self.theme.palette, style.color);
        let style = style.with(|s| s.color = color);
        self.current_style.set(style);
    }

    /// Sets the current style used by the printer.
    pub fn set_style<T>(&mut self, style: T)
    where
        T: Into<StyleType>,
    {
        let style = style.into();
        // eprintln!("Setting style for subprinter to {style:?}");

        let old = self.current_style();
        let style = style
            .resolve(&self.theme.palette)
            .resolve(&self.theme.palette, old);

        // eprintln!("Style resolved to {style:?}");

        self.current_style.set(style);
    }

    /// Deactivate the given effect for this printer.
    pub fn unset_effect(&mut self, effect: Effect) {
        let mut style = self.current_style();
        style.effects.remove(effect);
        self.current_style.set(style);
    }

    /// Active the given effect for this printer.
    pub fn set_effect(&mut self, effect: Effect) {
        let mut style = self.current_style();
        style.effects.insert(effect);
        self.current_style.set(style);
    }

    /// Call the given closure with a colored printer,
    /// that will apply the given color on prints.
    ///
    /// Does not change the current set of active effects (bold/underline/...).
    pub fn with_color<F>(&self, c: ColorStyle, f: F)
    where
        F: FnOnce(&Printer),
    {
        let sub = self.clone().with(|sub| sub.set_color(c));

        f(&sub);
    }

    /// Call the given closure with a styled printer,
    /// that will apply the given style on prints.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # use cursive_core::style;
    /// # fn with_printer(printer: &Printer) {
    /// printer.with_style(style::PaletteStyle::Highlight, |printer| {
    ///     printer.print((0, 0), "This text is highlighted!");
    /// });
    /// # }
    /// ```
    pub fn with_style<F, T>(&self, style: T, f: F)
    where
        F: FnOnce(&Printer),
        T: Into<StyleType>,
    {
        let sub = self.clone().with(|sub| sub.set_style(style));

        f(&sub);
    }

    /// Call the given closure with a modified printer
    /// that will apply the given effect on prints.
    pub fn with_effect<F>(&self, effect: Effect, f: F)
    where
        F: FnOnce(&Printer),
    {
        let sub = self.clone().with(|sub| sub.set_effect(effect));
        f(&sub);
    }

    /// Call the given closure with a modified printer
    /// that will apply the given theme on prints.
    pub fn with_theme<F>(&self, theme: &Theme, f: F)
    where
        F: FnOnce(&Printer),
    {
        f(&self.theme(theme));
    }

    /// Create a new sub-printer with the given theme.
    pub fn theme<'c>(&self, theme: &'c Theme) -> Printer<'c, 'b>
    where
        'a: 'c,
    {
        Printer {
            theme,
            ..self.clone()
        }
    }

    /// Call the given closure with a modified printer
    /// that will apply each given effect on prints.
    ///
    /// Note that this does not unset any active effect.
    pub fn with_effects<F>(&self, effects: EnumSet<Effect>, f: F)
    where
        F: FnOnce(&Printer),
    {
        match effects.iter().next() {
            None => f(self),
            Some(effect) => {
                let mut effects = effects;
                effects.remove(effect);

                self.with_effect(effect, |s| s.with_effects(effects, f));
            }
        }
    }

    /// Prints a rectangular box.
    ///
    /// If `invert` is `true`, and the theme uses `Outset` borders, then the
    /// box will use an "inset" style instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::Printer;
    /// # fn with_printer(printer: &Printer) {
    /// printer.print_box((0, 0), (6, 4), false);
    /// # }
    /// ```
    pub fn print_box<T: Into<Vec2>, S: Into<Vec2>>(&self, start: T, size: S, invert: bool) {
        let start = start.into();
        let size = size.into();

        if size.x < 2 || size.y < 2 {
            return;
        }
        let size = size - (1, 1);

        let Some([top_left, top_right, bottom_left, bottom_right]) = self.theme.borders.corners()
        else {
            return;
        };

        self.with_high_border(invert, |s| {
            s.print(start, top_left);
            s.print(start + size.keep_y(), bottom_left);
            s.print_hline(start + (1, 0), size.x - 1, "─");
            s.print_vline(start + (0, 1), size.y - 1, "│");
        });

        self.with_low_border(invert, |s| {
            s.print(start + size.keep_x(), top_right);
            s.print(start + size, bottom_right);
            s.print_hline(start + (1, 0) + size.keep_y(), size.x - 1, "─");
            s.print_vline(start + (0, 1) + size.keep_x(), size.y - 1, "│");
        });
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `false`,
    ///   use [`PaletteStyle::Tertiary`].
    /// * Otherwise, use [`PaletteStyle::Primary`], with the [`BORDER`] color in front
    ///   ([`BORDER_FOCUSED`](crate::style::BORDER_FOCUSED) when this printer has the focus).
    pub fn with_high_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let style = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if !invert => PaletteStyle::Tertiary.into(),
            _ => self.border_style(),
        };

        self.with_style(style, f);
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `true`,
    ///   use [`PaletteStyle::Tertiary`].
    /// * Otherwise, use [`PaletteStyle::Primary`], with the [`BORDER`] color in front
    ///   ([`BORDER_FOCUSED`](crate::style::BORDER_FOCUSED) when this printer has the focus).
    pub fn with_low_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let style = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if invert => PaletteStyle::Tertiary.into(),
            _ => self.border_style(),
        };

        self.with_style(style, f);
    }

    /// Returns the key of the color to use for borders, if the palette defines one.
    ///
    /// This is [`BORDER_FOCUSED`] for a focused printer when the palette defines it, and
    /// [`BORDER`] otherwise.
    fn border_color(&self) -> Option<&'static ColorKey> {
        let defined = |key: &'static ColorKey| {
            self.theme
                .palette
                .custom(key.name())
                .is_some()
                .then_some(key)
        };
        if self.focused {
            defined(&BORDER_FOCUSED).or_else(|| defined(&BORDER))
        } else {
            defined(&BORDER)
        }
    }

    /// Runs the given function with the border color as the front color, if the palette
    /// defines one ([`BORDER`], or [`BORDER_FOCUSED`] with the focus).
    ///
    /// If the palette does not define it, `f` is called with this printer unchanged.
    /// Other than the front color, the current style is kept.
    pub(crate) fn with_border_color<F>(&self, f: F)
    where
        F: FnOnce(&Printer),
    {
        match self.border_color() {
            Some(key) => self.with_color(ColorStyle::front(key), f),
            None => f(self),
        }
    }

    /// Returns the style to use for borders.
    ///
    /// This is `PaletteStyle::Primary`, with the front color replaced by the `border` entry (or
    /// `border_focused` with the focus) if the palette defines it.
    fn border_style(&self) -> StyleType {
        let Some(key) = self.border_color() else {
            return PaletteStyle::Primary.into();
        };

        let mut style = self.theme.palette[PaletteStyle::Primary];
        style.color.front = key.into();
        style.into()
    }

    /// Apply a selection style and call the given function.
    ///
    /// * If `selection` is `false`, simply uses the current style.
    /// * If `selection` is `true`:
    ///     * If the printer currently has the focus, uses [`PaletteStyle::Highlight`].
    ///     * Otherwise, uses [`PaletteStyle::HighlightInactive`].
    pub fn with_selection<F: FnOnce(&Printer)>(&self, selection: bool, f: F) {
        self.with_style(
            if selection {
                if self.focused {
                    StyleType::highlight()
                } else {
                    StyleType::highlight_inactive()
                }
            } else {
                StyleType::inherit_parent()
            },
            f,
        );
    }

    /// Prints a horizontal delimiter with side border `├` and `┤`.
    pub fn print_hdelim<T>(&self, start: T, len: usize)
    where
        T: Into<Vec2>,
    {
        let start = start.into();
        self.print(start, "├");
        self.print_hline(start + (1, 0), len.saturating_sub(2), "─");
        self.print(start + (len.saturating_sub(1), 0), "┤");
    }

    /// Returns a sub-printer with the given offset.
    ///
    /// It will print in an area slightly to the bottom/right.
    #[must_use]
    pub fn offset<S>(&self, offset: S) -> Self
    where
        S: Into<Vec2>,
    {
        let offset = offset.into();
        self.clone().with(|s| {
            // If we are drawing a part of the content,
            // let's reduce this first.
            let consumed = Vec2::min(s.content_offset, offset);

            let offset = offset - consumed;
            s.content_offset = s.content_offset - consumed;

            s.offset = s.offset + offset;

            s.output_size = s.output_size.saturating_sub(offset);
            s.size = s.size.saturating_sub(offset);

            s.clamp_output_to_buffer();
        })
    }

    /// Returns a new sub-printer inheriting the given focus.
    ///
    /// If `self` is focused and `focused == true`, the child will be focused.
    ///
    /// Otherwise, he will be unfocused.
    #[must_use]
    pub fn focused(&self, focused: bool) -> Self {
        self.clone().with(|s| {
            s.focused &= focused;
        })
    }

    /// Returns a new sub-printer inheriting the given enabled state.
    ///
    /// If `self` is enabled and `enabled == true`, the child will be enabled.
    ///
    /// Otherwise, he will be disabled.
    #[must_use]
    pub fn enabled(&self, enabled: bool) -> Self {
        self.clone().with(|s| s.enabled &= enabled)
    }

    /// Returns a new sub-printer for the given viewport.
    ///
    /// This is a combination of offset + cropped.
    #[must_use]
    pub fn windowed(&self, viewport: Rect) -> Self {
        self.offset(viewport.top_left()).cropped(viewport.size())
    }

    /// Returns a new sub-printer with a cropped area.
    ///
    /// The new printer size will be the minimum of `size` and its current size.\
    /// Any size reduction happens at the bottom-right.
    #[must_use]
    pub fn cropped<S>(&self, size: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.clone().with(|s| {
            let size = size.into();
            s.output_size = Vec2::min(s.output_size, size);
            s.size = Vec2::min(s.size, size);
            s.clamp_output_to_buffer();
        })
    }

    /// Returns a new sub-printer with a cropped area.
    ///
    /// The new printer size will be the minimum of `size` and its current size.\
    /// The view will stay centered.\
    /// Note that if shrinking by an odd number, the view will round to the top-left.
    #[must_use]
    pub fn cropped_centered<S>(&self, size: S) -> Self
    where
        S: Into<Vec2>,
    {
        let size = size.into();
        let borders = self.size.saturating_sub(size);
        let half_borders = borders / 2;

        self.cropped(size - half_borders).offset(half_borders)
    }

    /// Returns a new sub-printer with a shrinked area.
    ///
    /// The printer size will be reduced by the given border from the bottom-right.
    #[must_use]
    pub fn shrinked<S>(&self, borders: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.cropped(self.size.saturating_sub(borders))
    }

    /// Returns a new sub-printer with a shrinked area.
    ///
    /// The printer size will be reduced by the given border, and will stay centered.\
    /// Note that if shrinking by an odd number, the view will round to the top-left.
    #[must_use]
    pub fn shrinked_centered<S>(&self, borders: S) -> Self
    where
        S: Into<Vec2>,
    {
        let borders = borders.into();
        let half_borders = borders / 2;

        self.shrinked(borders - half_borders).offset(half_borders)
    }

    /// Returns a new sub-printer with a content offset.
    ///
    /// This is useful for parent views that only show a subset of their
    /// child, like [`crate::views::ScrollView`].
    #[must_use]
    pub fn content_offset<S>(&self, offset: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.clone().with(|s| {
            s.content_offset = s.content_offset + offset;
        })
    }

    /// Returns a sub-printer with a different inner size.
    ///
    /// This will not change the actual output size, but will appear bigger
    /// (or smaller) to users of this printer.\
    /// Useful to give to children who think they're big, but really aren't.
    #[must_use]
    pub fn inner_size<S>(&self, size: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.clone().with(|s| {
            s.size = size.into();
        })
    }
}

/// The glyphs of a horizontal bar, for a cell filled from the left by 1 to 8 eighths.
pub(crate) const H_BLOCKS: [&str; 8] = ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// Number of eighths of cells to fill in a bar of `cells` cells.
fn bar_eighths(fill: f32, cells: usize) -> usize {
    (fill.clamp(0.0, 1.0) * (cells * 8) as f32).round() as usize
}

#[cfg(test)]
mod aligned_tests {
    use super::*;
    use crate::utils::markup::StyledString;

    const RED: Color = Color::Rgb(255, 0, 0);
    const BLUE: Color = Color::Rgb(0, 0, 255);

    /// A buffer of the given width, filled with `.`, and a theme to draw it.
    fn dotted(width: usize) -> (Theme, RwLock<PrintBuffer>) {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(width, 1));
        let theme = Theme::default();
        let buffer = RwLock::new(buffer);
        Printer::new((width, 1), &theme, &buffer).print_hline((0, 0), width, ".");
        (theme, buffer)
    }

    /// The text of the buffer. A wide character is followed by nothing.
    fn text(buffer: &RwLock<PrintBuffer>, width: usize) -> String {
        let buffer = buffer.read();
        (0..width)
            .map(|x| buffer.cell_text(Vec2::new(x, 0)).unwrap_or("").to_string())
            .collect()
    }

    /// Prints on a 10-cell row after dots, within the cells `2..8`.
    fn aligned(align: HAlign, content: &str) -> String {
        let (theme, buffer) = dotted(10);
        Printer::new((10, 1), &theme, &buffer).print_aligned((2, 0), 6, align, content);
        text(&buffer, 10)
    }

    #[test]
    fn alignments() {
        assert_eq!(aligned(HAlign::Left, "ab"), "..ab......");
        assert_eq!(aligned(HAlign::Right, "ab"), "......ab..");
        assert_eq!(aligned(HAlign::Center, "ab"), "....ab....");
        // The extra cell of an odd margin goes on the right.
        assert_eq!(aligned(HAlign::Center, "abc"), "...abc....");
    }

    #[test]
    fn exact_fit() {
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            assert_eq!(aligned(align, "abcdef"), "..abcdef..");
        }
    }

    #[test]
    fn overflow() {
        assert_eq!(aligned(HAlign::Left, "abcdefgh"), "..abcdef..");
        assert_eq!(aligned(HAlign::Center, "abcdefgh"), "..abcdef..");
        // The end is kept, so the last digits are visible.
        assert_eq!(aligned(HAlign::Right, "abcdefgh"), "..cdefgh..");
    }

    #[test]
    fn wide_characters() {
        assert_eq!(aligned(HAlign::Left, "日本"), "..日本....");
        assert_eq!(aligned(HAlign::Right, "日本"), "....日本..");
        assert_eq!(aligned(HAlign::Center, "日本"), "...日本...");
        // Display width is used, not chars or bytes.
        assert_eq!(aligned(HAlign::Right, "é"), ".......é..");
    }

    #[test]
    fn wide_characters_are_not_split() {
        // 3 wide characters in 6 cells fit exactly.
        assert_eq!(aligned(HAlign::Left, "日本語"), "..日本語..");
        // 4 wide characters: the span holds 3.
        assert_eq!(aligned(HAlign::Left, "日本語字"), "..日本語..");
        assert_eq!(aligned(HAlign::Right, "日本語字"), "..本語字..");
    }

    #[test]
    fn wide_character_cut_leaves_a_cell_untouched() {
        let (theme, buffer) = dotted(10);
        let printer = Printer::new((10, 1), &theme, &buffer);
        printer.print_aligned((2, 0), 6, HAlign::Left, "abcde日");
        // "abcde" takes 5 cells, the wide one needs 2: the 6th cell stays a dot.
        assert_eq!(text(&buffer, 10), "..abcde...");

        let (theme, buffer) = dotted(10);
        let printer = Printer::new((10, 1), &theme, &buffer);
        printer.print_aligned((2, 0), 6, HAlign::Right, "日abcde");
        // The wide character is dropped, and its cell is the one left untouched.
        assert_eq!(text(&buffer, 10), "...abcde..");
    }

    #[test]
    fn empty_and_zero_width() {
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            assert_eq!(aligned(align, ""), "..........");
        }
        let (theme, buffer) = dotted(10);
        let printer = Printer::new((10, 1), &theme, &buffer);
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            printer.print_aligned((2, 0), 0, align, "abc");
            printer.print_styled_aligned((2, 0), 0, align, &StyledString::plain("abc"));
        }
        assert_eq!(text(&buffer, 10), "..........");
    }

    #[test]
    fn respects_printer_clipping() {
        let (theme, buffer) = dotted(10);
        let printer = Printer::new((10, 1), &theme, &buffer);
        // The printer only shows 4 cells, but the span is larger.
        printer
            .cropped((4, 1))
            .print_aligned((0, 0), 10, HAlign::Left, "abcdefgh");
        assert_eq!(text(&buffer, 10), "abcd......");
    }

    fn styled(
        align: HAlign,
        width: usize,
        content: &StyledString,
    ) -> (String, RwLock<PrintBuffer>) {
        let (theme, buffer) = dotted(10);
        Printer::new((10, 1), &theme, &buffer).print_styled_aligned((2, 0), width, align, content);
        (text(&buffer, 10), buffer)
    }

    fn fronts(buffer: &RwLock<PrintBuffer>, xs: std::ops::Range<usize>) -> Vec<Color> {
        let buffer = buffer.read();
        xs.map(|x| buffer.cell_style(Vec2::new(x, 0)).unwrap().color.front)
            .collect()
    }

    fn two_colors() -> StyledString {
        let mut content = StyledString::styled("ab", RED);
        content.append(StyledString::styled("cd", BLUE));
        content
    }

    #[test]
    fn styled_alignments_keep_styles() {
        let (out, buffer) = styled(HAlign::Left, 6, &two_colors());
        assert_eq!(out, "..abcd....");
        assert_eq!(fronts(&buffer, 2..6), [RED, RED, BLUE, BLUE]);

        let (out, buffer) = styled(HAlign::Right, 6, &two_colors());
        assert_eq!(out, "....abcd..");
        assert_eq!(fronts(&buffer, 4..8), [RED, RED, BLUE, BLUE]);

        let (out, buffer) = styled(HAlign::Center, 6, &two_colors());
        assert_eq!(out, "...abcd...");
        assert_eq!(fronts(&buffer, 3..7), [RED, RED, BLUE, BLUE]);
    }

    #[test]
    fn styled_overflow() {
        let (out, buffer) = styled(HAlign::Left, 3, &two_colors());
        assert_eq!(out, "..abc.....");
        assert_eq!(fronts(&buffer, 2..5), [RED, RED, BLUE]);

        let (out, buffer) = styled(HAlign::Center, 3, &two_colors());
        assert_eq!(out, "..abc.....");
        assert_eq!(fronts(&buffer, 2..5), [RED, RED, BLUE]);

        let (out, buffer) = styled(HAlign::Right, 3, &two_colors());
        assert_eq!(out, "..bcd.....");
        assert_eq!(fronts(&buffer, 2..5), [RED, BLUE, BLUE]);

        // A span is dropped entirely.
        let (out, buffer) = styled(HAlign::Right, 2, &two_colors());
        assert_eq!(out, "..cd......");
        assert_eq!(fronts(&buffer, 2..4), [BLUE, BLUE]);
    }

    #[test]
    fn styled_wide_characters() {
        let mut content = StyledString::styled("日", RED);
        content.append(StyledString::styled("本", BLUE));
        let (out, buffer) = styled(HAlign::Right, 6, &content);
        assert_eq!(out, "....日本..");
        assert_eq!(fronts(&buffer, 4..5), [RED]);
        assert_eq!(fronts(&buffer, 6..7), [BLUE]);

        // Cropping at the end of a span does not split the character.
        let (out, _) = styled(HAlign::Left, 3, &content);
        assert_eq!(out, "..日......");
        let (out, _) = styled(HAlign::Right, 3, &content);
        assert_eq!(out, "...本.....");
    }

    #[test]
    fn styled_empty() {
        let (out, _) = styled(HAlign::Right, 6, &StyledString::new());
        assert_eq!(out, "..........");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::View;
    use crate::views::FixedLayout;

    /// A view that draws through `Printer::on_window`, like `GradientView`.
    struct WindowUser;
    impl crate::View for WindowUser {
        fn draw(&self, printer: &Printer) {
            printer.on_window(|_| {});
        }
    }

    fn buffer_10x10() -> (Theme, RwLock<PrintBuffer>) {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(10, 10));
        (Theme::default(), RwLock::new(buffer))
    }

    #[test]
    fn offset_beyond_buffer_is_clamped_into_buffer() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        // What a positioned container hands to a fully off-screen child.
        let child = printer.offset((50, 50)).cropped((5, 5));
        assert!(
            child
                .output_window()
                .bottom_right()
                .fits_in(Vec2::new(10, 10)),
            "output window {:?} escapes the 10x10 buffer",
            child.output_window()
        );
    }

    #[test]
    fn clamped_printer_runs_on_window_on_empty_viewport() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        let child = printer.offset((50, 50)).cropped((5, 5));
        let mut called = false;
        child.on_window(|window| {
            called = true;
            // The window collapses to a zero-size rect pinned on the
            // buffer edge (the rect's inclusive `size()` still reports the
            // single pinned point, but the window itself is empty).
            assert_eq!(window.viewport(), Rect::from_size((10, 10), (0, 0)));
            assert_eq!(window.size(), Vec2::zero());
        });
        assert!(called, "closure should run on the clamped (empty) window");
    }

    #[test]
    fn partially_visible_child_keeps_visible_part() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        let child = printer.offset((8, 8)).cropped((5, 5));
        assert_eq!(child.output_window(), Rect::from_size((8, 8), (2, 2)));
    }

    #[test]
    fn clamp_leaves_inner_size_alone() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer).inner_size((100, 100));
        let child = printer.offset((50, 50));
        // `offset` itself shrinks `size` by its long-standing rule (100-50);
        // the buffer clamp must not touch it any further.
        assert_eq!(child.size, Vec2::new(50, 50));
    }

    #[test]
    fn offscreen_fixed_layout_child_does_not_panic() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        let layout = FixedLayout::new().child(Rect::from_size((50, 50), (5, 5)), WindowUser);
        layout.draw(&printer);
    }

    #[test]
    fn on_window_clips_to_a_smaller_buffer() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        // The output window no longer fits once the buffer shrinks.
        buffer.write().resize(Vec2::new(5, 5));

        let size = printer.on_window(|window| window.size());
        assert_eq!(size, Vec2::new(5, 5));
    }

    #[test]
    fn on_window_runs_on_empty_window_outside_buffer() {
        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        let child = printer.offset((8, 8));
        buffer.write().resize(Vec2::new(5, 5));

        let size = child.on_window(|window| {
            assert!(window.style_at_mut((0, 0)).is_none());
            assert_eq!(window.rows().count(), 0);
            window.size()
        });
        assert_eq!(size, Vec2::zero());
    }

    #[test]
    fn offscreen_gradient_view_leaves_buffer_alone() {
        use crate::style::gradient::{Angled, Linear};
        use crate::views::{DummyView, GradientView};

        let gradient = || {
            GradientView::new(
                DummyView::new(),
                Angled {
                    angle_rad: 0.0,
                    gradient: Linear::black_to_white(),
                },
            )
        };

        let styles = |buffer: &RwLock<PrintBuffer>| {
            let buffer = buffer.read();
            (0..100)
                .map(|i| buffer.cell_style(Vec2::new(i % 10, i / 10)))
                .collect::<Vec<_>>()
        };

        let (theme, buffer) = buffer_10x10();
        let printer = Printer::new((10, 10), &theme, &buffer);
        printer.clear();
        let before = styles(&buffer);

        // Off-screen past the bottom-right corner, and past the right edge.
        let layout = FixedLayout::new()
            .child(Rect::from_size((50, 50), (5, 5)), gradient())
            .child(Rect::from_size((50, 3), (5, 5)), gradient());
        layout.draw(&printer);

        assert_eq!(styles(&buffer), before);
    }

    /// Draws a 6x4 box with the given theme.
    fn draw_box(theme: &Theme, invert: bool) -> RwLock<PrintBuffer> {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(6, 4));
        let buffer = RwLock::new(buffer);
        Printer::new((6, 4), theme, &buffer).print_box((0, 0), (6, 4), invert);
        buffer
    }

    fn text_at(buffer: &RwLock<PrintBuffer>, x: usize, y: usize) -> String {
        buffer
            .read()
            .cell_text(Vec2::new(x, y))
            .unwrap_or("")
            .to_string()
    }

    fn front_at(buffer: &RwLock<PrintBuffer>, x: usize, y: usize) -> crate::style::Color {
        buffer
            .read()
            .cell_style(Vec2::new(x, y))
            .unwrap()
            .color
            .front
    }

    fn corners(buffer: &RwLock<PrintBuffer>) -> [String; 4] {
        [(0, 0), (5, 0), (0, 3), (5, 3)].map(|(x, y)| text_at(buffer, x, y))
    }

    fn test_theme(borders: BorderStyle) -> Theme {
        let mut theme = Theme::retro();
        theme.borders = borders;
        theme.palette[PaletteColor::Primary] = crate::style::Color::Rgb(10, 20, 30);
        theme.palette[PaletteColor::Tertiary] = crate::style::Color::Rgb(40, 50, 60);
        theme
    }

    #[test]
    fn box_corners_follow_border_style() {
        for (borders, expected) in [
            (BorderStyle::Simple, ["┌", "┐", "└", "┘"]),
            (BorderStyle::Outset, ["┌", "┐", "└", "┘"]),
            (BorderStyle::Rounded, ["╭", "╮", "╰", "╯"]),
        ] {
            let buffer = draw_box(&test_theme(borders), false);
            assert_eq!(corners(&buffer), expected, "{borders:?}");
            // The edges are the same for every style.
            assert_eq!(text_at(&buffer, 2, 0), "─");
            assert_eq!(text_at(&buffer, 0, 1), "│");
            assert_eq!(text_at(&buffer, 5, 2), "│");
        }

        let buffer = draw_box(&test_theme(BorderStyle::None), false);
        assert_eq!(corners(&buffer), ["", "", "", ""]);
    }

    #[test]
    fn border_color_defaults_to_primary() {
        let theme = test_theme(BorderStyle::Rounded);
        let primary = crate::style::Color::Rgb(10, 20, 30);
        let buffer = draw_box(&theme, false);

        for (x, y) in [
            (0, 0),
            (5, 0),
            (0, 3),
            (5, 3),
            (2, 0),
            (0, 1),
            (5, 2),
            (3, 3),
        ] {
            assert_eq!(front_at(&buffer, x, y), primary, "({x}, {y})");
        }

        // Without the entry, the full style is exactly `PaletteStyle::Primary`.
        let mut expected = PrintBuffer::new();
        expected.resize(Vec2::new(6, 4));
        let expected = RwLock::new(expected);
        Printer::new((6, 4), &theme, &expected)
            .with_style(PaletteStyle::Primary, |p| p.print((0, 0), "╭"));
        assert_eq!(
            buffer.read().cell_style(Vec2::new(0, 0)),
            expected.read().cell_style(Vec2::new(0, 0)),
        );
    }

    #[test]
    fn border_color_uses_palette_entry() {
        let mut theme = test_theme(BorderStyle::Rounded);
        let custom = crate::style::Color::Rgb(1, 2, 3);
        theme.palette.set_custom_color(&BORDER, custom);

        let buffer = draw_box(&theme, false);
        for (x, y) in [
            (0, 0),
            (5, 0),
            (0, 3),
            (5, 3),
            (2, 0),
            (0, 1),
            (5, 2),
            (3, 3),
        ] {
            assert_eq!(front_at(&buffer, x, y), custom, "({x}, {y})");
        }

        // Setting it by name works too.
        let mut theme = test_theme(BorderStyle::Simple);
        theme.palette.set_color("border", custom);
        assert_eq!(front_at(&draw_box(&theme, false), 0, 0), custom);

        // The lit side of outset borders keeps using the tertiary color.
        let mut theme = test_theme(BorderStyle::Outset);
        theme.palette.set_custom_color(&BORDER, custom);
        let buffer = draw_box(&theme, false);
        assert_eq!(
            front_at(&buffer, 0, 0),
            crate::style::Color::Rgb(40, 50, 60)
        );
        assert_eq!(front_at(&buffer, 5, 3), custom);
        let buffer = draw_box(&theme, true);
        assert_eq!(front_at(&buffer, 0, 0), custom);
        assert_eq!(
            front_at(&buffer, 5, 3),
            crate::style::Color::Rgb(40, 50, 60)
        );
    }

    #[cfg(feature = "toml")]
    #[test]
    fn border_color_from_toml() {
        let theme = crate::theme::load_toml(
            r##"
            borders = "rounded"

            [colors]
            primary = "#0a141e"
            border = "#010203"
            "##,
        )
        .unwrap();
        assert_eq!(theme.borders, BorderStyle::Rounded);

        let buffer = draw_box(&theme, false);
        assert_eq!(text_at(&buffer, 0, 0), "╭");
        assert_eq!(front_at(&buffer, 0, 0), crate::style::Color::Rgb(1, 2, 3));
        assert_eq!(front_at(&buffer, 5, 3), crate::style::Color::Rgb(1, 2, 3));

        // Without `border`, the primary color is used.
        let theme = crate::theme::load_toml("[colors]\nprimary = \"#0a141e\"\n").unwrap();
        let buffer = draw_box(&theme, false);
        assert_eq!(
            front_at(&buffer, 0, 0),
            crate::style::Color::Rgb(10, 20, 30)
        );
    }

    fn bar_buffer(size: (usize, usize)) -> RwLock<PrintBuffer> {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(size.0, size.1));
        RwLock::new(buffer)
    }

    fn row_text(buffer: &RwLock<PrintBuffer>, y: usize, width: usize) -> String {
        (0..width)
            .map(|x| {
                let text = text_at(buffer, x, y);
                if text.is_empty() || text == " " {
                    ".".to_string()
                } else {
                    text
                }
            })
            .collect()
    }

    fn column_text(buffer: &RwLock<PrintBuffer>, x: usize, height: usize) -> String {
        (0..height)
            .map(|y| {
                let text = text_at(buffer, x, y);
                if text.is_empty() || text == " " {
                    ".".to_string()
                } else {
                    text
                }
            })
            .collect()
    }

    #[test]
    fn hbar_fills() {
        let theme = Theme::default();
        let hbar = |fill: f32| {
            let buffer = bar_buffer((6, 1));
            Printer::new((6, 1), &theme, &buffer).print_hbar((1, 0), 4, fill);
            row_text(&buffer, 0, 6)
        };
        assert_eq!(hbar(1.0), ".████.");
        assert_eq!(hbar(0.0), "......");
        // 4 cells * 8 * 0.4375 = 14 eighths: one full cell and 6 eighths.
        assert_eq!(hbar(0.4375), ".█▊...");
        assert_eq!(hbar(0.03125), ".▏....");
        assert_eq!(hbar(7.0), ".████.");
        assert_eq!(hbar(-3.0), "......");
    }

    #[test]
    fn vbar_fills() {
        let theme = Theme::default();
        let vbar = |fill: f32| {
            let buffer = bar_buffer((1, 6));
            Printer::new((1, 6), &theme, &buffer).print_vbar((0, 1), 4, fill);
            column_text(&buffer, 0, 6)
        };
        assert_eq!(vbar(1.0), ".████.");
        assert_eq!(vbar(0.0), "......");
        // 14 eighths: the bottom cell is full, the one above has 6.
        assert_eq!(vbar(0.4375), "...▆█.");
        assert_eq!(vbar(0.03125), "....▁.");
        assert_eq!(vbar(7.0), ".████.");
        assert_eq!(vbar(-3.0), "......");
    }

    #[test]
    fn bars_pass_cell_index_from_origin() {
        let theme = Theme::default();
        let color = |i: usize| crate::style::Color::Rgb(i as u8, 0, 0);

        let buffer = bar_buffer((4, 1));
        Printer::new((4, 1), &theme, &buffer).print_hbar_with((0, 0), 4, 0.75, |i| color(i + 1));
        for x in 0..3 {
            assert_eq!(front_at(&buffer, x, 0), color(x + 1), "hbar x={x}");
        }
        // Not filled: not printed.
        assert_eq!(text_at(&buffer, 3, 0), "");

        // Index 0 is the bottom cell.
        let buffer = bar_buffer((1, 4));
        Printer::new((1, 4), &theme, &buffer).print_vbar_with((0, 0), 4, 0.75, |i| color(i + 1));
        for i in 0..3 {
            assert_eq!(front_at(&buffer, 0, 3 - i), color(i + 1), "vbar i={i}");
        }
        assert_eq!(text_at(&buffer, 0, 0), "");
    }

    #[test]
    fn plain_bars_keep_current_style() {
        let theme = Theme::default();
        let buffer = bar_buffer((4, 1));
        let red = crate::style::Color::Rgb(200, 0, 0);
        Printer::new((4, 1), &theme, &buffer)
            .with_color(ColorStyle::front(red), |p| p.print_hbar((0, 0), 4, 1.0));
        assert_eq!(front_at(&buffer, 2, 0), red);
    }

    #[test]
    fn resolve_color_uses_theme() {
        use crate::style::{BaseColor, ColorKey};
        static PLAIN: ColorKey = ColorKey::new("test.resolve.plain", BaseColor::Cyan.dark());
        static SET: ColorKey = ColorKey::new("test.resolve.set", BaseColor::Cyan.dark());

        let mut theme = Theme::default();
        let primary = Color::Rgb(10, 20, 30);
        let custom = Color::Rgb(1, 2, 3);
        theme.palette[PaletteColor::Primary] = primary;
        theme.palette.set_custom_color(&SET, custom);

        let buffer = bar_buffer((1, 1));
        let printer = Printer::new((1, 1), &theme, &buffer);
        assert_eq!(printer.resolve_color(PaletteColor::Primary), primary);
        assert_eq!(printer.resolve_color(&SET), custom);
        assert_eq!(printer.resolve_color(&PLAIN), Color::Dark(BaseColor::Cyan));
        assert_eq!(
            printer.resolve_color(Color::Rgb(9, 9, 9)),
            Color::Rgb(9, 9, 9)
        );

        // Inherit-parent gives the current front color.
        let front = Color::Rgb(7, 8, 9);
        printer.with_color(ColorStyle::front(front), |printer| {
            assert_eq!(printer.resolve_color(ColorType::InheritParent), front);
        });
    }

    const BLACK: Color = Color::Rgb(0, 0, 0);
    const WHITE: Color = Color::Rgb(255, 255, 255);

    fn back_at(buffer: &RwLock<PrintBuffer>, x: usize, y: usize) -> Color {
        buffer
            .read()
            .cell_style(Vec2::new(x, y))
            .unwrap()
            .color
            .back
    }

    #[test]
    fn filled_hbar_uses_the_fill_as_its_area() {
        let theme = Theme::default();

        // Horizontal gradient: first and last cells get the end colors.
        let buffer = bar_buffer((6, 1));
        Printer::new((6, 1), &theme, &buffer).print_hbar_filled(
            (1, 0),
            4,
            1.0,
            &Fill::horizontal([BLACK, WHITE]),
        );
        assert_eq!(row_text(&buffer, 0, 6), ".████.");
        assert_eq!(front_at(&buffer, 1, 0), BLACK);
        assert_eq!(front_at(&buffer, 4, 0), WHITE);

        // The value is the ratio, and empty cells are not printed.
        let buffer = bar_buffer((4, 1));
        Printer::new((4, 1), &theme, &buffer).print_hbar_filled(
            (0, 0),
            4,
            0.5,
            &Fill::steps([(0.0, BLACK), (0.5, WHITE)]),
        );
        assert_eq!(row_text(&buffer, 0, 4), "██..");
        assert_eq!(front_at(&buffer, 0, 0), WHITE);
        assert_eq!(front_at(&buffer, 1, 0), WHITE);
        assert_eq!(text_at(&buffer, 2, 0), "");

        // A single cell.
        let buffer = bar_buffer((1, 1));
        Printer::new((1, 1), &theme, &buffer).print_hbar_filled(
            (0, 0),
            1,
            1.0,
            &Fill::horizontal([WHITE, BLACK]),
        );
        assert_eq!(front_at(&buffer, 0, 0), WHITE);

        // A solid fill.
        let buffer = bar_buffer((3, 1));
        Printer::new((3, 1), &theme, &buffer).print_hbar_filled(
            (0, 0),
            3,
            1.0,
            &Fill::solid(WHITE),
        );
        for x in 0..3 {
            assert_eq!(front_at(&buffer, x, 0), WHITE);
        }
    }

    #[test]
    fn filled_hbar_keeps_the_background() {
        let theme = Theme::default();
        let back = Color::Rgb(5, 6, 7);
        let buffer = bar_buffer((2, 1));
        Printer::new((2, 1), &theme, &buffer).with_color(ColorStyle::back(back), |p| {
            p.print_hbar_filled((0, 0), 2, 1.0, &Fill::solid(WHITE))
        });
        assert_eq!(back_at(&buffer, 1, 0), back);
        assert_eq!(front_at(&buffer, 1, 0), WHITE);
    }

    #[test]
    fn filled_vbar_counts_rows_from_the_bottom() {
        let theme = Theme::default();

        let buffer = bar_buffer((1, 6));
        Printer::new((1, 6), &theme, &buffer).print_vbar_filled(
            (0, 1),
            4,
            1.0,
            &Fill::vertical([BLACK, WHITE]),
        );
        assert_eq!(column_text(&buffer, 0, 6), ".████.");
        assert_eq!(front_at(&buffer, 0, 4), BLACK, "bottom");
        assert_eq!(front_at(&buffer, 0, 1), WHITE, "top");

        // Only filled cells are printed, and the value is the ratio.
        let buffer = bar_buffer((1, 4));
        Printer::new((1, 4), &theme, &buffer).print_vbar_filled(
            (0, 0),
            4,
            0.5,
            &Fill::steps([(0.0, BLACK), (0.5, WHITE)]),
        );
        assert_eq!(column_text(&buffer, 0, 4), "..██");
        assert_eq!(front_at(&buffer, 0, 3), WHITE);
        assert_eq!(text_at(&buffer, 0, 1), "");

        // A single cell.
        let buffer = bar_buffer((1, 1));
        Printer::new((1, 1), &theme, &buffer).print_vbar_filled(
            (0, 0),
            1,
            1.0,
            &Fill::vertical([WHITE, BLACK]),
        );
        assert_eq!(front_at(&buffer, 0, 0), WHITE);

        // A solid fill.
        let buffer = bar_buffer((1, 3));
        Printer::new((1, 3), &theme, &buffer).print_vbar_filled(
            (0, 0),
            3,
            1.0,
            &Fill::solid(WHITE),
        );
        for y in 0..3 {
            assert_eq!(front_at(&buffer, 0, y), WHITE);
        }
    }

    #[test]
    fn print_filled_goes_across_the_text() {
        let theme = Theme::default();

        let buffer = bar_buffer((5, 1));
        Printer::new((5, 1), &theme, &buffer).print_filled(
            (1, 0),
            "abc",
            &Fill::horizontal([BLACK, WHITE]),
        );
        assert_eq!(row_text(&buffer, 0, 5), ".abc.");
        assert_eq!(front_at(&buffer, 1, 0), BLACK);
        assert_eq!(front_at(&buffer, 3, 0), WHITE);
        let Color::Rgb(r, _, _) = front_at(&buffer, 2, 0) else {
            panic!("gradients give RGB colors");
        };
        assert!((126..=129).contains(&r));

        // The value is the horizontal position.
        let buffer = bar_buffer((4, 1));
        Printer::new((4, 1), &theme, &buffer).print_filled(
            (0, 0),
            "abcd",
            &Fill::steps([(0.0, BLACK), (0.5, WHITE)]),
        );
        assert_eq!(front_at(&buffer, 1, 0), BLACK); // 1/3
        assert_eq!(front_at(&buffer, 2, 0), WHITE); // 2/3

        // A solid fill.
        let buffer = bar_buffer((3, 1));
        Printer::new((3, 1), &theme, &buffer).print_filled((0, 0), "xyz", &Fill::solid(WHITE));
        for x in 0..3 {
            assert_eq!(front_at(&buffer, x, 0), WHITE);
        }

        // A single cell.
        let buffer = bar_buffer((1, 1));
        Printer::new((1, 1), &theme, &buffer).print_filled(
            (0, 0),
            "x",
            &Fill::horizontal([WHITE, BLACK]),
        );
        assert_eq!(front_at(&buffer, 0, 0), WHITE);
    }

    #[test]
    fn print_filled_wide_characters_and_empty_text() {
        let theme = Theme::default();
        let other = Color::Rgb(1, 1, 1);

        // "a", a two-cell character, then "b": they start on cells 0, 1 and 3.
        let buffer = bar_buffer((4, 1));
        Printer::new((4, 1), &theme, &buffer).print_filled(
            (0, 0),
            "a世b",
            &Fill::steps([(0.0, BLACK), (0.3, WHITE), (0.9, other)]),
        );
        assert_eq!(text_at(&buffer, 0, 0), "a");
        assert_eq!(text_at(&buffer, 1, 0), "世");
        assert_eq!(text_at(&buffer, 3, 0), "b");
        assert_eq!(front_at(&buffer, 0, 0), BLACK);
        // The wide character starts at 1/3.
        assert_eq!(front_at(&buffer, 1, 0), WHITE);
        assert_eq!(front_at(&buffer, 3, 0), other);

        // Nothing to print: nothing happens.
        let buffer = bar_buffer((3, 1));
        Printer::new((3, 1), &theme, &buffer).print_filled((0, 0), "", &Fill::solid(WHITE));
        assert_eq!(row_text(&buffer, 0, 3), "...");
    }

    #[test]
    fn print_filled_keeps_other_attributes() {
        let theme = Theme::default();
        let back = Color::Rgb(5, 6, 7);
        let buffer = bar_buffer((2, 1));
        let style = Style::from(ColorStyle::back(back)).combine(Effect::Bold);
        Printer::new((2, 1), &theme, &buffer)
            .with_style(style, |p| p.print_filled((0, 0), "ab", &Fill::solid(WHITE)));
        assert_eq!(back_at(&buffer, 1, 0), back);
        assert_eq!(front_at(&buffer, 1, 0), WHITE);
        let style = buffer.read().cell_style(Vec2::new(1, 0)).unwrap();
        assert!(style.effects.contains(Effect::Bold));
    }

    #[test]
    fn focused_border_color() {
        use crate::style::{BORDER_FOCUSED, Color};

        let border = Color::Rgb(1, 2, 3);
        let focused = Color::Rgb(4, 5, 6);
        let front = |theme: &Theme, has_focus: bool| {
            let mut buffer = PrintBuffer::new();
            buffer.resize(Vec2::new(6, 4));
            let buffer = RwLock::new(buffer);
            Printer::new((6, 4), theme, &buffer)
                .focused(has_focus)
                .print_box((0, 0), (6, 4), false);
            (front_at(&buffer, 0, 0), front_at(&buffer, 5, 3))
        };

        // Without the entry, the focus changes nothing.
        let mut theme = test_theme(BorderStyle::Rounded);
        theme.palette.set_custom_color(&BORDER, border);
        assert_eq!(front(&theme, true), (border, border));
        assert_eq!(front(&theme, false), (border, border));

        // With it, focused boxes use it.
        theme.palette.set_custom_color(&BORDER_FOCUSED, focused);
        assert_eq!(front(&theme, true), (focused, focused));
        assert_eq!(front(&theme, false), (border, border));

        // It works without `border` too: other boxes keep the primary color.
        let mut theme = test_theme(BorderStyle::Simple);
        theme.palette.set_color("border_focused", focused);
        let primary = theme.palette[PaletteColor::Primary];
        assert_eq!(front(&theme, true), (focused, focused));
        assert_eq!(front(&theme, false), (primary, primary));
    }

    fn dotted_grid(w: usize, h: usize) -> (Theme, RwLock<PrintBuffer>) {
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(w, h));
        let theme = Theme::default();
        let buffer = RwLock::new(buffer);
        let p = Printer::new((w, h), &theme, &buffer);
        for y in 0..h {
            p.print_hline((0, y), w, ".");
        }
        (theme, buffer)
    }

    fn buffer_rows(buffer: &RwLock<PrintBuffer>, w: usize, h: usize) -> Vec<String> {
        let buffer = buffer.read();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buffer.cell_text(Vec2::new(x, y)).unwrap_or("").to_string())
                    .collect()
            })
            .collect()
    }

    const FILLS: [f32; 14] = [
        -1.0,
        0.0,
        0.01,
        0.0625,
        0.1,
        0.125,
        0.3,
        0.5,
        0.77,
        0.99,
        1.0,
        2.0,
        f32::NAN,
        f32::INFINITY,
    ];

    fn expected_eighths(fill: f32, cells: usize) -> usize {
        let f = if fill.is_nan() {
            0.0
        } else {
            fill.clamp(0.0, 1.0) as f64
        };
        (f * (cells * 8) as f64).round() as usize
    }

    #[test]
    fn hbar_matches_a_reference_for_every_width_and_fill() {
        let h_blocks = ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
        for width in 0..=12usize {
            for fill in FILLS.iter().copied().chain([f32::NEG_INFINITY]) {
                let (theme, buffer) = dotted_grid(width + 4, 3);
                let p = Printer::new((width + 4, 3), &theme, &buffer);
                p.print_hbar((2, 1), width, fill);
                let out = buffer_rows(&buffer, width + 4, 3);
                assert_eq!(out[0], ".".repeat(width + 4));
                assert_eq!(out[2], ".".repeat(width + 4));
                let chars: Vec<char> = out[1].chars().collect();
                assert_eq!(chars.len(), width + 4, "{out:?}");
                assert_eq!(&chars[..2], ['.', '.'], "w {width} f {fill}");
                assert_eq!(&chars[2 + width..], ['.', '.'], "w {width} f {fill}");
                let e = expected_eighths(fill, width).min(width * 8);
                let mut want = String::new();
                for i in 0..width {
                    let level = e.saturating_sub(i * 8).min(8);
                    if level == 0 {
                        want.push('.');
                    } else {
                        want.push_str(h_blocks[level - 1]);
                    }
                }
                let got: String = chars[2..2 + width].iter().collect();
                assert_eq!(got, want, "hbar w {width} fill {fill}");
            }
        }
    }

    #[test]
    fn vbar_matches_a_reference_for_every_height_and_fill() {
        let blocks = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        for height in 0..=12usize {
            for fill in FILLS.iter().copied() {
                let (theme, buffer) = dotted_grid(5, height + 4);
                let p = Printer::new((5, height + 4), &theme, &buffer);
                p.print_vbar((2, 2), height, fill);
                let out = buffer_rows(&buffer, 5, height + 4);
                let e = expected_eighths(fill, height).min(height * 8);
                for (y, row) in out.iter().enumerate() {
                    let chars: Vec<char> = row.chars().collect();
                    for (x, c) in chars.iter().enumerate() {
                        if x != 2 || y < 2 || y >= 2 + height {
                            assert_eq!(*c, '.', "outside bar: h {height} f {fill} ({x},{y})");
                        }
                    }
                }
                for i in 0..height {
                    let level = e.saturating_sub(i * 8).min(8);
                    let want = if level == 0 { "." } else { blocks[level - 1] };
                    let y = 2 + height - 1 - i;
                    let got = out[y].chars().nth(2).unwrap().to_string();
                    assert_eq!(got, want, "vbar h {height} fill {fill} row {i}");
                }
            }
        }
    }

    #[test]
    fn filled_bars_and_text_do_not_panic_in_any_size() {
        use crate::style::chart::Fill;
        use crate::style::{BaseColor, Rgb};
        let fills = [
            Fill::solid(BaseColor::Red.dark()),
            Fill::steps([(0.0, BaseColor::Green.dark()), (0.5, BaseColor::Red.dark())]),
            Fill::vertical([Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]),
            Fill::horizontal([Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]),
            Fill::by_value([Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]),
        ];
        for fill in &fills {
            for n in 0..=12usize {
                for ratio in FILLS {
                    let (theme, buffer) = dotted_grid(n + 4, n + 4);
                    let p = Printer::new((n + 4, n + 4), &theme, &buffer);
                    p.print_hbar_filled((2, 2), n, ratio, fill);
                    p.print_vbar_filled((1, 2), n, ratio, fill);
                    p.print_filled((2, 1), "日本語 abc é", fill);
                    p.print_filled((0, 0), "", fill);
                }
            }
        }
    }

    fn strings() -> Vec<&'static str> {
        vec![
            "",
            "a",
            "ab",
            "abc",
            "abcdefghij",
            "日",
            "日本",
            "日本語",
            "a日b",
            "日a",
            "e\u{301}",
            "e\u{301}e\u{301}e\u{301}",
            "\u{301}",
            "a\u{301}日",
            "👨\u{200d}👩\u{200d}👧",
            "🇫🇷🇫🇷",
            "a\u{200b}b",
            "x\u{fe0f}y",
            "a\u{200d}",
            "\u{200d}日",
        ]
    }

    fn graphemes_prefix(s: &str, width: usize) -> String {
        let mut w = 0;
        let mut out = String::new();
        for g in s.graphemes(true) {
            if w + g.width() > width {
                break;
            }
            w += g.width();
            out.push_str(g);
        }
        out
    }

    #[test]
    fn print_aligned_keeps_the_expected_text_and_stays_in_its_span() {
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            for width in 0..=8usize {
                for s in strings() {
                    let total = 16;
                    let (theme, buffer) = dotted_grid(total, 1);
                    let p = Printer::new((total, 1), &theme, &buffer);
                    p.print_aligned((3, 0), width, align, s);
                    let b = buffer.read();
                    // Nothing outside of the span changes.
                    for x in (0..3).chain(3 + width..total) {
                        assert_eq!(
                            b.cell_text(Vec2::new(x, 0)),
                            Some("."),
                            "{align:?} width {width} {s:?}: cell {x} changed"
                        );
                    }
                    // Nothing in the span is an unrelated wide fragment: the printed
                    // text is the expected kept part of the string.
                    let mut got = String::new();
                    for x in 3..3 + width {
                        got.push_str(b.cell_text(Vec2::new(x, 0)).unwrap_or(""));
                    }
                    let got_no_dots: String = got.replace('.', "");
                    let sw = s.width();
                    let expected = if sw == 0 {
                        String::new()
                    } else if sw <= width {
                        s.to_string()
                    } else {
                        match align {
                            HAlign::Right => {
                                let rev: Vec<&str> = s.graphemes(true).collect();
                                let mut w = 0;
                                let mut kept = Vec::new();
                                for g in rev.iter().rev() {
                                    if w + g.width() > width {
                                        break;
                                    }
                                    w += g.width();
                                    kept.push(*g);
                                }
                                kept.reverse();
                                kept.concat()
                            }
                            _ => graphemes_prefix(s, width),
                        }
                    };
                    let strip = |t: &str| -> String {
                        t.graphemes(true).filter(|g| g.width() > 0).collect()
                    };
                    assert_eq!(
                        strip(&got_no_dots),
                        strip(&expected),
                        "{align:?} width {width} {s:?} (span text {got:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn print_aligned_positions() {
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            for width in 0..=9usize {
                for s in strings() {
                    if s.graphemes(true).any(|g| g.width() == 0) {
                        continue;
                    }
                    let total = 18;
                    let (theme, buffer) = dotted_grid(total, 1);
                    Printer::new((total, 1), &theme, &buffer).print_aligned(
                        (3, 0),
                        width,
                        align,
                        s,
                    );
                    let b = buffer.read();
                    let row: String = (0..total)
                        .map(|x| b.cell_text(Vec2::new(x, 0)).unwrap_or("").to_string())
                        .collect();

                    let sw = s.width();
                    let (kept, offset) = if sw <= width {
                        (s.to_string(), align.get_offset(sw, width))
                    } else if align == HAlign::Right {
                        let gs: Vec<&str> = s.graphemes(true).collect();
                        let mut w = 0;
                        let mut kept = Vec::new();
                        for g in gs.iter().rev() {
                            if w + g.width() > width {
                                break;
                            }
                            w += g.width();
                            kept.push(*g);
                        }
                        kept.reverse();
                        (kept.concat(), width - w)
                    } else {
                        (graphemes_prefix(s, width), 0)
                    };
                    let kw = kept.width();
                    let expected = format!(
                        "{}{}{}",
                        ".".repeat(3 + offset),
                        kept,
                        ".".repeat(total - 3 - offset - kw)
                    );
                    assert_eq!(row, expected, "{align:?} width {width} {s:?}");
                }
            }
        }
    }

    #[test]
    fn print_styled_aligned_stays_in_its_span() {
        use crate::utils::markup::StyledString;
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            for width in 0..=8usize {
                for a in strings() {
                    for b in ["", "xy", "日本", "é"] {
                        let mut text = StyledString::styled(a, Color::Rgb(255, 0, 0));
                        text.append(StyledString::styled(b, Color::Rgb(0, 0, 255)));
                        let total = 16;
                        let (theme, buffer) = dotted_grid(total, 1);
                        let p = Printer::new((total, 1), &theme, &buffer);
                        p.print_styled_aligned((3, 0), width, align, &text);
                        let buf = buffer.read();
                        for x in (0..3).chain(3 + width..total) {
                            assert_eq!(
                                buf.cell_text(Vec2::new(x, 0)),
                                Some("."),
                                "{align:?} width {width} {a:?}+{b:?}: cell {x} changed"
                            );
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "toml")]
    #[test]
    fn border_focused_and_chart_keys_from_toml() {
        let theme = crate::theme::load_toml(
            "[colors]\nborder = \"#888888\"\nborder_focused = \"#ffffff\"\n[colors.chart]\naxis = \"#555555\"\n",
        )
        .unwrap();
        assert_eq!(
            theme.palette.custom("border"),
            Some(&Color::Rgb(0x88, 0x88, 0x88))
        );
        assert_eq!(
            theme.palette.custom("border_focused"),
            Some(&Color::Rgb(255, 255, 255))
        );
        assert_eq!(
            theme.palette.custom("chart.axis"),
            Some(&Color::Rgb(0x55, 0x55, 0x55))
        );
        // And a box really uses them.
        let mut buffer = PrintBuffer::new();
        buffer.resize(Vec2::new(6, 4));
        let buffer = RwLock::new(buffer);
        Printer::new((6, 4), &theme, &buffer).print_box((0, 0), (6, 4), false);
        let b = buffer.read();
        assert_eq!(
            b.cell_style(Vec2::new(0, 0)).unwrap().color.front,
            Color::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn print_aligned_with_a_content_offset_stays_inside() {
        // A printer with a content offset (as in a scroll view) cuts text on the left.
        for align in [HAlign::Left, HAlign::Center, HAlign::Right] {
            for offset in 0..6usize {
                for s in ["abc", "日本語", "a日b", "e\u{301}x"] {
                    let (theme, buffer) = dotted_grid(12, 1);
                    let p = Printer::new((12, 1), &theme, &buffer);
                    let mut sub = p.clone();
                    sub.content_offset = Vec2::new(offset, 0);
                    sub.print_aligned((0, 0), 6, align, s);
                    let b = buffer.read();
                    assert_eq!(b.cell_text(Vec2::new(11, 0)), Some("."));
                }
            }
        }
    }
}
