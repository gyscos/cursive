#[allow(deprecated)]
use crate::{
    direction::Direction,
    event::{Event, EventResult, Key, MouseButton, MouseEvent},
    rect::Rect,
    style::PaletteStyle,
    utils::lines::simple::{prefix, simple_prefix, LinesIterator, Row},
    view::{CannotFocus, ScrollBase, SizeCache, View},
    Vec2, {Printer, With, XY},
};
use log::debug;
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Multi-lines text editor.
///
/// A `TextArea` will attempt to grow vertically and horizontally
/// dependent on the content.  Wrap it in a `ResizedView` to
/// constrain its size.
///
/// # Examples
///
/// ```
/// use cursive_core::traits::{Nameable, Resizable};
/// use cursive_core::views::TextArea;
///
/// let text_area = TextArea::new()
///     .content("Write description here...")
///     .with_name("text_area")
///     .fixed_width(30)
///     .min_height(5);
/// ```
pub struct TextArea {
    // TODO: use a smarter data structure (rope?)
    content: String,

    /// Byte offsets within `content` representing text rows
    ///
    /// Invariant: never empty.
    rows: Vec<Row>,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Base for scrolling features
    #[allow(deprecated)]
    scrollbase: ScrollBase,

    /// Cache to avoid re-computing layout on no-op events
    size_cache: Option<XY<SizeCache>>,
    last_size: Vec2,

    /// Byte offset of the currently selected grapheme.
    cursor: usize,
}

fn make_rows(text: &str, width: usize) -> Vec<Row> {
    // We can't make rows with width=0, so force at least width=1.
    let width = usize::max(width, 1);
    LinesIterator::new(text, width).show_spaces().collect()
}

new_default!(TextArea);

impl TextArea {
    /// Creates a new, empty TextArea.
    pub fn new() -> Self {
        #[allow(deprecated)]
        TextArea {
            content: String::new(),
            rows: Vec::new(),
            enabled: true,
            scrollbase: ScrollBase::new().right_padding(0),
            size_cache: None,
            last_size: Vec2::zero(),
            cursor: 0,
        }
        .with(|area| area.compute_rows(Vec2::new(1, 1)))
        // Make sure we have valid rows, even for empty text.
    }

    /// Retrieves the content of the view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Ensures next layout call re-computes the rows.
    fn invalidate(&mut self) {
        self.size_cache = None;
    }

    /// Returns the position of the cursor in the content string.
    ///
    /// This is a byte index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Moves the cursor to the given byte position.
    ///
    /// # Panics
    ///
    /// This method panics if `cursor` is not the starting byte of a character in
    /// the content string.
    pub fn set_cursor(&mut self, cursor: usize) {
        // TODO: What to do if we fall inside a multi-codepoint grapheme?
        // Move back to the start of the grapheme?
        self.cursor = cursor;

        let focus = self.selected_row();
        self.scrollbase.scroll_to(focus);
    }

    /// Sets the content of the view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.content = content.into();

        // First, make sure we are within the bounds.
        self.cursor = min(self.cursor, self.content.len());

