#[allow(deprecated)]
use crate::{
    Vec2,
    direction::Direction,
    event::{Event, EventResult, Key, MouseButton, MouseEvent},
    rect::Rect,
    style::{PaletteStyle, StyleType},
    utils::lines::simple::{LinesIterator, Row, prefix, simple_prefix},
    view::{CannotFocus, ScrollBase, View},
    {Printer, With},
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

    // Changes whenever `content` does.
    version: u64,

    /// Byte offsets within `content` representing text rows
    ///
    /// Invariant: never empty.
    rows: Vec<Row>,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Base for scrolling features
    #[allow(deprecated)]
    scrollbase: ScrollBase,
    /// The size `rows` were computed for.
    rows_size: Option<Vec2>,

    /// Whether `rows` leave a column for a scrollbar, and if so, how many
    /// rows the text would take without it (which decides if we need it):
    /// not counting the ghost row, and whether there would be one.
    scrollbar: Option<(usize, bool)>,

    /// Whether the last row of `rows` is the "ghost" row for the cursor.
    ghost_row: bool,

    /// Sizes computed for other requests (for `sizes_version`), so
    /// `required_size` doesn't have to wrap the text again, nor touch `rows`.
    sizes: Vec<(Vec2, Vec2)>,
    sizes_version: u64,
    last_size: Vec2,

    /// Byte offset of the currently selected grapheme.
    cursor: usize,

    /// Style used for the text when the view is enabled.
    regular_style: StyleType,

    /// Style used for the text when the view is disabled.
    inactive_style: StyleType,

    /// Style used for the cursor.
    cursor_style: StyleType,
}

fn make_rows(text: &str, width: usize) -> Vec<Row> {
    // We can't make rows with width=0, so force at least width=1.
    let width = usize::max(width, 1);
    LinesIterator::new(text, width).show_spaces().collect()
}

// If we are editing the text, we add a fake "space" character for the cursor
// to indicate where the next character will appear. If the current line is
// full, adding a character will overflow into the next line. To show that,
// we need to add a fake "ghost" row, just for the cursor.
//
// Returns whether a ghost row was added.
fn add_ghost_row(rows: &mut Vec<Row>, content_len: usize) -> bool {
    if rows.last().is_none_or(|row| row.end != content_len) {
        rows.push(Row {
            start: content_len,
            end: content_len,
            width: 0,
            is_wrapped: false,
        });
        true
    } else {
        false
    }
}

new_default!(TextArea);

impl TextArea {
    /// Creates a new, empty TextArea.
    pub fn new() -> Self {
        #[allow(deprecated)]
        TextArea {
            content: String::new(),
            version: crate::view::fresh_layout_key(),
            rows: Vec::new(),
            enabled: true,
            scrollbase: ScrollBase::new().right_padding(0),
            rows_size: None,
            scrollbar: None,
            ghost_row: false,
            sizes: Vec::new(),
            sizes_version: 0,
            last_size: Vec2::zero(),
            cursor: 0,
            regular_style: PaletteStyle::EditableText.into(),
            inactive_style: PaletteStyle::EditableTextInactive.into(),
            cursor_style: PaletteStyle::EditableTextCursor.into(),
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
        self.rows_size = None;
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
        self.version = crate::view::fresh_layout_key();

        // First, make sure we are within the bounds.
        self.cursor = min(self.cursor, self.content.len());

        // We have no guarantee cursor is now at a correct UTF8 location.
        // So look backward until we find a valid grapheme start.
        while !self.content.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }

        self.refresh_rows();
    }

