//! Makes drawing on ncurses windows easier.


use backend::{self, Backend};
use std::cell::Cell;
use std::cmp::min;
use std::rc::Rc;

use theme::{BorderStyle, ColorStyle, Effect, Theme};
use unicode_segmentation::UnicodeSegmentation;

use utils::prefix_length;
use vec::Vec2;

/// Convenient interface to draw on a subset of the screen.
pub struct Printer<'a> {
    /// Offset into the window this printer should start drawing at.
    pub offset: Vec2,
    /// Size of the area we are allowed to draw on.
    pub size: Vec2,
    /// Whether the view to draw is currently focused or not.
    pub focused: bool,
    /// Currently used theme
    pub theme: Theme,

    /// `true` if nothing has been drawn yet.
    new: Rc<Cell<bool>>,
    /// Backend used to actually draw things
    backend: &'a backend::Concrete,
}

impl<'a> Printer<'a> {
    /// Creates a new printer on the given window.
    ///
    /// But nobody needs to know that.
    #[doc(hidden)]
    pub fn new<T: Into<Vec2>>(size: T, theme: Theme, backend: &'a backend::Concrete) -> Self {
        Printer {
            offset: Vec2::zero(),
            size: size.into(),
            focused: true,
            theme: theme,
            new: Rc::new(Cell::new(true)),
            backend: backend,
        }
    }

    /// Clear the screen.
    ///
    /// It will discard anything drawn before.
    ///
    /// Users rarely need to call this directly.
    pub fn clear(&self) {
        self.backend.clear();
    }

    /// Returns `true` if nothing has been printed yet.
    pub fn is_new(&self) -> bool {
        self.new.get()
    }

    // TODO: use &mut self? We don't *need* it, but it may make sense.
    // We don't want people to start calling prints in parallel?
    /// Prints some text at the given position relative to the window.
    pub fn print<S: Into<Vec2>>(&self, pos: S, text: &str) {
        self.new.set(false);

        let p = pos.into();
        if p.y >= self.size.y || p.x >= self.size.x {
            return;
        }
        // Do we have enough room for the entire line?
        let room = self.size.x - p.x;
        // We want the number of CHARACTERS, not bytes.
        // (Actually we want the "width" of the string, see unicode-width)
        let prefix_len = prefix_length(text.graphemes(true), room, "");
        let text = &text[..prefix_len];

        let p = p + self.offset;
        self.backend.print_at((p.x, p.y), text);
    }

    /// Prints a vertical line using the given character.
    pub fn print_vline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        self.new.set(false);

        let p = start.into();
        if p.y > self.size.y || p.x > self.size.x {
            return;
        }
        let len = min(len, self.size.y - p.y);

        let p = p + self.offset;
        for y in 0..len {
            self.backend.print_at((p.x, (p.y + y)), c);
        }
    }

    /// Prints a horizontal line using the given character.
    pub fn print_hline<T: Into<Vec2>>(&self, start: T, len: usize, c: &str) {
        self.new.set(false);

        let p = start.into();
        if p.y > self.size.y || p.x > self.size.x {
            return;
        }
        let len = min(len, self.size.x - p.x);
        let text: String = ::std::iter::repeat(c).take(len).collect();

        let p = p + self.offset;
        self.backend.print_at((p.x, p.y), &text);
    }

    /// Call the given closure with a colored printer,
    /// that will apply the given color on prints.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend::{self, Backend};
    /// # let b = backend::Concrete::init();
    /// # let printer = Printer::new((6,4), theme::load_default(), &b);
    /// printer.with_color(theme::ColorStyle::Highlight, |printer| {
    ///     printer.print((0,0), "This text is highlighted!");
    /// });
    /// ```
    pub fn with_color<F>(&self, c: ColorStyle, f: F)
        where F: FnOnce(&Printer)
    {
        self.backend.with_color(c, || f(self));
    }

    /// Same as `with_color`, but apply a ncurses style instead,
    /// like `ncurses::A_BOLD()` or `ncurses::A_REVERSE()`.
    ///
    /// Will probably use a cursive enum some day.
    pub fn with_effect<F>(&self, effect: Effect, f: F)
        where F: FnOnce(&Printer)
    {
        self.backend.with_effect(effect, || f(self));
    }

    /// Prints a rectangular box.
    ///
    /// If `invert` is `true`, and the theme uses `Outset` borders, then the
    /// box will use an "inset" style instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend::{self, Backend};
    /// # let b = backend::Concrete::init();
    /// # let printer = Printer::new((6,4), theme::load_default(), &b);
    /// printer.print_box((0,0), (6,4), false);
    /// ```
    pub fn print_box<T: Into<Vec2>, S: Into<Vec2>>(&self, start: T, size: S,
                                                   invert: bool) {
        self.new.set(false);

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
    ///   use `ColorStyle::Tertiary`.
    /// * Otherwise, use `ColorStyle::Primary`.
    pub fn with_high_border<F>(&self, invert: bool, f: F)
        where F: FnOnce(&Printer)
    {
        let color = match self.theme.borders {
            None => return,
            Some(BorderStyle::Outset) if !invert => ColorStyle::Tertiary,
            _ => ColorStyle::Primary,
        };

        self.with_color(color, f);
    }

    /// Runs the given function using a color depending on the theme.
    ///
    /// * If the theme's borders is `None`, return without calling `f`.
    /// * If the theme's borders is "outset" and `invert` is `true`,
    ///   use `ColorStyle::Tertiary`.
    /// * Otherwise, use `ColorStyle::Primary`.
    pub fn with_low_border<F>(&self, invert: bool, f: F)
        where F: FnOnce(&Printer)
    {
        let color = match self.theme.borders {
            None => return,
            Some(BorderStyle::Outset) if invert => ColorStyle::Secondary,
            _ => ColorStyle::Primary,
        };

        self.with_color(color, f);
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
    pub fn sub_printer<S: Into<Vec2>, T: Into<Vec2>>(&'a self, offset: S,
                                                     size: T, focused: bool)
                                                     -> Printer<'a> {
        let size = size.into();
        let offset = offset.into().or_min(self.size);
        Printer {
            offset: self.offset + offset,
            // We can't be larger than what remains
            size: Vec2::min(self.size - offset, size),
            focused: self.focused && focused,
            theme: self.theme.clone(),
            backend: self.backend,
            new: self.new.clone(),
        }
    }

    /// Returns a sub-printer with the given offset.
    pub fn offset<S: Into<Vec2>>(&self, offset: S, focused: bool) -> Printer {
        self.sub_printer(offset, self.size, focused)
    }
}
