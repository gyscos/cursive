use std::cmp::{max, min};

use theme::ColorStyle;
use vec::Vec2;
use Printer;

/// Provide scrolling functionalities to a view.
///
/// You're not supposed to use this directly,
/// but it can be helpful if you create your own Views.
#[derive(Default)]
pub struct ScrollBase {
    /// First line visible
    pub start_line: usize,
    /// Content height
    pub content_height: usize,
    /// Number of lines displayed
    pub view_height: usize,
    /// Padding for the scrollbar
    ///
    /// If present, the scrollbar will be shifted
    /// `scrollbar_padding` columns to the left.
    ///
    /// (Useful when each item includes its own side borders,
    /// to draw the scrollbar inside.)
    pub scrollbar_padding: usize,
}

impl ScrollBase {
    /// Creates a new, uninitialized scrollbar.
    pub fn new() -> Self {
        ScrollBase {
            start_line: 0,
            content_height: 0,
            view_height: 0,
            scrollbar_padding: 0,
        }
    }

    /// Shifts the scrollbar toward the inside of the view.
    ///
    /// Used by views that draw their side borders in the children.
    /// Pushing the scrollbar to the left allows it to stay inside
    /// the borders.
    pub fn bar_padding(mut self, padding: usize) -> Self {
        self.scrollbar_padding = padding;
        self
    }

    /// Call this method whem the content or the view changes.
    pub fn set_heights(&mut self, view_height: usize, content_height: usize) {
        self.view_height = view_height;
        self.content_height = content_height;

        if self.scrollable() {
            self.start_line = min(self.start_line,
                                  self.content_height - self.view_height);
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

    /// Scroll down by the given number of line.
    ///
    /// Never further than the bottom of the view.
    pub fn scroll_down(&mut self, n: usize) {
        self.start_line = min(self.start_line + n,
                              self.content_height - self.view_height);
    }

    /// Scroll up by the given number of lines.
    ///
    /// Never above the top of the view.
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
    /// # use cursive::Printer;
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
        if self.view_height == 0 {
            return;
        }
        // Print the content in a sub_printer
        let max_y = min(self.view_height,
                        self.content_height - self.start_line);
        let w = if self.scrollable() {
            if printer.size.x < 2 {
                return;
            }
            printer.size.x - 2 + self.scrollbar_padding // TODO: 2
        } else {
            printer.size.x
        };
        for y in 0..max_y {
            // Y is the actual coordinate of the line.
            // The item ID is then Y + self.start_line
            line_drawer(&printer.sub_printer(Vec2::new(0, y),
                                             Vec2::new(w, 1),
                                             true),
                        y + self.start_line);
        }


        // And draw the scrollbar if needed
        if self.view_height < self.content_height {
            // We directly compute the size of the scrollbar
            // (that way we avoid using floats).
            // (ratio) * max_height
            // Where ratio is ({start or end} / content.height)
            let height = max(1,
                             self.view_height * self.view_height /
                             self.content_height);
            // Number of different possible positions
            let steps = self.view_height - height + 1;

            // Now
            let start = steps * self.start_line /
                        (1 + self.content_height - self.view_height);

            let color = if printer.focused {
                ColorStyle::Highlight
            } else {
                ColorStyle::HighlightInactive
            };

            // TODO: use 1 instead of 2
            let scrollbar_x = printer.size.x - 1 - self.scrollbar_padding;
            printer.print_vline((scrollbar_x, 0), printer.size.y, "|");
            printer.with_color(color, |printer| {
                printer.print_vline((scrollbar_x, start), height, " ");
            });
        }
    }
}
