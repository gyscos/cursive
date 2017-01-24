use Printer;

use direction::Direction;
use event::{Event, EventResult};
use std::any::Any;
use theme::ColorStyle;
use vec::Vec2;
use view::{Offset, Position, Selector, View};
use views::{Layer, ShadowView};

/// Simple stack of views.
/// Only the top-most view is active and can receive input.
pub struct StackView {
    layers: Vec<Child>,
    last_size: Vec2,
}

enum Placement {
    Floating(Position),
    Fullscreen,
}

impl Placement {
    pub fn compute_offset<S, A, P>(&self, size: S, available: A, parent: P)
                                   -> Vec2
        where S: Into<Vec2>,
              A: Into<Vec2>,
              P: Into<Vec2>
    {
        match *self {
            Placement::Floating(ref position) => {
                position.compute_offset(size, available, parent)
            }
            Placement::Fullscreen => Vec2::zero(),
        }
    }
}

struct Child {
    view: Box<View>,
    size: Vec2,
    placement: Placement,

    // We cannot call `take_focus` until we've called `layout()`
    // So we want to call `take_focus` right after the first call
    // to `layout`; this flag remembers when we've done that.
    virgin: bool,
}

new_default!(StackView);

impl StackView {
    /// Creates a new empty StackView
    pub fn new() -> Self {
        StackView {
            layers: Vec::new(),
            last_size: Vec2::zero(),
        }
    }

    /// Adds a new full-screen layer on top of the stack.
    ///
    /// Fullscreen layers have no shadow.
    pub fn add_fullscreen_layer<T>(&mut self, view: T)
        where T: 'static + View
    {
        self.layers.push(Child {
            view: Box::new(Layer::new(view)),
            size: Vec2::zero(),
            placement: Placement::Fullscreen,
            virgin: true,
        });
    }

    /// Adds new view on top of the stack in the center of the screen.
    pub fn add_layer<T>(&mut self, view: T)
        where T: 'static + View
    {
        self.add_layer_at(Position::center(), view);
    }

    /// Adds a view on top of the stack.
    pub fn add_layer_at<T>(&mut self, position: Position, view: T)
        where T: 'static + View
    {
        self.layers.push(Child {
            // Skip padding for absolute/parent-placed views
            view: Box::new(ShadowView::new(Layer::new(view))
                .top_padding(position.y == Offset::Center)
                .left_padding(position.x == Offset::Center)),
            size: Vec2::new(0, 0),
            placement: Placement::Floating(position),
            virgin: true,
        });
    }

    /// Remove the top-most layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }

    /// Computes the offset of the current top view.
    pub fn offset(&self) -> Vec2 {
        let mut previous = Vec2::zero();
        for layer in &self.layers {
            let offset = layer.placement
                .compute_offset(layer.size, self.last_size, previous);
            previous = offset;
        }
        previous
    }

    /// Returns the size for each layer in this view.
    pub fn layer_sizes(&self) -> Vec<Vec2> {
        self.layers.iter().map(|layer| layer.size).collect()
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
                let offset = v.placement
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
        self.last_size = size;

        // The call has been made, we can't ask for more space anymore.
        // Let's make do with what we have.

        for layer in &mut self.layers {
            // Give each guy what he asks for, within the budget constraints.
            let size = Vec2::min(size, layer.view.get_min_size(size));
            layer.size = size;
            layer.view.layout(layer.size);

            // We need to call `layout()` on the view before giving it focus
            // for the first time. Otherwise it will not be properly set up.
            // Ex: examples/lorem.rs: the text view takes focus because it's
            // scrolling, but it only knows that after a call to `layout()`.
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

    fn find_any(&mut self, selector: &Selector) -> Option<&mut Any> {
        self.layers
            .iter_mut()
            .filter_map(|l| l.view.find_any(selector))
            .next()
    }
}
