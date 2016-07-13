use XY;
use vec::Vec2;
use view::View;
use view::SizeCache;
use printer::Printer;
use align::*;
use event::*;
use super::scroll::ScrollBase;

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;


/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,

    align: Align,

    // ScrollBase make many scrolling-related things easier
    scrollbase: ScrollBase,
    last_size: Option<XY<SizeCache>>,
    width: Option<usize>,
}

// Subset of the main content representing a row on the display.
struct Row {
    start: usize,
    end: usize,
    width: usize,
}

// If the last character is a newline, strip it.
fn strip_last_newline(content: &str) -> &str {
    if !content.is_empty() && content.chars().last().unwrap() == '\n' {
        &content[..content.len() - 1]
    } else {
        content
    }
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
            last_size: None,
            width: None,
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
        self.invalidate();
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.last_size {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }

    fn compute_rows(&mut self, size: Vec2) {
        if !self.is_cache_valid(size) {
            // Recompute
            self.rows = LinesIterator::new(&self.content, size.x).collect();
            let mut scrollbar = 0;
            if self.rows.len() > size.y {
                scrollbar = 2;
                // If we're too high, include a scrollbar
                self.rows = LinesIterator::new(&self.content,
                                               size.x - scrollbar)
                    .collect();
            }

            self.width = self.rows
                .iter()
                .map(|row| row.width)
                .max()
                .map(|w| w + scrollbar);

            // Our resulting size.
            let my_size =
                size.or_min((self.width.unwrap_or(0), self.rows.len()));
            self.last_size = Some(SizeCache::build(my_size, size));
        }
    }

    // Invalidates the cache, so next call will recompute everything.
    fn invalidate(&mut self) {
        self.last_size = None;
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

        let next = content.find('\n').unwrap_or(content.len());
        let content = &content[..next];

        let line_width = content.width();
        if line_width <= self.width {
            // We found a newline before the allowed limit.
            // Break early.
            self.start += next + 1;
            return Some(Row {
                start: start,
                end: next + start,
                width: line_width,
            });
        }

        // Keep adding indivisible tokens
        let head_bytes =
            match head_bytes(content.split(' '), self.width, " ") {
                0 => head_bytes(content.graphemes(true), self.width, ""),
                other => {
                    self.start += 1;
                    other
                }
            };

        self.start += head_bytes;

        Some(Row {
            start: start,
            end: start + head_bytes,
            width: self.width,
        })
    }
}

impl View for TextView {
    fn draw(&mut self, printer: &Printer) {

        let h = self.rows.len();
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer =
            &printer.sub_printer(Vec2::new(0, offset), printer.size, true);

        self.scrollbase.draw(printer, |printer, i| {
            let row = &self.rows[i];
            let text = &self.content[row.start..row.end];
            let l = text.width();
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
            Event::Key(Key::Up) if self.scrollbase.can_scroll_up() => {
                self.scrollbase.scroll_up(1)
            }
            Event::Key(Key::Down) if self.scrollbase
                .can_scroll_down() => self.scrollbase.scroll_down(1),
            Event::Key(Key::PageDown) => self.scrollbase.scroll_down(10),
            Event::Key(Key::PageUp) => self.scrollbase.scroll_up(10),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(None)
    }

    fn needs_relayout(&self) -> bool {
        self.last_size.is_none()
    }

    fn get_min_size(&mut self, size: Vec2) -> Vec2 {
        self.compute_rows(size);
        size.or_min((self.width.unwrap_or(0), self.rows.len()))
    }

    fn take_focus(&mut self) -> bool {
        self.scrollbase.scrollable()
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.compute_rows(size);
        self.scrollbase.set_heights(size.y, self.rows.len());
    }
}

fn head_bytes<'a, I: Iterator<Item = &'a str>>(iter: I, width: usize,
                                               overhead: &str)
                                               -> usize {
    let overhead_width = overhead.width();
    let overhead_len = overhead.len();

    let sum = iter.scan(0, |w, token| {
            *w += token.width();
            if *w > width {
                None
            } else {
                // Add a space
                *w += overhead_width;
                Some(token)
            }
        })
        .map(|token| token.len() + overhead_len)
        .fold(0, |a, b| a + b);

    // We counted overhead_len once too many times,
    // but only if the iterator was non empty.
    if sum == 0 {
        sum
    } else {
        sum - overhead_len
    }
}
