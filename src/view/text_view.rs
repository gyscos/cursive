use std::cmp;

use vec::Vec2;
use view::{View,DimensionRequest,SizeRequest};
use div::*;
use printer::Printer;

/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,
    start_line: usize,
}

struct Row {
    start: usize,
    end: usize,
}

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
    let mut lines = 1;
    let mut length = 0;
    for l in line.split(" ").map(|word| word.len()) {
        length += l;
        if length >= max_width {
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

    fn get_num_cols(&self, max_height: usize) -> usize {
        (div_up_usize(self.content.len(), max_height)..self.content.len())
            .find(|w| self.get_num_lines(*w) <= max_height)
            .unwrap()
    }

    fn get_ideal_size(&self) -> Vec2 {
        let mut max_width = 0;
        let mut height = 0;

        for line in self.content.split("\n") {
            height += 1;
            max_width = cmp::max(max_width, line.len() as u32);
        }

        Vec2::new(max_width, height)
    }
}

struct LinesIterator<'a> {
    content: &'a str,
    start: usize,
    width: usize,
}

impl <'a> LinesIterator<'a> {
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
            // This is the end
            return None;
        }

        let start = self.start;
        let content = &self.content[self.start..];

        if let Some(next) = content.find("\n") {
            if next < self.width {
                self.start += next+1;
                return Some(Row {
                    start: start,
                    end: next + start,
                });
            }
        }

        if content.len() <= self.width {
            self.start += content.len();
            return Some(Row{
                start: start,
                end: start + content.len(),
            });
        }


        if let Some(i) = content[..self.width].rfind(" ") {
            self.start += i+1;
            return Some(Row {
                start: start,
                end: i + start,
            });
        }

        self.start += self.width;
        return Some(Row {
            start: start,
            end: start + self.width,
        });
    }
}

impl View for TextView {
    fn draw(&mut self, printer: &Printer, _: bool) {
        // We don't have a focused view
        for (i,line) in self.rows.iter().skip(self.start_line).map(|row| &self.content[row.start..row.end]).enumerate() {
            printer.print((0,i), line);
        }
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        match (size.w,size.h) {
            // If we have no directive, ask for a single big line.
            // TODO: what if the text has newlines??
            (DimensionRequest::Unknown, DimensionRequest::Unknown) => Vec2::new(self.content.len() as u32, 1),
            (DimensionRequest::Fixed(w),_) => {
                let h = self.get_num_lines(w as usize) as u32;
                Vec2::new(w, h)
            },
            (_,DimensionRequest::Fixed(h)) => {
                let w = self.get_num_cols(h as usize) as u32;
                Vec2::new(w, h)
            },
            (DimensionRequest::AtMost(w),_) => {
                // Don't _force_ the max width, but take it if we have to.
                let ideal = self.get_ideal_size();

                if w >= ideal.x {
                    ideal
                } else {
                    let h = self.get_num_lines(w as usize) as u32;
                    Vec2::new(w, h)
                }
            },
            _ => unreachable!(),
        }
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.

        self.rows = LinesIterator::new(&self.content, size.x as usize).collect();
    }
}
