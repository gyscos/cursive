use event::EventResult;
use vec::{Vec2,ToVec2};
use super::{View,SizeRequest};
use printer::Printer;

/// BoxView is a wrapper around an other view, with a given minimum size.
pub struct BoxView {
    size: Vec2,

    content: Box<View>,
}

impl BoxView {
    /// Creates a new BoxView with the given minimum size and content
    ///
    /// # Example
    ///
    /// ```
    /// // Creates a 20x4 BoxView with a TextView content.
    /// let box = BoxView::new((20,4), TextView::new("Hello!"))
    /// ```
    pub fn new<S: ToVec2, V: View + 'static>(size: S, view: V) -> Self {
        BoxView {
            size: size.to_vec2(),
            content: Box::new(view),
        }
    }
}

impl View for BoxView {
    fn on_key_event(&mut self, ch: i32) -> EventResult {
        self.content.on_key_event(ch)
    }

    fn draw(&self, printer: &Printer) {
        self.content.draw(printer)
    }

    fn get_min_size(&self, _: SizeRequest) -> Vec2 {
        self.size
    }

    fn layout(&mut self, size: Vec2) {
        self.content.layout(size);
    }
}
