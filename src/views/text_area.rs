use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use odds::vec::VecExt;

use std::rc::Rc;

use {Printer, XY};
use direction::Direction;
use vec::Vec2;
use event::{Event, EventResult, Key};
use utils::{LinesIterator, Row, prefix_length};
use view::{ScrollBase, SizeCache, View};
use theme::{ColorStyle, Effect};

/// Multi-lines text editor.
///
/// A `TextArea` by itself doesn't have a well-defined size.
/// You should wrap it in a `BoxView` to control its size.
pub struct TextArea {
    // TODO: use a smarter data structure (rope?)
    content: String,
    /// Byte offsets within `content` representing text rows
    rows: Vec<Row>,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Base for scrolling features
    scrollbase: ScrollBase,

    /// Cache to avoid re-computing layout on no-op events
    last_size: Option<XY<SizeCache>>,

    /// Byte offset of the currently selected grapheme.
    cursor: usize,
}

fn make_rows(text: &str, width: usize) -> Vec<Row> {
    LinesIterator::new(text, width)
        .show_spaces()
        .collect()
}

impl TextArea {
    /// Creates a new, empty TextArea.
    pub fn new() -> Self {
        TextArea {
            content: String::new(),
            rows: vec![Row {
                           start: 0,
                           end: 0,
                           width: 0,
                       }],
            enabled: true,
            scrollbase: ScrollBase::new().right_padding(0),
            last_size: None,
            cursor: 0,
        }
    }

    /// Retrieves the content of the view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Finds the row containing the grapheme at the given offset
    fn row_at(&self, offset: usize) -> usize {
        // println_stderr!("Offset: {}", offset);
        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= offset)
            .map(|(i, _)| i)
            .last()
            .unwrap()
    }

    fn col_at(&self, offset: usize) -> usize {
        let row_id = self.row_at(offset);
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        self.content[row.start..offset].width()
    }

    /// Finds the row containing the cursor
    fn selected_row(&self) -> usize {
        self.row_at(self.cursor)
    }

    fn page_up(&mut self) {
        for _ in 0..5 {
            self.move_up();
        }
    }

    fn page_down(&mut self) {
        for _ in 0..5 {
            self.move_down();
        }
    }

    fn move_up(&mut self) {
        let row_id = self.selected_row();
        if row_id == 0 {
            return;
        }

        // Number of cells to the left of the cursor
        let x = self.col_at(self.cursor);

        let prev_row = self.rows[row_id - 1];
        let prev_text = &self.content[prev_row.start..prev_row.end];
        let offset = prefix_length(prev_text.graphemes(true), x, "");
        self.cursor = prev_row.start + offset;
    }

    fn move_down(&mut self) {
        let row_id = self.selected_row();
        if row_id + 1 == self.rows.len() {
            return;
        }
        let x = self.col_at(self.cursor);

        let next_row = self.rows[row_id + 1];
        let next_text = &self.content[next_row.start..next_row.end];
        let offset = prefix_length(next_text.graphemes(true), x, "");
        self.cursor = next_row.start + offset;
    }

    /// Moves the cursor to the left.
    ///
    /// Wraps the previous line if required.
    fn move_left(&mut self) {
        let len = {
            // We don't want to utf8-parse the entire content.
            // So restrict to the last row.
            let mut row = self.selected_row();
            if self.rows[row].start == self.cursor {
                row -= 1;
            }

            let text = &self.content[self.rows[row].start..self.cursor];
            text.graphemes(true)
                .last()
                .unwrap()
                .len()
        };
        self.cursor -= len;
    }

    /// Moves the cursor to the right.
    ///
    /// Jumps to the next line is required.
    fn move_right(&mut self) {
        let len = self.content[self.cursor..]
            .graphemes(true)
            .next()
            .unwrap()
            .len();
        self.cursor += len;
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        false
    }

    fn fix_ghost_row(&mut self) {
        if self.rows.is_empty() ||
           self.rows.last().unwrap().end != self.content.len() {
            // Add a fake, empty row at the end.
            self.rows.push(Row {
                start: self.content.len(),
                end: self.content.len(),
                width: 0,
            });
        }
    }

    fn compute_rows(&mut self, size: Vec2) {
        let mut available = size.x;
        self.rows = make_rows(&self.content, available);
        self.fix_ghost_row();
        if self.rows.len() > size.y {
            available -= 1;
            // Doh :(
            self.rows = make_rows(&self.content, available);
            self.fix_ghost_row();
        }

        if !self.rows.is_empty() {
            self.last_size = Some(SizeCache::build(size, size));
        }
    }

    fn backspace(&mut self) {
        if self.cursor != 0 {
            self.move_left();
            self.delete();
        }
    }

    fn delete(&mut self) {
        let len = self.content[self.cursor..]
            .graphemes(true)
            .next()
            .unwrap()
            .len();
        let start = self.cursor;
        let end = self.cursor + len;
        for _ in self.content.drain(start..end) {}

        let selected_row = self.selected_row();
        if self.cursor == self.rows[selected_row].end {
            // We're removing an (implicit) newline.
            // This means merging two rows.
        } else {
            self.rows[selected_row].end -= len;
        }

        // update all the rows downstream
        for row in &mut self.rows.iter_mut().skip(1 + selected_row) {
            row.rev_shift(len);
        }

        self.fix_damages(true);
    }

    fn insert(&mut self, ch: char) {

        // First, we inject the data, but keep the cursor unmoved
        // (So the cursor is to the left of the injected char)
        self.content.insert(self.cursor, ch);

        // Then, we shift the indexes of every row after this one.
        let shift = ch.len_utf8();

        // The current row grows, every other is just shifted.
        let selected_row = self.selected_row();
        self.rows[selected_row].end += shift;

        for row in &mut self.rows.iter_mut().skip(1 + selected_row) {
            row.shift(shift);
        }
        self.cursor += shift;

        // Finally, rows may not have the correct width anymore, so fix them.
        self.fix_damages(false);

    }

    /// Fix a damage located at the cursor.
    ///
    /// The only damages are assumed to have occured around the cursor.
    fn fix_damages(&mut self, delete: bool) {
        let size = self.last_size.unwrap().map(|s| s.value);

        // Find affected text.
        let first_row = self.selected_row();
        let start = self.rows[first_row].start;

        // We don't need to go beyond a newline.
        // If we don't find one, end of the text it is.
        // println_stderr!("Cursor: {}", self.cursor);
        let end = self.content[self.cursor..]
            .find('\n')
            .map(|i| i + self.cursor)
            .unwrap_or(self.content.len());
        // println_stderr!("Content: `{}` (len={})", self.content, self.content.len());
        // println_stderr!("start/end: {}/{}", start, end);
        let last_row = self.row_at(end);

        // Do we have access to the entire width?...
        let mut available = size.x;

        let scrollable = self.rows.len() > size.y;
        if scrollable {
            // ... not if a scrollbar is there
            available -= 1;
        }

        // First attempt, if scrollbase status didn't change.
        // println_stderr!("Rows: {:?}", self.rows);
        let mut new_rows = make_rows(&self.content[start..end], available);
        // How much did this add?
        let new_row_count = if delete {
            let delta = (1 + last_row - first_row) - new_rows.len();
            self.rows.len() - delta
        } else {
            let delta = new_rows.len() - (1 + last_row - first_row);
            self.rows.len() + delta

        };
        if !scrollable && new_row_count > size.y {
            // We just changed scrollable status.
            // This changes everything.
            // TODO: compute_rows() currently makes a scroll-less attempt.
            // Here, we know it's just no gonna happen.
            self.compute_rows(size);
            return;
        }

        // Otherwise, replace stuff.
        let affected_rows = first_row..(1 + last_row);
        let replacement_rows = new_rows.into_iter()
            .map(|row| row.shifted(start));
        self.rows.splice(affected_rows, replacement_rows);
        self.fix_ghost_row();
    }
}

