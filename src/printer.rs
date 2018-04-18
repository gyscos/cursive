//! Provide higher-level abstraction to draw things on backends.

use backend::Backend;
use enumset::EnumSet;
use std::cmp::min;
use theme::{BorderStyle, ColorStyle, Effect, PaletteColor, Style, Theme};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use utils::lines::simple::{prefix, suffix};
use vec::Vec2;
use with::With;

/// Convenient interface to draw on a subset of the screen.
///
/// The area it can print on is defined by `offset` and `size`.
///
/// The part of the content it will print there is defined by `content_offset`
/// and `size`.
pub struct Printer<'a> {
    /// Offset into the window this printer should start drawing at.
    ///
    /// Printing at `x` will really print at `x + offset`.
    pub offset: Vec2,

    /// Size of the area we are allowed to draw on.
    ///
    /// Anything outside of this should be discarded.
    pub size: Vec2,

    /// Offset into the view for this printer.
    ///
    /// Printing at `x`, will really print at `x - content_offset`.
    pub content_offset: Vec2,

    /// Whether the view to draw is currently focused or not.
    pub focused: bool,

    /// Currently used theme
    pub theme: &'a Theme,

    /// Backend used to actually draw things
    backend: &'a Backend,
}

impl<'a> Clone for Printer<'a> {
    fn clone(&self) -> Self {
        Printer {
            offset: self.offset,
            content_offset: self.content_offset,
            size: self.size,
            focused: self.focused,
            theme: self.theme,
            backend: self.backend,
        }
    }
}

impl<'a> Printer<'a> {
    /// Creates a new printer on the given window.
    ///
    /// But nobody needs to know that.
    #[doc(hidden)]
    pub fn new<T: Into<Vec2>>(
        size: T, theme: &'a Theme, backend: &'a Backend
    ) -> Self {
        Printer {
            offset: Vec2::zero(),
            content_offset: Vec2::zero(),
            size: size.into(),
            focused: true,
            theme,
            backend,
        }
    }

    /// Clear the screen.
    ///
    /// It will discard anything drawn before.
    ///
    /// Users rarely need to call this directly.
    pub fn clear(&self) {
        self.backend
            .clear(self.theme.palette[PaletteColor::Background]);
    }

    // TODO: use &mut self? We don't *need* it, but it may make sense.
    // We don't want people to start calling prints in parallel?
    /// Prints some text at the given position relative to the window.
    pub fn print<S: Into<Vec2>>(&self, pos: S, text: &str) {
        let pos = pos.into();

        if !pos.fits_in(self.size + self.content_offset) {
            return;
        }

        // This is the part of the text that's hidden
        // (smaller than the content offset)
        let hidden_part = self.content_offset.saturating_sub(pos);
        if hidden_part.y > 0 {
            // Since we are printing a single line, there's nothing we can do.
            return;
        }

        let text_width = text.width();

        // We have to drop hidden_part.x width from the start of the string.
        // prefix() may be too short if there's a double-width character.
        // So instead, keep the suffix and drop the prefix.

        // TODO: use a different prefix method that is *at least* the width
        // (and not *at most*)
        let tail = suffix(text.graphemes(true), text_width - hidden_part.x, "");
        let skipped_len = text.len() - tail.length;
        let skipped_width = text_width - tail.width;

        // This should be equal most of the time, except when there's a double
        // character preventing us from splitting perfectly.
        assert!(skipped_width >= hidden_part.x);

        // Drop part of the text, and move the cursor correspondingly.
        let text = &text[skipped_len..];
        let pos = pos + (skipped_width, 0);

        // What we did before should guarantee that this won't overflow.
        let pos = pos - self.content_offset;

        // Do we have enough room for the entire line?
        let room = self.size.x - pos.x;

        // Drop the end of the text if it's too long
        // We want the number of CHARACTERS, not bytes.
        // (Actually we want the "width" of the string, see unicode-width)
        let prefix_len = prefix(text.graphemes(true), room, "").length;
        let text = &text[..prefix_len];

        let pos = pos + self.offset;
        self.backend.print_at(pos, text);
    }

    /// Prints a vertical line using the given character.
    pub fn print_vline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        let start = start.into();

        if !start.fits_in(self.size + self.content_offset) {
            return;
        }

        let hidden_part = self.content_offset.saturating_sub(start);
        if hidden_part.x > 0 || hidden_part.y >= len {
            // We're printing a single column, so we can't do much here.
            return;
        }

        // Skip `hidden_part`
        let start = start + hidden_part;
        let len = len - hidden_part.y;

        // What we did before ensures this won't overflow.
        let start = start - self.content_offset;

        // Don't go overboard
        let len = min(len, self.size.y - start.y);

