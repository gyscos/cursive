//! Output buffer

use crate::backend::Backend;
use crate::theme::ConcreteStyle;
use crate::Vec2;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The width of a cell.
///
/// Most characters are single-width. Some asian characters and emojis are double-width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellWidth {
    Single,
    Double,
}

impl Default for CellWidth {
    fn default() -> Self {
        CellWidth::Single
    }
}

impl CellWidth {
    pub fn from_usize(width: usize) -> Self {
        match width {
            1 => CellWidth::Single,
            2 => CellWidth::Double,
            _ => panic!("expected width of 1 or 2 only."),
        }
    }

    pub fn as_usize(self) -> usize {
        match self {
            CellWidth::Single => 1,
            CellWidth::Double => 2,
        }
    }
}

/// A single cell in a grid.
///
/// Most characters use 1 cell in the grid. Some wide graphemes use 2 cells
/// (mostly asian characters and some emojis).
#[derive(Default, Clone, Debug, PartialEq, Eq)]
struct Cell {
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

    current_style: ConcreteStyle,

    size: Vec2,
}

impl Default for PrintBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PrintBuffer {
    pub const fn new() -> Self {
        PrintBuffer {
            active_buffer: Vec::new(),
            frozen_buffer: Vec::new(),
            current_style: ConcreteStyle::terminal_default(),
            size: Vec2::ZERO,
        }
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

    pub fn size(&self) -> Vec2 {
        self.size
    }

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

        // Fill our active buffer
        // TODO: Use some WithWidth(&str, usize) to not re-compute width a thousand times
        for g in text.graphemes(true) {
            let width = g.width();
            if width == 0 {
                continue;
            }
            self.set_cell(pos, g, CellWidth::from_usize(width), style);
            pos.x += width;
        }
    }

    fn cell_id(&self, pos: Vec2) -> usize {
        pos.x + pos.y * self.size.x
    }

    pub fn cell_text(&self, pos: Vec2) -> Option<&str> {
        let id = self.cell_id(pos);
        self.active_buffer[id]
            .as_ref()
            .map(|cell| cell.text.as_str())
    }

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
        cell.style = style;
        cell.text.clear();
        cell.text.push_str(grapheme);
        cell.width = width;

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

        let mut current_pos = Vec2::zero();
        backend.move_to(current_pos);

        for (i, (active, frozen)) in self
            .active_buffer
            .iter()
            .zip(self.frozen_buffer.iter())
            .enumerate()
        {
            if active == frozen {
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
