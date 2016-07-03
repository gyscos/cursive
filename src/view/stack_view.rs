use std::any::Any;

use vec::Vec2;
use view::{Position, Selector, ShadowView, View};
use event::{Event, EventResult};
use printer::Printer;
use theme::ColorStyle;

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Layer>,
}

struct Layer {
    view: Box<View>,
    size: Vec2,
    position: Position,
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

    /// Adds new view on top of the stack in the center of the screen.
    pub fn add_layer<T: 'static + View>(&mut self, view: T) {
        self.add_layer_at(Position::center(), view);
    }

    /// Adds a view on top of the stack.
    pub fn add_layer_at<T: 'static + View>(&mut self, position: Position,
                                           view: T) {
        self.layers.push(Layer {
            view: Box::new(ShadowView::new(view).no_topleft_padding()),
            size: Vec2::new(0, 0),
            position: position,
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
        let mut previous = Vec2::zero();
        printer.with_color(ColorStyle::Primary, |printer| {
            for (i, v) in self.layers.iter_mut().enumerate() {
                // Place the view
                // Center the view
                let mut offset = v.position
                              .compute_offset(v.size, printer.size, previous);

                previous = offset;
                v.view
                 .draw(&printer.sub_printer(offset, v.size, i + 1 == last));
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
        // The call has been made, we can't ask for more space anymore.
        // Let's make do with what we have.

        for layer in &mut self.layers {
            // Give each guy what he asks for, within the budget constraints.
            layer.size = Vec2::min(size, layer.view.get_min_size(size));
            layer.view.layout(layer.size);

            // We do it here instead of when adding a new layer because...?
            // (TODO: try to make it during layer addition)
            if layer.virgin {
                layer.view.take_focus();
                layer.virgin = false;
            }
        }
    }

    fn get_min_size(&self, size: Vec2) -> Vec2 {
        // The min size is the max of all children's

        self.layers
            .iter()
            .map(|layer| layer.view.get_min_size(size))
            .fold(Vec2::new(1, 1), Vec2::max)
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
