use std::cmp::{max,min};

use color;
use vec::Vec2;
use view::{View,DimensionRequest,SizeRequest};
use div::*;
use printer::Printer;
use event::*;

/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,
    start_line: usize,
    view_height: usize,
}

// Subset of the main content representing a row on the display.
struct Row {
    start: usize,
    end: usize,
}

// If the last character is a newline, strip it.
fn strip_last_newline(content: &str) -> &str {
    if !content.is_empty() && content.chars().last().unwrap() == '\n' {
        &content[..content.len() - 1]
    } else {
        content
    }
}

/// Returns the number of lines required to display the given text with the
/// specified maximum line width.
fn get_line_span(line: &str, max_width: usize) -> usize {
    // TODO: this method is stupid. Look at LinesIterator and do the same
    // (Or use a common function? Better!)
    let mut lines = 1;
    let mut length = 0;
    for l in line.split(" ").map(|word| word.len()) {
        length += l;
        if length > max_width {
            length = l;
            lines += 1;
        }
        length += 1;
    }
    lines
}

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new(content: &str) -> Self {
        let content = strip_last_newline(content);
        TextView {
            content: content.to_string(),
            start_line: 0,
            rows: Vec::new(),
            view_height: 0,
        }
    }

    /// Replace the text in this view.
    pub fn set_content(&mut self, content: &str) {
        let content = strip_last_newline(content);
        self.content = content.to_string();
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Returns the number of lines required to display the content
    /// with the given width.
    fn get_num_lines(&self, max_width: usize) -> usize {
        self.content.split("\n")
            .map(|line| get_line_span(line, max_width))
            .fold(0, |sum, x| sum + x)
    }

    // Given the specified height, how many columns do we need to properly display?
    fn get_num_cols(&self, max_height: usize) -> usize {
        (div_up_usize(self.content.len(), max_height)..self.content.len())
            .find(|w| self.get_num_lines(*w) <= max_height)
            .unwrap()
    }

    // In the absence of any constraint, what size would we like?
    fn get_ideal_size(&self) -> Vec2 {
        let mut max_width = 0;
        let mut height = 0;

        for line in self.content.split("\n") {
            height += 1;
            max_width = max(max_width, line.len());
        }

        Vec2::new(max_width, height)
    }
}

// Given a multiline string, and a given maximum width,
// iterates on the computed rows.
struct LinesIterator<'a> {
    content: &'a str,
    start: usize,
    width: usize,
}

impl <'a> LinesIterator<'a> {
    // Start an iterator on the given content.
    fn new(content: &'a str, width: usize) -> Self {
        LinesIterator {
            content: content,
            width: width,
            start: 0,
        }
    }
}

impl <'a> Iterator for LinesIterator<'a> {

    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.start >= self.content.len() {
            // This is the end.
            return None;
        }

        let start = self.start;
        let content = &self.content[self.start..];

        if let Some(next) = content.find("\n") {
            if next <= self.width {
                // We found a newline before the allowed limit.
                // Break early.
                self.start += next+1;
                return Some(Row {
                    start: start,
                    end: next + start,
                });
            }
        }

        if content.len() <= self.width {
            // I thought it would be longer! -- that's what she said :(
            self.start += content.len();
            return Some(Row{
                start: start,
                end: start + content.len(),
            });
        }

        if let Some(i) = content[..self.width+1].rfind(" ") {
            // If we have to break, try to find a whitespace for that.
            self.start += i+1;
            return Some(Row {
                start: start,
                end: i + start,
            });
        }

        // Meh, no whitespace, so just cut in this mess.
        // TODO: look for ponctuation instead?
        self.start += self.width;
        return Some(Row {
            start: start,
            end: start + self.width,
        });
    }
}

impl View for TextView {
    fn draw(&mut self, printer: &Printer, focused: bool) {
        // We don't have a focused view
        for (i,line) in self.rows.iter().skip(self.start_line).map(|row| &self.content[row.start..row.end]).enumerate() {
            printer.print((0,i), line);
        }
        if self.view_height < self.rows.len() {
            // We directly compute the size of the scrollbar (this allow use to avoid using floats).
            // (ratio) * max_height
            // Where ratio is ({start or end} / content.height)
            let start = self.view_height * self.start_line / self.rows.len();
            let end = self.view_height * (self.start_line + self.view_height) / self.rows.len();
            printer.with_color(
                if focused { color::HIGHLIGHT } else { color::HIGHLIGHT_INACTIVE },
                |printer| {
                    printer.print_vline((printer.size.x-1, start), end-start, ' ' as u64);
                });
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.view_height >= self.rows.len() {
            return EventResult::Ignored;
        }

        match event {
            Event::KeyEvent(Key::Up) if self.start_line > 0 => self.start_line -= 1,
            Event::KeyEvent(Key::Down) if self.start_line+self.view_height < self.rows.len() => self.start_line += 1,
            Event::KeyEvent(Key::PageUp) => self.start_line = min(self.start_line+10, self.rows.len()-self.view_height),
            Event::KeyEvent(Key::PageDown) => self.start_line -= min(self.start_line, 10),
            _ => return EventResult::Ignored,
        }

        return EventResult::Consumed(None);
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        match (size.w,size.h) {
            // If we have no directive, ask for a single big line.
            // TODO: what if the text has newlines??
            (DimensionRequest::Unknown, DimensionRequest::Unknown) => self.get_ideal_size(),
            (DimensionRequest::Fixed(w),_) => {
                let h = self.get_num_lines(w);
                Vec2::new(w, h)
            },
            (_,DimensionRequest::Fixed(h)) => {
                let w = self.get_num_cols(h);
                Vec2::new(w, h)
            },
            (DimensionRequest::AtMost(w),_) => {
                // Don't _force_ the max width, but take it if we have to.
                let ideal = self.get_ideal_size();

                if w >= ideal.x {
                    ideal
                } else {
                    let h = self.get_num_lines(w);
                    Vec2::new(w, h)
                }
            },
            _ => unreachable!(),
        }
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.view_height = size.y;
        self.rows = LinesIterator::new(&self.content, size.x ).collect();
        if self.rows.len() > size.y {
            self.rows = LinesIterator::new(&self.content, size.x - 1).collect();
            self.start_line = min(self.start_line, self.rows.len() - size.y);
        } else {
            self.start_line = 0;
        }
    }
}
