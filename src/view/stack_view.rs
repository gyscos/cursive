use std::cmp::max;
use std::any::Any;

use vec::Vec2;
use view::{View,SizeRequest,DimensionRequest,Selector,ShadowView};
use event::EventResult;
use printer::Printer;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Layer>,
}

struct Layer {
    view: Box<View>,
    size: Vec2,
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
        self.layers.push(Layer {
            view: Box::new(ShadowView::new(view)),
            size: Vec2::new(0,0),
        });
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }
}

impl View for StackView {
    fn draw(&mut self, printer: &Printer, focused: bool) {
        for v in self.layers.iter_mut() {
            // Center the view
            let size = v.size;
            let offset = (printer.size - size) / 2;
            v.view.draw(&printer.sub_printer(offset, size), focused);
        }
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        match self.layers.last_mut() {
            None => EventResult::Ignored,
            Some(v) => v.view.on_key_event(ch),
        }
    }

    fn layout(&mut self, size: Vec2) {
        let req = SizeRequest {
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };
        for layer in self.layers.iter_mut() {
            layer.size = Vec2::min(size, layer.view.get_min_size(req));
            layer.view.layout(layer.size);
        }
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        // The min size is the max of all children's
        let mut s = Vec2::new(1,1);

        for layer in self.layers.iter() {
            let vs = layer.view.get_min_size(size);
            s = Vec2::max(s, vs);
        }

        s
    }

    fn take_focus(&mut self) -> bool {
        match self.layers.last_mut() {
            None => false,
            Some(mut v) => v.view.take_focus()
        }
    }

    fn find(&mut self, selector: &Selector) -> Option<&mut Any> {
        for layer in self.layers.iter_mut() {
            if let Some(any) = layer.view.find(selector) {
                return Some(any);
            }
        }
        None
    }
}