    /// Sets the content of the view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn content<S: Into<String>>(self, content: S) -> Self {
        self.with(|s| s.set_content(content))
    }

    /// Sets the style used for the text.
    ///
    /// Defaults to `PaletteStyle::EditableText`.
    pub fn set_style<S: Into<StyleType>>(&mut self, style: S) {
        self.regular_style = style.into();
    }

    /// Sets the style used for the text.
    ///
    /// Chainable variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::views::TextArea;
    /// # use cursive_core::style::PaletteColor;
    /// let text_area = TextArea::new().style(PaletteColor::Primary);
    /// ```
    #[must_use]
    pub fn style<S: Into<StyleType>>(self, style: S) -> Self {
        self.with(|s| s.set_style(style))
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

        // Text that can't fit at all (wider than a whole row) is left out
        // of the rows: an offset there counts as the first row.
        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= byte_offset)
            .map(|(i, _)| i)
            .last()
            .unwrap_or(0)
    }

    fn col_at(&self, byte_offset: usize) -> usize {
        let row_id = self.row_at(byte_offset);
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        self.content
            .get(row.start..byte_offset)
            .map_or(0, UnicodeWidthStr::width)
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
        // Looking backward only parses the end of the text.
        let len = self.content[..self.cursor]
            .graphemes(true)
            .next_back()
            .map_or(0, str::len);
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

    // The rows for the given size (with a column for the scrollbar if
    // needed), with the scrollbar and ghost row status.
    fn rows_for(&self, size: Vec2) -> (Vec<Row>, Option<(usize, bool)>, bool) {
        let mut rows = make_rows(&self.content, size.x);
        let ghost_row = add_ghost_row(&mut rows, self.content.len());

        if rows.len() > size.y {
            // Apparently we'll need a scrollbar. Doh :(
            let full_rows = (rows.len() - usize::from(ghost_row), ghost_row);
            let mut rows = make_rows(&self.content, size.x.saturating_sub(1));
            let ghost_row = add_ghost_row(&mut rows, self.content.len());
            return (rows, Some(full_rows), ghost_row);
        }

        (rows, None, ghost_row)
    }

    fn compute_rows(&mut self, size: Vec2) {
        if self.rows_size != Some(size) {
            (self.rows, self.scrollbar, self.ghost_row) = self.rows_for(size);
            self.rows_size = Some(size);
        }
        self.scrollbase.set_heights(size.y, self.rows.len());
    }

    // Replaces `range` in the content with `replacement`, and updates rows.
    //
    // Rows of a paragraph (up to a line break) only depend on that
    // paragraph: only the one around the edit is wrapped again, and the
    // following rows are shifted.
    fn edit(&mut self, range: std::ops::Range<usize>, replacement: &str) {
        let Some(size) = self.rows_size else {
            // No rows yet: we'll compute them on layout.
            self.content.replace_range(range, replacement);
            self.version = crate::view::fresh_layout_key();
            return;
        };

        // The paragraph around the edit, in the current content.
        let para_start = self.content[..range.start].rfind('\n').map_or(0, |i| i + 1);
        let old_para_end = self.content[range.end..]
            .find('\n')
            .map_or(self.content.len(), |i| range.end + i + 1);
        let last_para = old_para_end == self.content.len();

        // How many rows it took without a scrollbar, if we need to know.
        let old_full_rows = self
            .scrollbar
            .map(|_| make_rows(&self.content[para_start..old_para_end], size.x).len());

        let removed = range.len();
        self.content.replace_range(range, replacement);
        self.version = crate::view::fresh_layout_key();
        let para_end = old_para_end + replacement.len() - removed;

        // Replace the paragraph's rows (not counting the ghost row).
        if self.ghost_row {
            self.rows.pop();
        }
        let first = self.rows.partition_point(|row| row.start < para_start);
        let last = if last_para {
            self.rows.len()
        } else {
            self.rows.partition_point(|row| row.start < old_para_end)
        };
        let width = if self.scrollbar.is_some() {
            size.x.saturating_sub(1)
        } else {
            size.x
        };
        let new_rows: Vec<Row> = make_rows(&self.content[para_start..para_end], width)
            .into_iter()
            .map(|row| row.shifted(para_start))
            .collect();
        let new_count = new_rows.len();
        self.rows.splice(first..last, new_rows);

        // Following rows just moved.
        for row in &mut self.rows[first + new_count..] {
            if replacement.len() >= removed {
                row.shift(replacement.len() - removed);
            } else {
                row.rev_shift(removed - replacement.len());
            }
        }
        self.ghost_row = add_ghost_row(&mut self.rows, self.content.len());

        // Do we still need a scrollbar (or not)? If that changed, all rows
        // change width.
        let needs_scrollbar = match (self.scrollbar, old_full_rows) {
            (Some((full_rows, full_ghost)), Some(old)) => {
                let new = make_rows(&self.content[para_start..para_end], size.x);
                let full_rows = full_rows + new.len() - old;
                // Only the last paragraph decides if there is a ghost row.
                let full_ghost = if last_para {
                    new.last()
                        .is_none_or(|row| para_start + row.end != self.content.len())
                } else {
                    full_ghost
                };
                self.scrollbar = Some((full_rows, full_ghost));
                full_rows + usize::from(full_ghost) > size.y
            }
            _ => self.rows.len() > size.y,
        };
        if needs_scrollbar != self.scrollbar.is_some() {
            self.invalidate();
            self.compute_rows(size);
        } else {
            self.scrollbase.set_heights(size.y, self.rows.len());
        }
    }

    // Computes rows again after the content changed, for our current size.
    fn refresh_rows(&mut self) {
        self.invalidate();
        self.compute_rows(self.last_size);
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
        self.edit(self.cursor..self.cursor + len, "");
    }

    fn insert(&mut self, ch: char) {
        let mut buffer = [0; 4];
        self.edit(self.cursor..self.cursor, ch.encode_utf8(&mut buffer));
        self.cursor += ch.len_utf8();
    }
}