impl View for TextArea {
    fn draw(&self, printer: &Printer) {
        printer.with_color(ColorStyle::Secondary, |printer| {
            let effect = if self.enabled {
                Effect::Reverse
            } else {
                Effect::Simple
            };

            let w = if self.scrollbase.scrollable() {
                printer.size.x - 1
            } else {
                printer.size.x
            };
            printer.with_effect(effect, |printer| {
                for y in 0..printer.size.y {
                    printer.print_hline((0, y), w, " ");
                }
            });

            self.scrollbase.draw(printer, |printer, i| {
                let row = &self.rows[i];
                let text = &self.content[row.start..row.end];
                printer.with_effect(effect, |printer| {
                    printer.print((0, 0), text);
                });

                if printer.focused && i == self.selected_row() {
                    let cursor_offset = self.cursor - row.start;
                    let c = if cursor_offset == text.len() {
                        "_"
                    } else {
                        text[cursor_offset..]
                            .graphemes(true)
                            .next()
                            .expect("Found no char!")
                    };
                    let offset = text[..cursor_offset].width();
                    printer.print((offset, 0), c);
                }

            });
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Char(ch) => self.insert(ch),
            Event::Key(Key::Enter) => self.insert('\n'),
            Event::Key(Key::Backspace) if self.cursor > 0 => self.backspace(),
            Event::Key(Key::Del) if self.cursor < self.content.len() => {
                self.delete()
            }

            Event::Key(Key::End) => {
                let row = self.selected_row();
                self.cursor = self.rows[row].end;
                if row + 1 < self.rows.len() &&
                   self.cursor == self.rows[row + 1].start {
                    self.move_left();
                }
            }
            Event::Ctrl(Key::Home) => self.cursor = 0,
            Event::Ctrl(Key::End) => self.cursor = self.content.len(),
            Event::Key(Key::Home) => {
                self.cursor = self.rows[self.selected_row()].start
            }
            Event::Key(Key::Up) if self.selected_row() > 0 => self.move_up(),
            Event::Key(Key::Down) if self.selected_row() + 1 <
                                     self.rows.len() => self.move_down(),
            Event::Key(Key::PageUp) => self.page_up(),
            Event::Key(Key::PageDown) => self.page_down(),
            Event::Key(Key::Left) if self.cursor > 0 => self.move_left(),
            Event::Key(Key::Right) if self.cursor < self.content.len() => {
                self.move_right()
            }
            _ => return EventResult::Ignored,
        }

        // println_stderr!("Rows: {:?}", self.rows);
        let focus = self.selected_row();
        self.scrollbase.scroll_to(focus);

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = Some(SizeCache::build(size, size));
        self.scrollbase.set_heights(size.y, self.rows.len());
    }
}
