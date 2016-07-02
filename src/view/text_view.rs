use std::cmp::max;

use vec::Vec2;
use view::View;
use printer::Printer;
use align::*;
use event::*;
use super::scroll::ScrollBase;

/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,

    align: Align,

    // ScrollBase make many scrolling-related things easier
    scrollbase: ScrollBase,
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
    for l in line.split(' ').map(|word| word.chars().count()) {
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
            rows: Vec::new(),
            scrollbase: ScrollBase::new(),
            align: Align::top_left(),
        }
    }

    /// Sets the horizontal alignment for this view.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Sets the vertical alignment for this view.
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;

        self
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
        self.content
            .split('\n')
            .map(|line| get_line_span(line, max_width))
            .fold(0, |sum, x| sum + x)
    }

    // In the absence of any constraint, what size would we like?
    fn get_ideal_size(&self) -> Vec2 {
        let mut max_width = 0;
        let mut height = 0;

        for line in self.content.split('\n') {
            height += 1;
            max_width = max(max_width, line.chars().count());
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

impl<'a> LinesIterator<'a> {
    // Start an iterator on the given content.
    fn new(content: &'a str, width: usize) -> Self {
        LinesIterator {
            content: content,
            width: width,
            start: 0,
        }
    }
}

impl<'a> Iterator for LinesIterator<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.start >= self.content.len() {
            // This is the end.
            return None;
        }

        let start = self.start;
        let content = &self.content[self.start..];

        if let Some(next) = content.find('\n') {
            if content[..next].chars().count() <= self.width {
                // We found a newline before the allowed limit.
                // Break early.
                self.start += next + 1;
                return Some(Row {
                    start: start,
                    end: next + start,
                });
            }
        }

        let content_len = content.chars().count();
        if content_len <= self.width {
            // I thought it would be longer! -- that's what she said :(
            self.start += content.len();
            return Some(Row {
                start: start,
                end: start + content.len(),
            });
        }

        let i = if content_len == self.width + 1 {
            // We can't look at the index
            // if we're looking at the end of the string
            content.len()
        } else {
            content.char_indices().nth(self.width + 1).unwrap().0
        };
        let substr = &content[..i];
        if let Some(i) = substr.rfind(' ') {
            // If we have to break, try to find a whitespace for that.
            self.start += i + 1;
            return Some(Row {
                start: start,
                end: i + start,
            });
        }

        // Meh, no whitespace, so just cut in this mess.
        // TODO: look for ponctuation instead?
        self.start += self.width;

        Some(Row {
            start: start,
            end: start + self.width,
        })
    }
}

impl View for TextView {
    fn draw(&mut self, printer: &Printer) {

        let h = self.rows.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

        self.scrollbase.draw(printer, |printer, i| {
            let row = &self.rows[i];
            let text = &self.content[row.start..row.end];
            let l = text.chars().count();
            let x = self.align.h.get_offset(l, printer.size.x);
            printer.print((x, 0), text);
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.scrollbase.scrollable() {
            return EventResult::Ignored;
        }

        match event {
            Event::Key(Key::Home) => self.scrollbase.scroll_top(),
            Event::Key(Key::End) => self.scrollbase.scroll_bottom(),
            Event::Key(Key::Up) if self.scrollbase.can_scroll_up() => self.scrollbase.scroll_up(1),
            Event::Key(Key::Down) if self.scrollbase
                                         .can_scroll_down() => self.scrollbase.scroll_down(1),
            Event::Key(Key::PageDown) => self.scrollbase.scroll_down(10),
            Event::Key(Key::PageUp) => self.scrollbase.scroll_up(10),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }

    fn get_min_size(&self, size: Vec2) -> Vec2 {
        // If we have no directive, ask for a single big line.
        // TODO: what if the text has newlines??
        // Don't _force_ the max width, but take it if we have to.
        let ideal = self.get_ideal_size();

        if size.x >= ideal.x {
            ideal
        } else {
            // Ok, se we have less width than we'd like.
            // Take everything we can, and plan our height accordingly.
            let h = self.get_num_lines(size.x);
            Vec2::new(size.x, h)
        }
    }

    fn take_focus(&mut self) -> bool {
        self.scrollbase.scrollable()
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.rows = LinesIterator::new(&self.content, size.x).collect();
        if self.rows.len() > size.y {
            self.rows = LinesIterator::new(&self.content, size.x - 2).collect();
        }
        self.scrollbase.set_heights(size.y, self.rows.len());
    }
}