        // We have no guarantee cursor is now at a correct UTF8 location.
        // So look backward until we find a valid grapheme start.
        while !self.content.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }

        if let Some(size) = self.size_cache.map(|s| s.map(|s| s.value)) {
            self.invalidate();
            self.compute_rows(size);
        }
    }

    /// Sets the content of the view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn content<S: Into<String>>(self, content: S) -> Self {
        self.with(|s| s.set_content(content))
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Re-enables this view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn enabled(self) -> Self {
        self.with(Self::enable)
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Finds the row containing the grapheme at the given offset
    fn row_at(&self, byte_offset: usize) -> usize {
        debug!("Offset: {}", byte_offset);

        assert!(!self.rows.is_empty());
        assert!(byte_offset >= self.rows[0].start);

        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= byte_offset)
            .map(|(i, _)| i)
            .last()
            .unwrap()
    }

    fn col_at(&self, byte_offset: usize) -> usize {
        let row_id = self.row_at(byte_offset);
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        self.content[row.start..byte_offset].width()
    }

    /// Finds the row containing the cursor
    fn selected_row(&self) -> usize {
        assert!(!self.rows.is_empty(), "Rows should never be empty.");
        self.row_at(self.cursor)
    }

    fn selected_col(&self) -> usize {
        self.col_at(self.cursor)
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
        let offset = prefix(prev_text.graphemes(true), x, "").length;
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
        let offset = prefix(next_text.graphemes(true), x, "").length;
        self.cursor = next_row.start + offset;
    }

    /// Moves the cursor to the left.
    ///
    /// Wraps the previous line if required.
    fn move_left(&mut self) {
        let len = {
            // We don't want to utf8-parse the entire content.
            // So only consider the last row.
            let mut row = self.selected_row();
            if self.rows[row].start == self.cursor {
                row = row.saturating_sub(1);
            }

            let text = &self.content[self.rows[row].start..self.cursor];
            text.graphemes(true).last().unwrap().len()
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
        match self.size_cache {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }

    // If we are editing the text, we add a fake "space" character for the
    // cursor to indicate where the next character will appear.
    // If the current line is full, adding a character will overflow into the
    // next line. To show that, we need to add a fake "ghost" row, just for
    // the cursor.
    fn fix_ghost_row(&mut self) {
        if self.rows.is_empty() || self.rows.last().unwrap().end != self.content.len() {
            // Add a fake, empty row at the end.
            self.rows.push(Row {
                start: self.content.len(),
                end: self.content.len(),
                width: 0,
                is_wrapped: false,
            });
        }
    }

    fn soft_compute_rows(&mut self, size: Vec2) {
        if self.is_cache_valid(size) {
            debug!("Cache is still valid.");
            return;
        }
        debug!("Computing! Oh yeah!");

        let mut available = size.x;

        self.rows = make_rows(&self.content, available);
        self.fix_ghost_row();

        if self.rows.len() > size.y {
            available = available.saturating_sub(1);
            // Apparently we'll need a scrollbar. Doh :(
            self.rows = make_rows(&self.content, available);
            self.fix_ghost_row();
        }

        if !self.rows.is_empty() {
            self.size_cache = Some(SizeCache::build(size, size));
        }
    }

    fn compute_rows(&mut self, size: Vec2) {
        self.soft_compute_rows(size);
        self.scrollbase.set_heights(size.y, self.rows.len());
    }

    fn backspace(&mut self) {
        self.move_left();
        self.delete();
    }

    fn delete(&mut self) {
        if self.cursor == self.content.len() {
            return;
        }
        debug!("Rows: {:?}", self.rows);
        let len = self.content[self.cursor..]
            .graphemes(true)
            .next()
            .unwrap()
            .len();
        let start = self.cursor;
        let end = self.cursor + len;
        debug!("Start/end: {}/{}", start, end);
        debug!("Content: `{}`", self.content);
        for _ in self.content.drain(start..end) {}
        debug!("Content: `{}`", self.content);

        let selected_row = self.selected_row();
        debug!("Selected row: {}", selected_row);
        if self.cursor == self.rows[selected_row].end {
            // We're removing an (implicit) newline.
            // This means merging two rows.
            let new_end = self.rows[selected_row + 1].end;
            self.rows[selected_row].end = new_end;
            self.rows.remove(selected_row + 1);
        }
        self.rows[selected_row].end -= len;

        // update all the rows downstream
        for row in &mut self.rows.iter_mut().skip(1 + selected_row) {
            row.rev_shift(len);
        }
        debug!("Rows: {:?}", self.rows);

        self.fix_damages();
        debug!("Rows: {:?}", self.rows);
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
        self.fix_damages();
    }

    /// Fix a damage located at the cursor.
    ///
    /// The only damages are assumed to have occurred around the cursor.
    ///
    /// This is an optimization to not re-compute the entire rows when an
    /// insert happened.
    fn fix_damages(&mut self) {
        if self.size_cache.is_none() {
            // If we don't know our size, we'll get a layout command soon.
            // So no need to do that here.
            return;
        }

        let size = self.size_cache.unwrap().map(|s| s.value);

        // Find affected text.
        // We know the damage started at this row, so it'll need to go.
        //
        // Actually, if possible, also re-compute the previous row.
        // Indeed, the previous row may have been cut short, and if we now
        // break apart a big word, maybe the first half can go up one level.
        let first_row = self.selected_row().saturating_sub(1);

        let first_byte = self.rows[first_row].start;

        // We don't need to go beyond a newline.
        // If we don't find one, end of the text it is.
        debug!("Cursor: {}", self.cursor);
        let last_byte = self.content[self.cursor..]
            .find('\n')
            .map(|i| 1 + i + self.cursor);
        let last_row = last_byte.map_or(self.rows.len(), |last_byte| self.row_at(last_byte));
        let last_byte = last_byte.unwrap_or(self.content.len());

        debug!("Content: `{}` (len={})", self.content, self.content.len());
        debug!("start/end: {}/{}", first_byte, last_byte);
        debug!("start/end rows: {}/{}", first_row, last_row);

        // Do we have access to the entire width?...
        let mut available = size.x;

        let scrollable = self.rows.len() > size.y;
        if scrollable {
            // ... not if a scrollbar is there
            available = available.saturating_sub(1);
        }

        // First attempt, if scrollbase status didn't change.
        debug!("Rows: {:?}", self.rows);
        let new_rows = make_rows(&self.content[first_byte..last_byte], available);
        // How much did this add?
        debug!("New rows: {:?}", new_rows);
        debug!("{}-{}", first_row, last_row);
        let new_row_count = self.rows.len() + new_rows.len() + first_row - last_row;
        if !scrollable && new_row_count > size.y {
            // We just changed scrollable status.
            // This changes everything.
            // TODO: compute_rows() currently makes a scroll-less attempt.
            // Here, we know it's just no gonna happen.
            self.invalidate();
            self.compute_rows(size);
            return;
        }

        // Otherwise, replace stuff.
        let affected_rows = first_row..last_row;
        let replacement_rows = new_rows.into_iter().map(|row| row.shifted(first_byte));
        self.rows.splice(affected_rows, replacement_rows);
        self.fix_ghost_row();
        self.scrollbase.set_heights(size.y, self.rows.len());
    }
}

impl View for TextArea {
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // Make sure our structure is up to date
        self.soft_compute_rows(constraint);

        // Ideally, we'd want x = the longest row + 1
        // (we always keep a space at the end)
        // And y = number of rows
        debug!("{:?}", self.rows);
        let scroll_width = usize::from(self.rows.len() > constraint.y);

        let content_width = if self.rows.iter().any(|row| row.is_wrapped) {
            // If any row has been wrapped, we want to take the full width.
            constraint.x.saturating_sub(1 + scroll_width)
        } else {
            self.rows.iter().map(|r| r.width).max().unwrap_or(1)
        };

        Vec2::new(scroll_width + 1 + content_width, self.rows.len())
    }

    fn draw(&self, printer: &Printer) {
        let (style, cursor_style) = if self.enabled && printer.enabled {
            (PaletteStyle::EditableText, PaletteStyle::EditableTextCursor)
        } else {
            (
                PaletteStyle::EditableTextInactive,
                PaletteStyle::EditableTextInactive,
            )
        };

        let w = if self.scrollbase.scrollable() {
            printer.size.x.saturating_sub(1)
        } else {
            printer.size.x
        };
        printer.with_style(style, |printer| {
            for y in 0..printer.size.y {
                printer.print_hline((0, y), w, " ");
            }
        });

        debug!("Content: `{}`", &self.content);
        self.scrollbase.draw(printer, |printer, i| {
            debug!("Drawing row {}", i);
            let row = &self.rows[i];
            debug!("row: {:?}", row);
            let text = &self.content[row.start..row.end];
            debug!("row text: `{}`", text);
            printer.with_style(style, |printer| {
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
                printer.with_style(cursor_style, |printer| {
                    printer.print((offset, 0), c);
                });
            }
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        let mut fix_scroll = true;
        match event {
            Event::Char(ch) => self.insert(ch),
            Event::Key(Key::Enter) => self.insert('\n'),
            Event::Key(Key::Backspace) if self.cursor > 0 => self.backspace(),
            Event::Key(Key::Del) if self.cursor < self.content.len() => self.delete(),

            Event::Key(Key::End) => {
                let row = self.selected_row();
                self.cursor = self.rows[row].end;
                if row + 1 < self.rows.len() && self.cursor == self.rows[row + 1].start {
                    self.move_left();
                }
            }
            Event::Ctrl(Key::Home) => self.cursor = 0,
            Event::Ctrl(Key::End) => self.cursor = self.content.len(),
            Event::Key(Key::Home) => self.cursor = self.rows[self.selected_row()].start,
            Event::Key(Key::Up) if self.selected_row() > 0 => self.move_up(),
            Event::Key(Key::Down) if self.selected_row() + 1 < self.rows.len() => self.move_down(),
            Event::Key(Key::PageUp) => self.page_up(),
            Event::Key(Key::PageDown) => self.page_down(),
            Event::Key(Key::Left) if self.cursor > 0 => self.move_left(),
            Event::Key(Key::Right) if self.cursor < self.content.len() => self.move_right(),
            Event::Mouse {
                event: MouseEvent::WheelUp,
                ..
            } if self.scrollbase.can_scroll_up() => {
                fix_scroll = false;
                self.scrollbase.scroll_up(5);
            }
            Event::Mouse {
                event: MouseEvent::WheelDown,
                ..
            } if self.scrollbase.can_scroll_down() => {
                fix_scroll = false;
                self.scrollbase.scroll_down(5);
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| self.scrollbase.start_drag(position, self.last_size.x))
                .unwrap_or(false) =>
            {
                fix_scroll = false;
            }
            Event::Mouse {
                event: MouseEvent::Hold(MouseButton::Left),
                position,
                offset,
            } => {
                fix_scroll = false;
                let position = position.saturating_sub(offset);
                self.scrollbase.drag(position);
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if !self.rows.is_empty() && position.fits_in_rect(offset, self.last_size) => {
                if let Some(position) = position.checked_sub(offset) {
                    #[allow(deprecated)]
                    let y = position.y + self.scrollbase.start_line;
                    let y = min(y, self.rows.len() - 1);
                    let x = position.x;
                    let row = &self.rows[y];
                    let content = &self.content[row.start..row.end];

                    self.cursor = row.start + simple_prefix(content, x).length;
                }
            }
            _ => return EventResult::Ignored,
        }

        debug!("Rows: {:?}", self.rows);
        if fix_scroll {
            let focus = self.selected_row();
            self.scrollbase.scroll_to(focus);
        }

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
        self.compute_rows(size);
    }

    fn important_area(&self, _: Vec2) -> Rect {
        // The important area is a single character
        let char_width = if self.cursor >= self.content.len() {
            // If we're are the end of the content, it'll be a space
            1
        } else {
            // Otherwise it's the selected grapheme
            self.content[self.cursor..]
                .graphemes(true)
                .next()
                .unwrap()
                .width()
        };

        Rect::from_size((self.selected_col(), self.selected_row()), (char_width, 1))
    }
}

#[crate::blueprint(TextArea::new())]
struct Blueprint {
    content: Option<String>,
}
