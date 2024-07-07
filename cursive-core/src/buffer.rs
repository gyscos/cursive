//! Output buffer

use crate::backend::Backend;
use crate::style::ConcreteStyle;
use crate::{Rect, Vec2};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The width of a cell.
///
/// Most characters are single-width. Some asian characters and emojis are double-width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellWidth {
    /// This character takes a single cell in the grid.
    Single,

    /// This character takes 2 cells in the grid (mostly for emojis and asian characters).
    Double,
}

impl Default for CellWidth {
    fn default() -> Self {
        CellWidth::Single
    }
}

impl CellWidth {
    /// Convert the width as returned from `UnicodeWidthStr::width()` into a `CellWidth`.
    ///
    /// # Panics
    ///
    /// If `width > 2`.
    pub fn from_usize(width: usize) -> Self {
        match width {
            1 => CellWidth::Single,
            2 => CellWidth::Double,
            n => panic!("expected width of 1 or 2 only. Got {n}."),
        }
    }

    /// Returns the width as a usize: 1 or 2.
    pub fn as_usize(self) -> usize {
        match self {
            CellWidth::Single => 1,
            CellWidth::Double => 2,
        }
    }

    /// Returns the width of the given grapheme.
    ///
    /// # Panics
    ///
    /// If `text` has a width > 2 (it means it is not a single grapheme).
    pub fn from_grapheme(text: &str) -> Self {
        Self::from_usize(text.width())
    }
}

/// A single cell in a grid.
///
/// Most characters use 1 cell in the grid. Some wide graphemes use 2 cells
/// (mostly asian characters and some emojis).
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Style used for this cell.
    style: ConcreteStyle,

    /// Text buffer for this cell.
    ///
    /// Most graphemes fit in a couple bytes, but there's theoretically no limit.
    text: compact_str::CompactString,

    /// Either 1 or 2.
    ///
    /// If it's 2, the next cell should be None.
    ///
    /// Should be equal to `text.width()`
    ///
    /// TODO: Use a smaller sized integer to reduce the memory footprint?
    width: CellWidth,
}

impl Cell {
    /// Returns the text content of this cell.
    ///
    /// This should be a single grapheme.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the width of this cell: either 1 or 2.
    ///
    /// If this returns 2, then the next cell in the grid should be empty.
    pub fn width(&self) -> usize {
        self.width.as_usize()
    }

    /// Sets the style for this cell.
    pub fn set_style(&mut self, style: ConcreteStyle) {
        self.style = style;
    }

    /// Sets the content of this cell.
    ///
    /// `text` should be a single grapheme, with width 1 or 2.
    ///
    /// # Panics
    ///
    /// If `text.width() > 2`.
    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        self.text.push_str(text);
        self.width = CellWidth::from_grapheme(text);
    }

    fn set(&mut self, style: ConcreteStyle, text: &str, width: CellWidth) {
        self.style = style;
        self.text.clear();
        self.text.push_str(text);
        self.width = width;
    }
}

/// A buffer for printing stuff.
///
/// Can be flushed out to the backend.
///
/// The goal of the buffer is to accumulate all print operation for a refresh cycle, then flush it
/// all to the backend in one go. This should improve performance by:
/// * Reducing the amount of cursor movements.
/// * Only flushing the diff from the previous screen.
/// * Removing any delay during printing the screen that could result in tearing.
pub struct PrintBuffer {
    // A cell can be `None` if the cell before was double-wide (or more).
    // It can also be `None` if this is the first time since last resize.

    // Where we are actively writing.
    active_buffer: Vec<Option<Cell>>,

    // Last buffer flushed.
    //
    // Used to compute the diff between active and frozen when flushing.
    frozen_buffer: Vec<Option<Cell>>,

    // This is an internal cache used to remember the last style flushed to the backend.
    current_style: ConcreteStyle,

    size: Vec2,
}

/// A view into a rectangular area of the buffer.
pub struct Window<'a> {
    buffer: &'a mut PrintBuffer,
    viewport: Rect,
}

impl<'a> Window<'a> {
    /// Returns the cell at the given location.
    ///
    /// Returns `None` if the cell is empty because the previous one was double-wide.
    pub fn cell_at(&self, pos: Vec2) -> Option<&Cell> {
        let pos = self.absolute_pos(pos)?;

        self.buffer.cell_at(pos)
    }

    fn absolute_pos(&self, pos: Vec2) -> Option<Vec2> {
        if !pos.fits_in(self.viewport.size()) {
            return None;
        }

        Some(pos + self.viewport.top_left())
    }

    /// Iterate on the rows of this window.
    pub fn rows(&self) -> impl Iterator<Item = &[Option<Cell>]> {
        self.buffer
            .rows()
            .skip(self.viewport.top())
            .take(self.viewport.height())
            .map(|row| &row[self.viewport.left()..=self.viewport.right()])
    }

