use vec2::Vec2;
use view::{View,DimensionRequest,SizeRequest};
use div::*;
use printer::Printer;

/// A simple view showing a fixed text
pub struct TextView {
    content: String,
}

/// Returns the number of lines required to display the given text with the
/// specified maximum line width.
fn get_line_span(line: &str, max_width: usize) -> usize {
    let mut lines = 1;
    let mut length = 0;
    for l in line.split(" ").map(|word| word.len()) {
        length += l;
        if length > max_width {
            length = l;
            lines += 1;
        }
    }
    lines
}

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new(content: &str) -> Self {
        TextView {
            content: content.to_string(),
        }
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
}

struct LinesIterator<'a> {
    line: &'a str,
    start: usize,
    width: usize,
}

impl <'a> LinesIterator<'a> {
    fn new(line: &'a str, width: usize) -> Self {
        if line.len() == 0 {
            LinesIterator {
                line: " ",
                width: width,
                start: 0,
            }
        } else {
            LinesIterator {
                line: line,
                width: width,
                start: 0,
            }
        }
    }
}

impl <'a> Iterator for LinesIterator<'a> {

    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.start >= self.line.len() {
            None
        } else if self.start + self.width >= self.line.len() {
            let start = self.start;
            self.start = self.line.len();
            Some(&self.line[start..])
        } else {
            let start = self.start;
            let (end,skip_space) = match self.line[start..start+self.width].rfind(" ") {
                // Hard break
                None => (start + self.width, false),
                Some(i) => (start+i, true),
            };
            self.start = end;
            if skip_space {
                self.start += 1;
            }
            Some(&self.line[start..end])
        }
    }
}

impl View for TextView {
    fn draw(&self, printer: &Printer) {
        let lines = self.content.split("\n")
            .flat_map(|line| LinesIterator::new(line, printer.size.x as usize));
        for (i, line) in lines.enumerate() {
            printer.print((0,i), line);
        }

    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        match (size.w,size.h) {
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
                if w >= self.content.len() as u32 {
                    Vec2::new(self.content.len() as u32, 1)
                } else {
                    let h = self.get_num_lines(w as usize) as u32;
                    Vec2::new(w, h)
                }
            },
            _ => unreachable!(),
        }
    }
}
