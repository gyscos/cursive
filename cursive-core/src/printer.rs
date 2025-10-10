//! Provide higher-level abstraction to draw things on buffers.

use crate::buffer::{PrintBuffer, Window};
use crate::direction::Orientation;
use crate::rect::Rect;
use crate::style::{
    BorderStyle, ColorPair, ColorStyle, ConcreteStyle, Effect, PaletteColor, PaletteStyle, Style,
    StyleType,
};
use crate::theme::Theme;
use crate::utils::lines::simple::{prefix, suffix};
use crate::utils::span::IndexedSpan;
use crate::with::With;
use crate::Vec2;

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
        Printer {
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
        }
    }

    /// Returns the region of the window where this printer can write.
    pub fn output_window(&self) -> Rect {
        Rect::from_size(self.offset, self.output_size)
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
    pub fn on_window<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Window<'_>) -> R,
    {
        let mut buffer = self.buffer.write();
        let mut window = buffer
            .window(self.output_window())
            .expect("printer size exceeds backend size");
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

        self.with_high_border(invert, |s| {
            s.print(start, "┌");
            s.print(start + size.keep_y(), "└");
            s.print_hline(start + (1, 0), size.x - 1, "─");
            s.print_vline(start + (0, 1), size.y - 1, "│");
        });

        self.with_low_border(invert, |s| {
            s.print(start + size.keep_x(), "┐");
            s.print(start + size, "┘");
            s.print_hline(start + (1, 0) + size.keep_y(), size.x - 1, "─");
            s.print_vline(start + (0, 1) + size.keep_x(), size.y - 1, "│");
        });
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `false`,
    ///   use [`PaletteStyle::Tertiary`].
    /// * Otherwise, use [`PaletteStyle::Primary`].
    pub fn with_high_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let style = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if !invert => PaletteStyle::Tertiary,
            _ => PaletteStyle::Primary,
        };

        self.with_style(style, f);
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `true`,
    ///   use [`PaletteStyle::Tertiary`].
    /// * Otherwise, use [`PaletteStyle::Primary`].
    pub fn with_low_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let color = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if invert => PaletteStyle::Tertiary,
            _ => PaletteStyle::Primary,
        };

        self.with_style(color, f);
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
