//! Makes drawing on ncurses windows easier.

use ncurses;
use vec::{Vec2,ToVec2};

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

    /// Prints a vertical line using the given character.
    pub fn print_vline<T: ToVec2>(&self, c: char, start: T, len: u32) {
        let p = start.to_vec2();
        ncurses::mvwvline(self.win, (p.y + self.offset.y) as i32, (p.x + self.offset.x) as i32, c as u64, len as i32);
    }

    /// Prints a horizontal line using the given character.
    pub fn print_hline<T: ToVec2>(&self, c: char, start: T, len: u32) {
        let p = start.to_vec2();
        ncurses::mvwhline(self.win, (p.y + self.offset.y) as i32, (p.x + self.offset.x) as i32, c as u64, len as i32);
    }

    pub fn print_box<T: ToVec2>(&self, start: T, size: T, corners: char, horizontal: char, vertical: char) {
        let start_v = start.to_vec2();
        let size_v = size.to_vec2() - (1,1);

        self.print(start_v, &corners.to_string());
        self.print(start_v + size_v.keep_x(), &corners.to_string());
        self.print(start_v + size_v.keep_y(), &corners.to_string());
        self.print(start_v + size_v, &corners.to_string());
        self.print_hline(horizontal, start_v + (1,0), size_v.x - 1);
        self.print_hline(horizontal, start_v + (1,0) + size_v.keep_y(), size_v.x - 1);
        self.print_vline(vertical, start_v + (0,1), size_v.y - 1);
        self.print_vline(vertical, start_v + (0,1) + size_v.keep_x(), size_v.y - 1);
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