    /// Returns the viewport this window is covering.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Returns the size of this window.
    pub fn size(&self) -> Vec2 {
        self.viewport.size()
    }

    /// Get mutable access to the style at the given cell, if any.
    pub fn style_at_mut<V>(&mut self, pos: V) -> Option<&mut ConcreteStyle>
    where
        V: Into<Vec2>,
    {
        let pos = pos.into();
        let pos = self.absolute_pos(pos)?;

        self.buffer.style_at_mut(pos)
    }
}

impl Default for PrintBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PrintBuffer {
    /// Create a new empty print buffer.
    pub const fn new() -> Self {
        PrintBuffer {
            active_buffer: Vec::new(),
            frozen_buffer: Vec::new(),
            current_style: ConcreteStyle::terminal_default(),
            size: Vec2::ZERO,
        }
    }

    /// Iterate on the rows of this buffer.
    pub fn rows(&self) -> impl Iterator<Item = &[Option<Cell>]> {
        self.active_buffer.chunks(self.size.x)
    }

    /// Clear this buffer.
    ///
    /// The buffer will be empty afterwards.
    pub fn clear(&mut self) {
        let len = self.size.x * self.size.y;

        let reset = |buffer: &mut Vec<Option<Cell>>| {
            buffer.clear();
            buffer.resize_with(len, Default::default);
        };

        reset(&mut self.active_buffer);
        reset(&mut self.frozen_buffer);
    }

    /// Fill the buffer with the given text and style.
    pub fn fill(&mut self, text: &str, style: impl Into<ConcreteStyle>) {
        let style = style.into();
        let width = CellWidth::from_usize(text.width());
        if width != CellWidth::Single {
            panic!("Filling the screen with double-wide characters is not currently supported.");
        }

        for cell in &mut *self.active_buffer {
            let cell = cell.get_or_insert_with(Default::default);
            cell.style = style;
            cell.text.clear();
            cell.text.push_str(text);
            cell.width = width;
        }
    }

    /// Returns the current size of the buffer.
    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// Resize the buffer to the given size.
    pub fn resize(&mut self, size: Vec2) {
        if self.size == size {
            return;
        }

        self.size = size;
        let len = size.x * size.y;

        self.active_buffer.clear();
        self.frozen_buffer.clear();

        self.active_buffer.resize_with(len, Default::default);
        self.frozen_buffer.resize_with(len, Default::default);
    }

    /// Print some text at the given location.
    pub fn print_at(&mut self, start: Vec2, text: &str, style: ConcreteStyle) {
        if !(start.strictly_lt(self.size)) {
            return;
        }

        // If the character before was double-wide, we need to remove it.
        // TODO: Do not re-compute the ID of the first cell twice?
        let id = self.cell_id(start);
        if self.active_buffer[id].is_none() && start.x > 0 {
            // If the previous character is double-wide, then this cell would be None.
            // So only check that if we're None to begin with.
            // Here `id - 1` is safe to compute since `start.x > 0`.
            if let Some(ref mut prev) = self.active_buffer[id - 1] {
                if prev.width == CellWidth::Double {
                    prev.width = CellWidth::Single;
                    prev.text.clear();
                    prev.text.push_str(" ");
                    // Preserve style.
                }
            }
        }

        let mut pos = start;

        // We only consider "regular" graphemes that can be displayed.
        //
        // Control characters should not be displayed.
        fn is_control_char(g: &str) -> bool {
            g.chars()
                .all(|c| matches!(c, (..='\u{001F}') | ('\u{007F}'..='\u{009F}')))
        }

        // Fill our active buffer
        // TODO: Use some WithWidth(&str, usize) to not re-compute width a thousand times
        for g in text.graphemes(true) {
            let width = g.width();
            if width == 0 {
                // Any zero-width grapheme can be ignored.
                // With unicode-width < 0.1.13, this includes control chars.
                continue;
            }

            if is_control_char(g) {
                // With unicode-width >= 0.1.13, control chars have non-zero width (in
                // practice width = 1).
                debug_assert_eq!(
                    1, width,
                    "Control character '{g:?}' should've had a width of 1"
                );
                debug_assert_eq!(1, "\u{FFFD}".width(), "\u{FFFD} should've had a width of 1");
                self.set_cell(pos, "\u{fffd}", CellWidth::from_usize(width), style);
            } else {
                self.set_cell(pos, g, CellWidth::from_usize(width), style);
            }
            pos.x += width;
        }
    }

    fn cell_id(&self, pos: Vec2) -> usize {
        pos.x + pos.y * self.size.x
    }

