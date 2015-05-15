use ncurses;
use event::EventResult;
use super::{Size,ToSize};
use view::{View,SizeRequest};

/// BoxView is a wrapper around an other view, with a given minimum size.
pub struct BoxView {
    size: Size,

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
    pub fn new<S: ToSize, V: View + 'static>(size: S, view: V) -> Self {
        BoxView {
            size: size.to_size(),
            content: Box::new(view),
        }
    }
}

impl View for BoxView {
    fn on_key_event(&mut self, ch: i32) -> EventResult {
        self.content.on_key_event(ch)
    }

    fn draw(&self, win: ncurses::WINDOW, size: Size) {
        self.content.draw(win, size)
    }

    fn get_min_size(&self, _: SizeRequest) -> Size {
        self.size
    }

    fn layout(&mut self, size: Size) {
        self.content.layout(size);
    }
}
