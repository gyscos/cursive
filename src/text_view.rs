use ncurses;

use super::Size;
use view::{View,DimensionRequest,SizeRequest};
use div::*;

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

impl View for TextView {
    fn draw(&self, win: ncurses::WINDOW, size: Size) {
    }

    fn get_min_size(&self, size: SizeRequest) -> Size {
        match (size.w,size.h) {
            (DimensionRequest::Unknown, DimensionRequest::Unknown) => Size::new(self.content.len() as u32, 1),
            (DimensionRequest::Fixed(w),_) => {
                let h = self.get_num_lines(w as usize) as u32;
                Size::new(w, h)
            },
            (_,DimensionRequest::Fixed(h)) => {
                let w = self.get_num_cols(h as usize) as u32;
                Size::new(w, h)
            },
            (DimensionRequest::AtMost(_),DimensionRequest::AtMost(_)) => unreachable!(),
            _ => unreachable!(),
        }
    }
}
