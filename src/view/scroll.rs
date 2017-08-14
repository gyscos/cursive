use Printer;
use std::cmp::{max, min};

use theme::ColorStyle;
use vec::Vec2;

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
    /// `scrollbar_offset` columns to the left.
    ///
    /// (Useful when each item includes its own side borders,
    /// to draw the scrollbar inside.)
    pub scrollbar_offset: usize,

    /// Blank between the text and the scrollbar.
    pub right_padding: usize,
}

/// Defines the scrolling behaviour on content or size change
pub enum ScrollStrategy {
    /// Keeps the same row number
    KeepRow,
    /// Sticks to the top.
    StickToTop,
    /// Sticks to the bottom of the view.
    StickToBottom,
}

impl Default for ScrollStrategy {
    fn default() -> Self {
        ScrollStrategy::KeepRow
    }
}

impl ScrollBase {
    /// Creates a new, uninitialized scrollbar.
    pub fn new() -> Self {
        ScrollBase {
            start_line: 0,
            content_height: 0,
            view_height: 0,
            scrollbar_offset: 0,
            right_padding: 1,
        }
    }

    /// Shifts the scrollbar toward the inside of the view.
    ///
    /// Used by views that draw their side borders in the children.
    /// Pushing the scrollbar to the left allows it to stay inside
    /// the borders.
    pub fn scrollbar_offset(mut self, offset: usize) -> Self {
        self.scrollbar_offset = offset;
        self
    }

    /// Sets the number of blank cells between the text and the scrollbar.
    ///
    /// Defaults to 1.
    pub fn right_padding(mut self, padding: usize) -> Self {
        self.right_padding = padding;
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
        if self.scrollable() {
            self.start_line = self.content_height - self.view_height;
        }
    }

    /// Scroll down by the given number of line.
    ///
    /// Never further than the bottom of the view.
    pub fn scroll_down(&mut self, n: usize) {
        if self.scrollable() {
            self.start_line = min(self.start_line + n,
                                  self.content_height - self.view_height);
        }
    }

    /// Scroll up by the given number of lines.
    ///
    /// Never above the top of the view.
    pub fn scroll_up(&mut self, n: usize) {
        if self.scrollable() {
            self.start_line -= min(self.start_line, n);
        }
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
    /// ```no_run
    /// # use cursive::view::ScrollBase;
    /// # use cursive::Printer;
    /// # use cursive::theme;
    /// # use cursive::backend::{self, Backend};
    /// # let scrollbase = ScrollBase::new();
    /// # let b = backend::Concrete::init();
    /// # let t = theme::load_default();
    /// # let printer = Printer::new((5,1), &t, &b);
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
            // We have to remove the bar width and the padding.
            printer.size.x.saturating_sub(1 + self.right_padding)
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
            let scrollbar_x = printer.size.x.saturating_sub(1 + self.scrollbar_offset);
            printer.print_vline((scrollbar_x, 0), printer.size.y, "|");
            printer.with_color(color, |printer| {
                printer.print_vline((scrollbar_x, start), height, "▒");
            });
        }
    }
}
