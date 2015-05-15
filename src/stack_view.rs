use view::View;

use super::Size;
use ncurses;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Box<View>>,
}

impl StackView {
    /// Creates a new empty StackView
    pub fn new() -> Self {
        StackView {
            layers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, view: Box<View>) {
        self.layers.push(view);
    }
}


impl View for StackView {
    fn draw(&self, win: ncurses::WINDOW, size: Size) {
        match self.layers.last() {
            None => (),
            Some(v) => v.draw(win, size),
        }
    }
}
