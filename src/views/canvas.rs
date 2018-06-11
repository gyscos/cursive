use direction::Direction;
use event::{Event, EventResult};
use vec::Vec2;
use view::View;
use Printer;
use With;

/// A blank view that forwards calls to closures.
///
/// You can use this view to easily draw your own interface.
pub struct Canvas<T> {
    state: T,

    draw: Box<Fn(&T, &Printer)>,
    on_event: Box<FnMut(&mut T, Event) -> EventResult>,
    required_size: Box<FnMut(&mut T, Vec2) -> Vec2>,
    layout: Box<FnMut(&mut T, Vec2)>,
    take_focus: Box<FnMut(&mut T, Direction) -> bool>,
    needs_relayout: Box<Fn(&T) -> bool>,
}

impl<T: 'static + View> Canvas<T> {
    /// Creates a new Canvas around the given view.
    ///
    /// By default, forwards all calls to the inner view.
    pub fn wrap(view: T) -> Self {
        Canvas::new(view)
            .with_draw(T::draw)
            .with_on_event(T::on_event)
            .with_required_size(T::required_size)
            .with_layout(T::layout)
            .with_take_focus(T::take_focus)
            .with_needs_relayout(T::needs_relayout)
    }
}

impl<T> Canvas<T> {
    /// Creates a new, empty Canvas.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::Canvas;
    /// let canvas = Canvas::new(())
    ///                     .with_draw(|printer, _| {
    ///                         // Print the view
    ///                     });
    /// ```
    pub fn new(state: T) -> Self {
        Canvas {
            state,
            draw: Box::new(|_, _| ()),
            on_event: Box::new(|_, _| EventResult::Ignored),
            required_size: Box::new(|_, _| Vec2::new(1, 1)),
            layout: Box::new(|_, _| ()),
            take_focus: Box::new(|_, _| false),
            needs_relayout: Box::new(|_| true),
        }
    }

    /// Gets a mutable reference to the inner state.
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    /// Sets the closure for `draw(&Printer)`.
    pub fn set_draw<F>(&mut self, f: F)
    where
        F: 'static + Fn(&T, &Printer),
    {
        self.draw = Box::new(f);
    }

    /// Sets the closure for `draw(&Printer)`.
    ///
    /// Chainable variant.
    pub fn with_draw<F>(self, f: F) -> Self
    where
        F: 'static + Fn(&T, &Printer),
    {
        self.with(|s| s.set_draw(f))
    }

    /// Sets the closure for `on_event(Event)`.
    pub fn set_on_event<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Event) -> EventResult,
    {
        self.on_event = Box::new(f);
    }

    /// Sets the closure for `on_event(Event)`.
    ///
    /// Chainable variant.
    pub fn with_on_event<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Event) -> EventResult,
    {
        self.with(|s| s.set_on_event(f))
    }

    /// Sets the closure for `required_size(Vec2)`.
    pub fn set_required_size<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Vec2) -> Vec2,
    {
        self.required_size = Box::new(f);
    }

    /// Sets the closure for `required_size(Vec2)`.
    ///
    /// Chainable variant.
    pub fn with_required_size<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Vec2) -> Vec2,
    {
        self.with(|s| s.set_required_size(f))
    }

    /// Sets the closure for `layout(Vec2)`.
    pub fn set_layout<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Vec2),
    {
        self.layout = Box::new(f);
    }

    /// Sets the closure for `layout(Vec2)`.
    ///
    /// Chainable variant.
    pub fn with_layout<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Vec2),
    {
        self.with(|s| s.set_layout(f))
    }

    /// Sets the closure for `take_focus(Direction)`.
    pub fn set_take_focus<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Direction) -> bool,
    {
        self.take_focus = Box::new(f);
    }

    /// Sets the closure for `take_focus(Direction)`.
    ///
    /// Chainable variant.
    pub fn with_take_focus<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Direction) -> bool,
    {
        self.with(|s| s.set_take_focus(f))
    }

    /// Sets the closure for `needs_relayout()`.
    pub fn set_needs_relayout<F>(&mut self, f: F)
    where
        F: 'static + Fn(&T) -> bool,
    {
        self.needs_relayout = Box::new(f);
    }

    /// Sets the closure for `needs_relayout()`.
    ///
    /// Chainable variant.
    pub fn with_needs_relayout<F>(self, f: F) -> Self
    where
        F: 'static + Fn(&T) -> bool,
    {
        self.with(|s| s.set_needs_relayout(f))
    }
}

impl<T: 'static> View for Canvas<T> {
    fn draw(&self, printer: &Printer) {
        (self.draw)(&self.state, printer);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        (self.on_event)(&mut self.state, event)
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        (self.required_size)(&mut self.state, constraint)
    }

    fn layout(&mut self, size: Vec2) {
        (self.layout)(&mut self.state, size);
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        (self.take_focus)(&mut self.state, source)
    }
}
