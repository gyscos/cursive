use std::cmp::max;

use vec2::Vec2;
use view::{View,SizeRequest};
use event::EventResult;
use printer::Printer;

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

    /// Add new view on top of the stack.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.layers.push(Box::new(view));
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }
}


impl View for StackView {
    fn draw(&self, printer: &Printer) {
        match self.layers.last() {
            None => (),
            Some(v) => v.draw(printer),
        }
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match self.layers.last_mut() {
            None => EventResult::Ignored,
            Some(v) => v.on_key_event(ch),
        }
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        // The min size is the max of all children's
        let mut s = Vec2::new(1,1);

        for view in self.layers.iter() {
            let vs = view.get_min_size(size);
            s.x = max(s.x, vs.x);
            s.y = max(s.y, vs.y);
        }

        s
    }
}
