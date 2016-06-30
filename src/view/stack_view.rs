use std::any::Any;

use vec::Vec2;
use view::{DimensionRequest, Selector, ShadowView, SizeRequest, View};
use event::{Event, EventResult};
use printer::Printer;
use theme::ColorPair;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Layer>,
}

struct Layer {
    view: Box<View>,
    size: Vec2,
    // Has it received the gift yet?
    virgin: bool,
}

impl Default for StackView {
    fn default() -> Self {
        Self::new()
    }
}

impl StackView {
    /// Creates a new empty StackView
    pub fn new() -> Self {
        StackView { layers: Vec::new() }
    }

    /// Add new view on top of the stack.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.layers.push(Layer {
            view: Box::new(ShadowView::new(view)),
            size: Vec2::new(0, 0),
            virgin: true,
        });
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }
}

impl View for StackView {
    fn draw(&mut self, printer: &Printer) {
        let last = self.layers.len();
        printer.with_color(ColorPair::Primary, |printer| {
            for (i, v) in self.layers.iter_mut().enumerate() {
                // Center the view
                let size = v.size;
                let offset = (printer.size - size) / 2;
                // TODO: only draw focus for the top view
                v.view.draw(&printer.sub_printer(offset, size, i + 1 == last));
            }
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.layers.last_mut() {
            None => EventResult::Ignored,
            Some(v) => v.view.on_event(event),
        }
    }

    fn layout(&mut self, size: Vec2) {
        let req = SizeRequest {
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };
        for layer in &mut self.layers {
            layer.size = Vec2::min(size, layer.view.get_min_size(req));
            layer.view.layout(layer.size);
            // We do it here instead of when adding a new layer because...?
            if layer.virgin {
                layer.view.take_focus();
                layer.virgin = false;
            }
        }
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        // The min size is the max of all children's
        let mut s = Vec2::new(1, 1);

        for layer in &self.layers {
            let vs = layer.view.get_min_size(size);
            s = Vec2::max(s, vs);
        }

        s
    }

    fn take_focus(&mut self) -> bool {
        match self.layers.last_mut() {
            None => false,
            Some(mut v) => v.view.take_focus(),
        }
    }

    fn find(&mut self, selector: &Selector) -> Option<&mut Any> {
        for layer in &mut self.layers {
            if let Some(any) = layer.view.find(selector) {
                return Some(any);
            }
        }
        None
    }
}