impl View for TextArea {
    fn layout_key(&self) -> u64 {
        crate::view::combine_layout_key(crate::view::layout_key_seed::<Self>(), &self.version)
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        if self.sizes_version != self.version {
            self.sizes.clear();
            self.sizes_version = self.version;
        }
        if let Some(&(_, size)) = self.sizes.iter().find(|&&(req, _)| req == constraint) {
            return size;
        }

        let (rows, _, _) = self.rows_for(constraint);

        // Ideally, we'd want x = the longest row + 1
        // (we always keep a space at the end)
        // And y = number of rows
        let scroll_width = usize::from(rows.len() > constraint.y);

        let content_width = if rows.iter().any(|row| row.is_wrapped) {
            // If any row has been wrapped, we want to take the full width.
            constraint.x.saturating_sub(1 + scroll_width)
        } else {
            rows.iter().map(|r| r.width).max().unwrap_or(1)
        };

        let size = Vec2::new(scroll_width + 1 + content_width, rows.len());
        const SIZES_CAP: usize = 8;
        if self.sizes.len() == SIZES_CAP {
            self.sizes.remove(0);
        }
        self.sizes.push((constraint, size));
        size
    }

    fn draw(&self, printer: &Printer) {
        let (style, cursor_style) = if self.enabled && printer.enabled {
            (self.regular_style, self.cursor_style)
        } else {
            (self.inactive_style, self.inactive_style)
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

        debug!("Content: `{}`", self.content);
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
                    printer.theme.glyphs.edit_filler()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::PaletteColor;

    #[test]
    fn set_style_overrides_default() {
        let mut area = TextArea::new();
        assert_eq!(area.regular_style, PaletteStyle::EditableText.into());

        area.set_style(PaletteColor::Primary);
        assert_eq!(area.regular_style, PaletteColor::Primary.into());

        // Setting the text style leaves the cursor and inactive styles alone.
        assert_eq!(area.cursor_style, PaletteStyle::EditableTextCursor.into());
        assert_eq!(
            area.inactive_style,
            PaletteStyle::EditableTextInactive.into()
        );
    }

    #[test]
    fn size_cache_stays_bounded() {
        let mut area = TextArea::new().content("some text\nthat wraps at many widths");
        for i in 0..5000 {
            area.required_size(Vec2::new(i % 300, 1 + i % 40));
            if i % 7 == 0 {
                area.layout(Vec2::new(i % 300, 1 + i % 40));
            }
            assert!(area.sizes.len() <= 8);
        }
    }

    #[test]
    fn edits_match_fresh() {
        // Rows used to be patched incrementally on edits, which could drift
        // from what the full computation gives (and panic later). Type into
        // a text area at random, with layouts at random sizes, and compare it
        // with a fresh one every time.
        use crate::event::{Event, Key};
        let mut seed = 99u64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let chars = ['a', 'b', ' ', 'x', '中', '\n', 'é'];
        let keys = [
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
            Key::Home,
            Key::End,
        ];
        for _ in 0..200 {
            let mut area = TextArea::new();
            let mut size = Vec2::new(1 + rnd(20), 1 + rnd(8));
            area.layout(size);
            for _ in 0..60 {
                let event = match rnd(10) {
                    0..=4 => Event::Char(chars[rnd(chars.len())]),
                    5 => Event::Key(Key::Backspace),
                    6 => Event::Key(Key::Del),
                    7 => Event::Key(Key::Enter),
                    8 => Event::Key(keys[rnd(keys.len())]),
                    _ => {
                        size = Vec2::new(1 + rnd(20), 1 + rnd(8));
                        area.layout(size);
                        continue;
                    }
                };
                area.on_event(event);

                let mut fresh = TextArea::new().content(area.get_content());
                let req = Vec2::new(1 + rnd(20), 1 + rnd(8));
                assert_eq!(area.required_size(req), fresh.required_size(req));
                fresh.layout(size);
                assert_eq!(area.rows, fresh.rows, "{:?}", area.get_content());
            }
        }
    }
}