        let start = start + self.offset;
        for y in 0..len {
            self.backend.print_at(start + (0,y), c);
        }
    }

    /// Prints a horizontal line using the given character.
    pub fn print_hline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        let start = start.into();

        if !start.fits_in(self.size + self.content_offset) {
            return;
        }

        let hidden_part = self.content_offset.saturating_sub(start);
        if hidden_part.y > 0 || hidden_part.x >= len {
            // We're printing a single line, so we can't do much here.
            return;
        }

        // Skip `hidden_part`
        let start = start + hidden_part;
        let len = len - hidden_part.x;

        // Don't go overboard
        let start = start - self.content_offset;

        // Don't write too much if we're close to the end
        let len = min(len, (self.size.x - start.x) / c.width());

        // Could we avoid allocating?
        let text: String = ::std::iter::repeat(c).take(len).collect();

        let start = start + self.offset;
        self.backend.print_at(start, &text);
    }

    /// Call the given closure with a colored printer,
    /// that will apply the given color on prints.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend;
    /// # let b = backend::dummy::Backend::init();
    /// # let t = theme::load_default();
    /// # let printer = Printer::new((6,4), &t, &*b);
    /// printer.with_color(theme::ColorStyle::highlight(), |printer| {
    ///     printer.print((0,0), "This text is highlighted!");
    /// });
    /// ```
    pub fn with_color<F>(&self, c: ColorStyle, f: F)
    where
        F: FnOnce(&Printer),
    {
        let old = self.backend.set_color(c.resolve(&self.theme.palette));
        f(self);
        self.backend.set_color(old);
    }

    /// Call the given closure with a styled printer,
    /// that will apply the given style on prints.
    pub fn with_style<F, T>(&self, style: T, f: F)
    where
        F: FnOnce(&Printer),
        T: Into<Style>,
    {
        let style = style.into();

        let color = style.color;
        let effects = style.effects;

        // eprintln!("{:?}", effects);

        if let Some(color) = color {
            self.with_color(color, |printer| {
                printer.with_effects(effects, f);
            });
        } else {
            self.with_effects(effects, f);
        }
    }

    /// Call the given closure with a modified printer
    /// that will apply the given effect on prints.
    pub fn with_effect<F>(&self, effect: Effect, f: F)
    where
        F: FnOnce(&Printer),
    {
        self.backend.set_effect(effect);
        f(self);
        self.backend.unset_effect(effect);
    }

    /// Call the given closure with a modified printer
    /// that will apply each given effect on prints.
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
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend;
    /// # let b = backend::dummy::Backend::init();
    /// # let t = theme::load_default();
    /// # let printer = Printer::new((6,4), &t, &*b);
    /// printer.print_box((0,0), (6,4), false);
    /// ```
    pub fn print_box<T: Into<Vec2>, S: Into<Vec2>>(
        &self, start: T, size: S, invert: bool
    ) {
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
            s.print_hline(
                start + (1, 0) + size.keep_y(),
                size.x - 1,
                "─",
            );
            s.print_vline(
                start + (0, 1) + size.keep_x(),
                size.y - 1,
                "│",
            );
        });
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `false`,
    ///   use `ColorStyle::Tertiary`.
    /// * Otherwise, use `ColorStyle::Primary`.
    pub fn with_high_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let color = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if !invert => ColorStyle::tertiary(),
            _ => ColorStyle::primary(),
        };

        self.with_color(color, f);
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `true`,
    ///   use `ColorStyle::tertiary()`.
    /// * Otherwise, use `ColorStyle::primary()`.
    pub fn with_low_border<F>(&self, invert: bool, f: F)
    where
        F: FnOnce(&Printer),
    {
        let color = match self.theme.borders {
            BorderStyle::None => return,
            BorderStyle::Outset if invert => ColorStyle::tertiary(),
            _ => ColorStyle::primary(),
        };

        self.with_color(color, f);
    }

    /// Apply a selection style and call the given function.
    ///
    /// * If `selection` is `false`, simply uses `ColorStyle::primary()`.
    /// * If `selection` is `true`:
    ///     * If the printer currently has the focus,
    ///       uses `ColorStyle::highlight()`.
    ///     * Otherwise, uses `ColorStyle::highlight_inactive()`.
    pub fn with_selection<F: FnOnce(&Printer)>(&self, selection: bool, f: F) {
        self.with_color(
            if selection {
                if self.focused {
                    ColorStyle::highlight()
                } else {
                    ColorStyle::highlight_inactive()
                }
            } else {
                ColorStyle::primary()
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
    pub fn offset<S>(&self, offset: S) -> Printer
    where
        S: Into<Vec2>,
    {
        let offset = offset.into();
        self.clone().with(|s| {
            let consumed = Vec2::min(s.content_offset, offset);

            s.content_offset = s.content_offset - consumed;
            s.offset = s.offset + offset - consumed;
            s.size = s.size.saturating_sub(offset);
        })
    }

    /// Returns a new sub-printer inheriting the given focus.
    ///
    /// If `self` is focused and `focused == true`, the child will be focused.
    ///
    /// Otherwise, he will be unfocused.
    pub fn focused(&self, focused: bool) -> Self {
        self.clone().with(|s| {
            s.focused &= focused;
        })
    }

    /// Returns a new sub-printer with a cropped area.
    ///
    /// The new printer size will be the minimum of `size` and its current size.
    ///
    /// Any size reduction happens at the bottom-right.
    pub fn cropped<S>(&self, size: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.clone().with(|s| {
            s.size = Vec2::min(s.size, size);
        })
    }

    /// Returns a new sub-printer with a shrinked area.
    ///
    /// The printer size will be reduced by the given border from the bottom-right.
    pub fn shrinked<S>(&self, borders: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.cropped(self.size.saturating_sub(borders))
    }

    /// Returns a new sub-printer with a content offset.
    pub fn content_offset<S>(&self, offset: S) -> Self
    where
        S: Into<Vec2>,
    {
        self.clone().with(|s| {
            s.content_offset = s.content_offset + offset;
        })
    }
}
