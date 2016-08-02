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
            scrollbase: ScrollBase::new(),
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
        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= offset)
            .map(|(i, _)| i)
            .last()
            .unwrap()
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

        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        let x = self.content[row.start..self.cursor].width();

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
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        let x = self.content[row.start..self.cursor].width();

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

    fn compute_rows(&mut self, size: Vec2) {
        let mut available = size.x;
        let content = format!("{} ", self.content);
        self.rows = make_rows(&content, available);
        if self.rows.len() > size.y {
            available -= 1;
            // Doh :(
            self.rows = make_rows(&content, available);
        }


        if !self.rows.is_empty() {
            // The last row probably contains a fake whitespace.
            // Unless... the whitespace was used as an implicit newline.
            // This means the last row ends in a newline-d whitespace.
            // How do we detect that?
            // By checking if the last row takes all the available width.
            if self.rows.last().unwrap().width != available {
                self.rows.last_mut().unwrap().end -= 1;
            }
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

        let size = self.last_size.unwrap().map(|s| s.value);
        self.compute_rows(size);
    }

    fn insert(&mut self, ch: char) {
        let cursor = self.cursor;
        self.content.insert(cursor, ch);

        let shift = ch.len_utf8();
        let selected_row = self.selected_row();
        self.rows[selected_row].end += shift;

        if selected_row < self.rows.len() {
            for row in &mut self.rows[1 + selected_row..] {
                row.start += shift;
                row.end += shift;
            }
        }

        let size = self.last_size.unwrap().map(|s| s.value);
        self.compute_rows(size);
        self.cursor += shift;
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
            Event::Ctrl(Key::Home) => {
                self.cursor = 0
            }
            Event::Ctrl(Key::End) => {
                self.cursor = self.content.len()
            }
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