    /// Get mutable access to the style at the given cell, if any.
    ///
    /// Returns `None` if the previous cell was double-wide.
    pub fn style_at_mut(&mut self, pos: Vec2) -> Option<&mut ConcreteStyle> {
        let id = self.cell_id(pos);
        self.active_buffer[id].as_mut().map(|cell| &mut cell.style)
    }

    /// Returns the cell at the given location.
    pub fn cell_at(&self, pos: Vec2) -> Option<&Cell> {
        let id = self.cell_id(pos);
        self.active_buffer[id].as_ref()
    }

    /// Returns a mutable access to a sub-region from this buffer.
    pub fn window(&mut self, viewport: Rect) -> Option<Window<'_>> {
        if !viewport.bottom_right().fits_in(self.size) {
            return None;
        }

        Some(Window {
            buffer: self,
            viewport,
        })
    }

    /// Get the text at the given position
    ///
    /// Returns `None` if there is no text, because the previous cell was double-wide.
    pub fn cell_text(&self, pos: Vec2) -> Option<&str> {
        let id = self.cell_id(pos);
        self.active_buffer[id].as_ref().map(|cell| cell.text())
    }

    /// Get the style at the given position.
    ///
    /// Returns `None` if there is no text, because the previous cell was double-wide.
    pub fn cell_style(&self, pos: Vec2) -> Option<ConcreteStyle> {
        let id = self.cell_id(pos);
        self.active_buffer[id].as_ref().map(|cell| cell.style)
    }

    /// Set a cell.
    ///
    /// width _must_ be grapheme.width().
    fn set_cell(&mut self, pos: Vec2, grapheme: &str, width: CellWidth, style: ConcreteStyle) {
        debug_assert_eq!(width.as_usize(), grapheme.width());

        let id = self.cell_id(pos);

        let cell = &mut self.active_buffer[id].get_or_insert_with(Default::default);
        cell.set(style, grapheme, width);

        // If this is a double-wide grapheme, mark the next cell as blocked.
        for dx in 1..width.as_usize() {
            if pos.x + dx >= self.size.x {
                break;
            }

            self.active_buffer[id + dx] = None;
        }
    }

    /// Flush out all accumulated prints so far.
    ///
    /// * Assumes the backend was representing `self.frozen_buffer`.
    /// * Ensures the backend now represents `self.active_buffer`.
    /// * Try to minimize the commands sent to the backend to achieve that.
    ///
    /// Afterwards, replace `self.frozen_buffer` with `self.active_buffer`.
    /// `self.active_buffer` should not be affected by this call.
    ///
    /// Note: this does _not_ call `backend.refresh()`!
    ///
    /// Ideally it should be possible to call `flush()` at any times, possibly repeatedly.
    ///
    /// (Successive calls should do nothing.)
    pub fn flush(&mut self, backend: &dyn Backend) {
        let terminal_width = self.size.x;

        let persistent = backend.is_persistent();

        let mut current_pos = Vec2::zero();
        backend.move_to(current_pos);

        for (i, (active, frozen)) in self
            .active_buffer
            .iter()
            .zip(self.frozen_buffer.iter())
            .enumerate()
        {
            if persistent && active == frozen {
                // TODO (optim): it may be pricier to omit printing a letter but to then "move to" the
                // cell to the right. So there should be a price N for the jump, and wait until we see
                // N bytes without changes to actually jump. If it changes before that, flush the
                // unchanged bytes rather than the jump.

                // Let's not change this cell.
                continue;
            }

            // eprintln!("Non matching: {frozen:?} -> {active:?}");

            // Skip empty cells.
            let Some(Cell { style, text, width }) = active else {
                continue;
            };

            let x = i % terminal_width;
            let y = i / terminal_width;

            // Should we move?
            if current_pos != (x, y) {
                current_pos = Vec2::new(x, y);
                backend.move_to(current_pos);
            }

            // Make sure we have the correct style
            // eprintln!("Applying {style:?} over {:?} for {text} @ {x}:{y}", self.current_style);
            apply_diff(&self.current_style, style, backend);
            self.current_style = *style;

            backend.print(text);

            current_pos.x += width.as_usize();

            // Assume we never wrap over?
        }

        // Keep the active buffer the same, because why not?
        // We could also flush it to Nones?
        self.frozen_buffer.clone_from_slice(&self.active_buffer);
    }
}

fn apply_diff(old: &ConcreteStyle, new: &ConcreteStyle, backend: &dyn Backend) {
    if old.color != new.color {
        // TODO: flush front/back colors separately?
        backend.set_color(new.color);
    }

    // Check the diff between two effect sets:
    // - Effects in new but not in old
    for effect in new.effects.iter() {
        if old.effects.contains(effect) {
            continue;
        }
        backend.set_effect(effect);
    }
    // - Effects in old but not in new
    for effect in old.effects.iter() {
        if new.effects.contains(effect) {
            continue;
        }
        backend.unset_effect(effect);
    }
}
