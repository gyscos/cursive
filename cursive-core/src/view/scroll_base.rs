use crate::div::div_up;
use crate::theme::ColorStyle;
use crate::Printer;
use crate::Vec2;
use std::cmp::{max, min};

/// Provide scrolling functionalities to a view.
///
/// This is a legacy helper utility to define scrollable views.
///
/// The [`scroll`] module is the preferred way to achieve that.
///
/// [`scroll`]: ./scroll/index.html
#[derive(Default, Debug)]
#[deprecated(
    since = "0.16.0",
    note = "`ScrollBase` is being deprecated in favor of the view::scroll module."
)]
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

    /// Initial position of the cursor when dragging.
    pub thumb_grab: Option<usize>,
}

#[allow(deprecated)]
impl ScrollBase {
    /// Creates a new, uninitialized scrollbar.
    pub fn new() -> Self {
        ScrollBase {
            start_line: 0,
            content_height: 0,
            view_height: 0,
            scrollbar_offset: 0,
            right_padding: 1,
            thumb_grab: None,
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

        // eprintln!("Setting heights: {} in {}", content_height, view_height);

        if self.scrollable() {
            self.start_line =
                min(self.start_line, self.content_height - self.view_height);
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
            self.start_line = min(
                self.start_line + n,
                self.content_height - self.view_height,
            );
        }
    }

    /// Scrolls down until the scrollbar thumb is at the given location.
    pub fn scroll_to_thumb(&mut self, thumb_y: usize, thumb_height: usize) {
        // The min() is there to stop at the bottom of the content.
        // The saturating_sub is there to stop at the bottom of the content.
        // eprintln!("Scrolling to {}", thumb_y);
        self.start_line = min(
            div_up(
                (1 + self.content_height - self.view_height) * thumb_y,
                self.view_height - thumb_height + 1,
            ),
            self.content_height - self.view_height,
        );
    }

    /// Scroll up by the given number of lines.
    ///
    /// Never above the top of the view.
    pub fn scroll_up(&mut self, n: usize) {
        if self.scrollable() {
            self.start_line -= min(self.start_line, n);
        }
    }

    /// Starts scrolling from the given cursor position.
    pub fn start_drag(&mut self, position: Vec2, width: usize) -> bool {
        // First: are we on the correct column?
        let scrollbar_x = self.scrollbar_x(width);
        // eprintln!("Grabbed {} for {}", position.x, scrollbar_x);
        if position.x != scrollbar_x {
            return false;
        }

        // Now, did we hit the thumb? Or should we direct-jump?
        let height = self.scrollbar_thumb_height();
        let thumb_y = self.scrollbar_thumb_y(height);

        if position.y >= thumb_y && position.y < thumb_y + height {
            // Grabbed!
            self.thumb_grab = Some(position.y - thumb_y);
        } else {
            // Just jump a bit...
            self.thumb_grab = Some((height - 1) / 2);
            // eprintln!("Grabbed at {}", self.thumb_grab);
            self.drag(position);
        }

        true
    }

    /// Keeps scrolling by dragging the cursor.
    pub fn drag(&mut self, position: Vec2) {
        // Our goal is self.scrollbar_thumb_y()+thumb_grab == position.y
        // Which means that position.y is the middle of the scrollbar.
        // eprintln!("Dragged: {:?}", position);
        // eprintln!("thumb: {:?}", self.thumb_grab);
        if let Some(grab) = self.thumb_grab {
            let height = self.scrollbar_thumb_height();
            self.scroll_to_thumb(position.y.saturating_sub(grab), height);
        }
    }

    /// Returns `true` if we are in the process of dragging the scroll thumb.
    pub fn is_dragging(&self) -> bool {
        self.thumb_grab.is_some()
    }

    /// Stops grabbing the scrollbar.
    pub fn release_grab(&mut self) {
        self.thumb_grab = None;
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
    /// ```rust
    /// # let scrollbase = cursive_core::view::ScrollBase::new();
    /// # let b = cursive_core::backend::Dummy::init();
    /// # let t = cursive_core::theme::load_default();
    /// # let printer = &cursive_core::Printer::new((5,1), &t, &*b);
    /// # let printer = &printer;
    /// let lines = ["Line 1", "Line number 2"];
    /// scrollbase.draw(printer, |printer, i| {
    ///     printer.print((0, 0), lines[i]);
    /// });
    /// ```
    pub fn draw<F>(&self, printer: &Printer, line_drawer: F)
    where
        F: Fn(&Printer, usize),
    {
        if self.view_height == 0 {
            return;
        }
        // Print the content in a sub_printer
        let max_y =
            min(self.view_height, self.content_height - self.start_line);
        let w = if self.scrollable() {
            // We have to remove the bar width and the padding.
            printer.size.x.saturating_sub(1 + self.right_padding)
        } else {
            printer.size.x
        };

        for y in 0..max_y {
            // Y is the actual coordinate of the line.
            // The item ID is then Y + self.start_line
            line_drawer(
                &printer.offset((0, y)).cropped((w, 1)),
                y + self.start_line,
            );
        }

        // And draw the scrollbar if needed
        if self.view_height < self.content_height {
            // We directly compute the size of the scrollbar
            // (that way we avoid using floats).
            // (ratio) * max_height
            // Where ratio is ({start or end} / content.height)
            let height = self.scrollbar_thumb_height();
            let start = self.scrollbar_thumb_y(height);

            let color = if printer.focused {
                ColorStyle::highlight()
            } else {
                ColorStyle::highlight_inactive()
            };

            let scrollbar_x = self.scrollbar_x(printer.size.x);
            // eprintln!("Drawing bar at x={}", scrollbar_x);

            // The background
            printer.print_vline((scrollbar_x, 0), printer.size.y, "|");

            // The scrollbar thumb
            printer.with_color(color, |printer| {
                printer.print_vline((scrollbar_x, start), height, "▒");
            });
        }
    }

    /// Returns the X position of the scrollbar, given the size available.
    ///
    /// Note that this does not depend whether or
    /// not a scrollbar will actually be present.
    pub fn scrollbar_x(&self, total_size: usize) -> usize {
        total_size.saturating_sub(1 + self.scrollbar_offset)
    }

    /// Returns the height of the scrollbar thumb.
    pub fn scrollbar_thumb_height(&self) -> usize {
        max(1, self.view_height * self.view_height / self.content_height)
    }

    /// Returns the y position of the scrollbar thumb.
    pub fn scrollbar_thumb_y(&self, scrollbar_thumb_height: usize) -> usize {
        let steps = self.view_height - scrollbar_thumb_height + 1;
        steps * self.start_line / (1 + self.content_height - self.view_height)
    }
}
