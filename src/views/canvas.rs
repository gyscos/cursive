use Printer;
use With;
use direction::Direction;
use event::{Event, EventResult};
use vec::Vec2;
use view::View;

/// A blank view that forwards calls to closures.
///
/// You can use this view to easily draw your own interface.
pub struct Canvas<T> {
    state: T,

    draw: Box<Fn(&Printer, &T)>,
    on_event: Box<FnMut(Event, &mut T) -> EventResult>,
    get_min_size: Box<FnMut(Vec2, &mut T) -> Vec2>,
    layout: Box<FnMut(Vec2, &mut T)>,
    take_focus: Box<FnMut(Direction, &mut T) -> bool>,
}

impl<T> Canvas<T> {
    /// Creates a new, empty Canvas.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let canvas = Canvas::new(())
    ///                     .with_draw(|printer, _| {
    ///                         // Print the view
    ///                     });
    /// ```
    pub fn new(state: T) -> Self {
        Canvas {
            state: state,
            draw: Box::new(|_, _| ()),
            on_event: Box::new(|_, _| EventResult::Ignored),
            get_min_size: Box::new(|_, _| Vec2::new(1, 1)),
            layout: Box::new(|_, _| ()),
            take_focus: Box::new(|_, _| false),
        }
    }

    /// Sets the closure for `draw(&Printer)`
    pub fn set_draw<F>(&mut self, f: F)
        where F: 'static + Fn(&Printer, &T)
    {
        self.draw = Box::new(f);
    }

    /// Sets the closure for `draw(&Printer)`
    ///
    /// Chainable variant.
    pub fn with_draw<F>(self, f: F) -> Self
        where F: 'static + Fn(&Printer, &T)
    {
        self.with(|s| s.set_draw(f))
    }

    /// Sets the closure for `on_event(Event)`
    pub fn set_on_event<F>(&mut self, f: F)
        where F: 'static + FnMut(Event, &mut T) -> EventResult
    {
        self.on_event = Box::new(f);
    }

    /// Sets the closure for `on_event(Event)`
    ///
    /// Chainable variant.
    pub fn with_on_event<F>(self, f: F) -> Self
        where F: 'static + FnMut(Event, &mut T) -> EventResult
    {
        self.with(|s| s.set_on_event(f))
    }

    /// Sets the closure for `get_min_size(Vec2)`
    pub fn set_get_min_size<F>(&mut self, f: F)
        where F: 'static + FnMut(Vec2, &mut T) -> Vec2
    {
        self.get_min_size = Box::new(f);
    }

    /// Sets the closure for `get_min_size(Vec2)`
    ///
    /// Chainable variant.
    pub fn with_get_min_size<F>(self, f: F) -> Self
        where F: 'static + FnMut(Vec2, &mut T) -> Vec2
    {
        self.with(|s| s.set_get_min_size(f))
    }

    /// Sets the closure for `layout(Vec2)`
    pub fn set_layout<F>(&mut self, f: F)
        where F: 'static + FnMut(Vec2, &mut T)
    {
        self.layout = Box::new(f);
    }

    /// Sets the closure for `layout(Vec2)`
    ///
    /// Chainable variant.
    pub fn with_layout<F>(self, f: F) -> Self
        where F: 'static + FnMut(Vec2, &mut T)
    {
        self.with(|s| s.set_layout(f))
    }

    /// Sets the closure for `take_focus(Direction)`
    pub fn set_take_focus<F>(&mut self, f: F)
        where F: 'static + FnMut(Direction, &mut T) -> bool
    {
        self.take_focus = Box::new(f);
    }

    /// Sets the closure for `take_focus(Direction)`
    ///
    /// Chainable variant.
    pub fn with_take_focus<F>(self, f: F) -> Self
        where F: 'static + FnMut(Direction, &mut T) -> bool
    {
        self.with(|s| s.set_take_focus(f))
    }
}

impl <T> View for Canvas<T> {
    fn draw(&self, printer: &Printer) {
        (self.draw)(printer, &self.state);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        (self.on_event)(event, &mut self.state)
    }

    fn get_min_size(&mut self, constraint: Vec2) -> Vec2 {
        (self.get_min_size)(constraint, &mut self.state)
    }

    fn layout(&mut self, size: Vec2) {
        (self.layout)(size, &mut self.state);
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        (self.take_focus)(source, &mut self.state)
    }
}
