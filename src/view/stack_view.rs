use std::any::Any;

use direction::Direction;
use backend::Backend;
use vec::Vec2;
use view::{Offset, Position, Selector, ShadowView, View};
use event::{Event, EventResult};
use Printer;
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
            // Skip padding for absolute/parent-placed views
            view: Box::new(ShadowView::new(view)
                .top_padding(position.y == Offset::Center)
                .left_padding(position.x == Offset::Center)),
            size: Vec2::new(0, 0),
            position: position,
            virgin: true,
        });
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
        ::B::clear();
    }
}

impl View for StackView {
    fn draw(&self, printer: &Printer) {
        let last = self.layers.len();
        let mut previous = Vec2::zero();
        printer.with_color(ColorStyle::Primary, |printer| {
            for (i, v) in self.layers.iter().enumerate() {
                // Place the view
                // Center the view
                let offset = v.position
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
                layer.view.take_focus(Direction::none());
                layer.virgin = false;
            }
        }
    }

    fn get_min_size(&mut self, size: Vec2) -> Vec2 {
        // The min size is the max of all children's

        self.layers
            .iter_mut()
            .map(|layer| layer.view.get_min_size(size))
            .fold(Vec2::new(1, 1), Vec2::max)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match self.layers.last_mut() {
            None => false,
            Some(mut v) => v.view.take_focus(source),
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
