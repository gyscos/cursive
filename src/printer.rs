//! Makes drawing on ncurses windows easier.

use ncurses;
use vec2::{Vec2,ToVec2};

/// Wrapper around a subset of a ncurses window.
pub struct Printer {
    /// ncurses window this printer will use. You can use it directly if you want.
    pub win: ncurses::WINDOW,
    /// Offset into the window this printer should start drawing at.
    pub offset: Vec2,
    /// Size of the area we are allowed to draw on.
    pub size: Vec2,
}

impl Printer {
    /// Prints some text at the given position relative to the window.
    pub fn print<S: ToVec2>(&self, pos: S, text: &str) {
        let p = pos.to_vec2();
        ncurses::mvwprintw(self.win, (p.y + self.offset.y) as i32, (p.x + self.offset.x) as i32, text);
    }

    /// Returns a printer on a subset of this one's area.
    pub fn sub_printer<S: ToVec2>(&self, offset: S, size: S) -> Printer {
        let offset_v = offset.to_vec2();
        Printer {
            win: self.win,
            offset: self.offset + offset_v,
            // We can't be larger than what remains
            size: Vec2::min(self.size - offset_v, size.to_vec2()),
        }
    }
}
