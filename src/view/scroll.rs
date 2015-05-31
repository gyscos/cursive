use std::cmp::{min,max};

use color;
use vec::Vec2;
use printer::Printer;

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

    pub fn set_heights(&mut self, view_height: usize, content_height: usize) {
        self.view_height = view_height;
        self.content_height = content_height;

        if self.scrollable() {
            self.start_line = min(self.start_line, self.content_height - self.view_height);
        } else {
            self.start_line = 0;
        }
    }

    pub fn scrollable(&self) -> bool {
        self.view_height < self.content_height
    }

    pub fn can_scroll_up(&self) -> bool {
        self.start_line > 0
    }

    pub fn can_scroll_down(&self) -> bool {
        self.start_line + self.view_height < self.content_height
    }

    pub fn scroll_top(&mut self) {
        self.start_line = 0;
    }

    pub fn scroll_to(&mut self, y: usize) {
        if y >= self.start_line + self.view_height {
            self.start_line = 1 + y - self.view_height;
        } else if y < self.start_line {
            self.start_line = y;
        }
    }

    pub fn scroll_bottom(&mut self) {
        self.start_line = self.content_height - self.view_height;
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.start_line = min(self.start_line+n, self.content_height - self.view_height);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.start_line -= min(self.start_line, n);
    }

    pub fn draw<F>(&self, printer: &Printer, line_drawer: F)
        where F: Fn(&Printer,usize)
    {
        for y in 0..self.view_height {
            line_drawer(&printer.sub_printer(Vec2::new(0,y),printer.size,true), y+self.start_line);
        }


        if self.view_height < self.content_height {
            // We directly compute the size of the scrollbar (this allow use to avoid using floats).
            // (ratio) * max_height
            // Where ratio is ({start or end} / content.height)
            let height = max(1,self.view_height * self.view_height / self.content_height);
            // Number of different possible positions
            let steps = self.view_height - height + 1;

            // Now
            let start = steps * self.start_line / (1 + self.content_height - self.view_height);

            printer.with_color(
                if printer.focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE },
                |printer| {
                    printer.print_vline((printer.size.x-1, start), height, ' ' as u64);
                });
        }
    }
}
