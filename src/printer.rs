//! Makes drawing on ncurses windows easier.

use std::cmp::min;

use backend::Backend;

use B;

use theme::{ColorStyle, Effect, Theme};
use vec::Vec2;

/// Convenient interface to draw on a subset of the screen.
#[derive(Clone)]
pub struct Printer {
    /// Offset into the window this printer should start drawing at.
    pub offset: Vec2,
    /// Size of the area we are allowed to draw on.
    pub size: Vec2,
    /// Whether the view to draw is currently focused or not.
    pub focused: bool,
    /// Currently used theme
    pub theme: Theme,
}

impl Printer {
    /// Creates a new printer on the given window.
    pub fn new<T: Into<Vec2>>(size: T, theme: Theme) -> Self {
        Printer {
            offset: Vec2::zero(),
            size: size.into(),
            focused: true,
            theme: theme,
        }
    }

    // TODO: use &mut self? We don't *need* it, but it may make sense.
    // We don't want people to start calling prints in parallel?
    /// Prints some text at the given position relative to the window.
    pub fn print<S: Into<Vec2>>(&self, pos: S, text: &str) {
        let p = pos.into();
        if p.y >= self.size.y || p.x >= self.size.x {
            return;
        }
        // Do we have enough room for the entire line?
        let room = self.size.x - p.x;
        // We want the number of CHARACTERS, not bytes.
        // (Actually we want the "width" of the string, see unicode-width)
        let text = match text.char_indices().nth(room) {
            Some((i, _)) => &text[..i],
            _ => text,
        };

        let p = p + self.offset;
        if text.contains('%') {
            B::print_at((p.x, p.y), &text.replace("%", "%%"));
        } else {
            B::print_at((p.x, p.y), text);
        }
    }

    /// Prints a vertical line using the given character.
    pub fn print_vline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        let p = start.into();
        if p.y > self.size.y || p.x > self.size.x {
            return;
        }
        let len = min(len, self.size.y - p.y);

        let p = p + self.offset;
        for y in 0..len {
            B::print_at((p.x, (p.y + y)), c);
        }
    }

    /// Prints a horizontal line using the given character.
    pub fn print_hline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        let p = start.into();
        if p.y > self.size.y || p.x > self.size.x {
            return;
        }
        let len = min(len, self.size.x - p.x);

        let p = p + self.offset;
        for x in 0..len {
            B::print_at((p.x + x, p.y), c);
        }
    }

    /// Call the given closure with a colored printer,
    /// that will apply the given color on prints.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cursive::printer::Printer;
    /// # use cursive::theme;
    /// # let printer = Printer::new((6,4), theme::load_default());
    /// printer.with_color(theme::ColorStyle::Highlight, |printer| {
    ///     printer.print((0,0), "This text is highlighted!");
    /// });
    /// ```
    pub fn with_color<F>(&self, c: ColorStyle, f: F)
        where F: FnOnce(&Printer)
    {
        B::with_color(c, || f(self));
    }

    /// Same as `with_color`, but apply a ncurses style instead,
    /// like `ncurses::A_BOLD()` or `ncurses::A_REVERSE()`.
    ///
    /// Will probably use a cursive enum some day.
    pub fn with_effect<F>(&self, effect: Effect, f: F)
        where F: FnOnce(&Printer)
    {
        B::with_effect(effect, || f(self));
    }

    /// Prints a rectangular box.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive::printer::Printer;
    /// # use cursive::theme;
    /// # let printer = Printer::new((6,4), theme::load_default());
    /// printer.print_box((0,0), (6,4));
    /// ```
    pub fn print_box<T: Into<Vec2>, S: Into<Vec2>>(&self, start: T, size: S) {
        let start_v = start.into();
        let size_v = size.into() - (1, 1);

        self.print(start_v, "┌");
        self.print(start_v + size_v.keep_x(), "┐");
        self.print(start_v + size_v.keep_y(), "└");
        self.print(start_v + size_v, "┘");

        self.print_hline(start_v + (1, 0), size_v.x - 1, "─");
        self.print_vline(start_v + (0, 1), size_v.y - 1, "│");
        self.print_hline(start_v + (1, 0) + size_v.keep_y(),
                         size_v.x - 1,
                         "─");
        self.print_vline(start_v + (0, 1) + size_v.keep_x(),
                         size_v.y - 1,
                         "│");
    }

    /// Apply a selection style and call the given function.
    ///
    /// * If `selection` is `false`, simply uses `ColorStyle::Primary`.
    /// * If `selection` is `true`:
    ///     * If the printer currently has the focus,
    ///       uses `ColorStyle::Highlight`.
    ///     * Otherwise, uses `ColorStyle::HighlightInactive`.
    pub fn with_selection<F: FnOnce(&Printer)>(&self, selection: bool, f: F) {
        self.with_color(if selection {
                            if self.focused {
                                ColorStyle::Highlight
                            } else {
                                ColorStyle::HighlightInactive
                            }
                        } else {
                            ColorStyle::Primary
                        },
                        f);
    }

    /// Prints a horizontal delimiter with side border `├` and `┤`.
    pub fn print_hdelim<T: Into<Vec2>>(&self, start: T, len: usize) {
        let start = start.into();
        self.print(start, "├");
        self.print_hline(start + (1, 0), len - 2, "─");
        self.print(start + (len - 1, 0), "┤");
    }

    /// Returns a printer on a subset of this one's area.
    pub fn sub_printer<S: Into<Vec2>>(&self, offset: S, size: S, focused: bool)
                                      -> Printer {
        let offset_v = offset.into();
        Printer {
            offset: self.offset + offset_v,
            // We can't be larger than what remains
            size: Vec2::min(self.size - offset_v, size.into()),
            focused: self.focused && focused,
            theme: self.theme.clone(),
        }
    }
}
