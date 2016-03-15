use std::cmp::{min, max};
use ncurses::chtype;

use theme::ColorPair;
use vec::Vec2;
use printer::Printer;

/// Provide scrolling functionalities to a view.
pub struct ScrollBase {
    pub start_line: usize,
    pub content_height: usize,
    pub view_height: usize,
}

impl ScrollBase {
    pub fn new() -> Self {
        ScrollBase {
            start_line: 0,
            content_height: 0,
            view_height: 0,
        }
    }

    /// Call this method whem the content or the view changes.
    pub fn set_heights(&mut self, view_height: usize, content_height: usize) {
        self.view_height = view_height;
        self.content_height = content_height;

        if self.scrollable() {
            self.start_line = min(self.start_line, self.content_height - self.view_height);
        } else {
            self.start_line = 0;
        }
    }

    /// Returns `TRUE` if the view needs to scroll.
    pub fn scrollable(&self) -> bool {
        self.view_height < self.content_height
    }

    /// Returns `TRUE` unless we are at the top.
    pub fn can_scroll_up(&self) -> bool {
        self.start_line > 0
    }

    /// Returns `TRUE` unless we are at the bottom.
    pub fn can_scroll_down(&self) -> bool {
        self.start_line + self.view_height < self.content_height
    }

    /// Scroll to the top of the view.
    pub fn scroll_top(&mut self) {
        self.start_line = 0;
    }

    /// Makes sure that the given line is visible, scrolling if needed.
    pub fn scroll_to(&mut self, y: usize) {
        if y >= self.start_line + self.view_height {
            self.start_line = 1 + y - self.view_height;
        } else if y < self.start_line {
            self.start_line = y;
        }
    }

    /// Scroll to the bottom of the view.
    pub fn scroll_bottom(&mut self) {
        self.start_line = self.content_height - self.view_height;
    }

    /// Scroll down by the given number of line, never going further than the bottom of the view.
    pub fn scroll_down(&mut self, n: usize) {
        self.start_line = min(self.start_line + n, self.content_height - self.view_height);
    }

    /// Scroll up by the given number of lines, never going above the top of the view.
    pub fn scroll_up(&mut self, n: usize) {
        self.start_line -= min(self.start_line, n);
    }

    /// Draws the scroll bar and the content using the given drawer.
    ///
    /// `line_drawer` will be called once for each line that needs to be drawn.
    /// It will be given the absolute ID of the item to draw..
    /// It will also be given a printer with the correct offset,
    /// so it should only print on the first line.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive::view::ScrollBase;
    /// # use cursive::printer::Printer;
    /// # use cursive::theme;
    /// # let scrollbase = ScrollBase::new();
    /// # let printer = Printer::new((5,1), theme::load_default());
    /// # let printer = &printer;
    /// let lines = ["Line 1", "Line number 2"];
    /// scrollbase.draw(printer, |printer, i| {
    ///     printer.print((0,0), lines[i]);
    /// });
    /// ```
    pub fn draw<F>(&self, printer: &Printer, line_drawer: F)
        where F: Fn(&Printer, usize)
    {
        // Print the content in a sub_printer
        let max_y = min(self.view_height, self.content_height - self.start_line);
        let w = if self.scrollable() {
            printer.size.x - 2
        } else {
            printer.size.x
        };
        for y in 0..max_y {
            // Y is the actual coordinate of the line.
            // The item ID is then Y + self.start_line
            line_drawer(&printer.sub_printer(Vec2::new(0, y), Vec2::new(w, 1), true),
                        y + self.start_line);
        }


        // And draw the scrollbar if needed
        if self.view_height < self.content_height {
            // We directly compute the size of the scrollbar (this allow use to avoid using floats).
            // (ratio) * max_height
            // Where ratio is ({start or end} / content.height)
            let height = max(1, self.view_height * self.view_height / self.content_height);
            // Number of different possible positions
            let steps = self.view_height - height + 1;

            // Now
            let start = steps * self.start_line / (1 + self.content_height - self.view_height);

            let color = if printer.focused {
                ColorPair::Highlight
            } else {
                ColorPair::HighlightInactive
            };

            printer.print_vline((printer.size.x - 1, 0), printer.size.y, '|' as chtype);
            printer.with_color(color, |printer| {
                printer.print_vline((printer.size.x - 1, start), height, ' ' as chtype);
            });
        }
    }
}
