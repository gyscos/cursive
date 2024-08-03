use crate::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    rect::Rect,
    view::{CannotFocus, Selector, View, ViewNotFound},
    Printer, Vec2, With,
};

// Define these types separately to appease the Clippy god
type Draw<T> = dyn Fn(&T, &Printer) + Send + Sync;
type OnEvent<T> = dyn FnMut(&mut T, Event) -> EventResult + Send + Sync;
type RequestSize<T> = dyn FnMut(&mut T, Vec2) -> Vec2 + Send + Sync;
type Layout<T> = dyn FnMut(&mut T, Vec2) + Send + Sync;
type NeedsRelayout<T> = dyn Fn(&T) -> bool + Send + Sync;
type TakeFocus<T> = dyn FnMut(&mut T, Direction) -> Result<EventResult, CannotFocus> + Send + Sync;
type FocusView<T> = dyn FnMut(&mut T, &Selector) -> Result<EventResult, ViewNotFound> + Send + Sync;
type CallOnAny<T> = dyn FnMut(&mut T, &Selector, AnyCb) + Send + Sync;
type ImportantArea<T> = dyn Fn(&T, Vec2) -> Rect + Send + Sync;

/// A blank view that forwards calls to closures.
///
/// You can use this view to easily draw your own interface.
///
/// # Examples
///
/// ```rust
/// use cursive_core::event::{Event, EventResult, Key};
/// use cursive_core::views::{Canvas, Dialog};
/// use unicode_width::UnicodeWidthStr; // To get the width of some text.
///
/// // Build a canvas around a string.
/// let state = String::new();
/// let canvas = Canvas::new(state)
///     .with_draw(|text: &String, printer| {
///         // Simply print our string
///         printer.print((0, 0), text);
///     })
///     .with_on_event(|text: &mut String, event| match event {
///         Event::Char(c) => {
///             text.push(c);
///             EventResult::Consumed(None)
///         }
///         Event::Key(Key::Enter) => {
///             let text = text.clone();
///             EventResult::with_cb(move |s| {
///                 s.add_layer(Dialog::info(&text));
///             })
///         }
///         _ => EventResult::Ignored,
///     })
///     .with_required_size(|text, _constraints| (text.width(), 1).into());
/// ```
pub struct Canvas<T> {
    state: T,

    draw: Box<Draw<T>>,
    on_event: Box<OnEvent<T>>,
    required_size: Box<RequestSize<T>>,
    layout: Box<Layout<T>>,
    take_focus: Box<TakeFocus<T>>,
    needs_relayout: Box<NeedsRelayout<T>>,
    focus_view: Box<FocusView<T>>,
    call_on_any: Box<CallOnAny<T>>,
    important_area: Box<ImportantArea<T>>,
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
            .with_focus_view(T::focus_view)
            .with_call_on_any(T::call_on_any)
            .with_important_area(T::important_area)
    }
}

impl<T> Canvas<T> {
    /// Creates a new, empty Canvas.
    pub fn new(state: T) -> Self {
        Canvas {
            state,
            draw: Box::new(|_, _| ()),
            on_event: Box::new(|_, _| EventResult::Ignored),
            required_size: Box::new(|_, _| Vec2::new(1, 1)),
            layout: Box::new(|_, _| ()),
            take_focus: Box::new(|_, _| Err(CannotFocus)),
            needs_relayout: Box::new(|_| true),
            focus_view: Box::new(|_, _| Err(ViewNotFound)),
            call_on_any: Box::new(|_, _, _| ()),
            important_area: Box::new(|_, size| Rect::from_corners((0, 0), size)),
        }
    }

    /// Gets a mutable reference to the inner state.
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    /// Sets the closure for `draw(&Printer)`.
    pub fn set_draw<F>(&mut self, f: F)
    where
        F: 'static + Fn(&T, &Printer) + Send + Sync,
    {
        self.draw = Box::new(f);
    }

    /// Sets the closure for `draw(&Printer)`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_draw<F>(self, f: F) -> Self
    where
        F: 'static + Fn(&T, &Printer) + Send + Sync,
    {
        self.with(|s| s.set_draw(f))
    }

    /// Sets the closure for `on_event(Event)`.
    pub fn set_on_event<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Event) -> EventResult + Send + Sync,
    {
        self.on_event = Box::new(f);
    }

    /// Sets the closure for `on_event(Event)`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_on_event<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Event) -> EventResult + Send + Sync,
    {
        self.with(|s| s.set_on_event(f))
    }

    /// Sets the closure for `required_size(Vec2)`.
    pub fn set_required_size<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Vec2) -> Vec2 + Send + Sync,
    {
        self.required_size = Box::new(f);
    }

    /// Sets the closure for `required_size(Vec2)`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_required_size<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Vec2) -> Vec2 + Send + Sync,
    {
        self.with(|s| s.set_required_size(f))
    }

    /// Sets the closure for `layout(Vec2)`.
    pub fn set_layout<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Vec2) + Send + Sync,
    {
        self.layout = Box::new(f);
    }

    /// Sets the closure for `layout(Vec2)`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_layout<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Vec2) + Send + Sync,
    {
        self.with(|s| s.set_layout(f))
    }

    /// Sets the closure for `take_focus(Direction)`.
    pub fn set_take_focus<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, Direction) -> Result<EventResult, CannotFocus> + Send + Sync,
    {
        self.take_focus = Box::new(f);
    }

    /// Sets the closure for `take_focus(Direction)`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_take_focus<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, Direction) -> Result<EventResult, CannotFocus> + Send + Sync,
    {
        self.with(|s| s.set_take_focus(f))
    }

    /// Sets the closure for `needs_relayout()`.
    pub fn set_needs_relayout<F>(&mut self, f: F)
    where
        F: 'static + Fn(&T) -> bool + Send + Sync,
    {
        self.needs_relayout = Box::new(f);
    }

    /// Sets the closure for `needs_relayout()`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_needs_relayout<F>(self, f: F) -> Self
    where
        F: 'static + Fn(&T) -> bool + Send + Sync,
    {
        self.with(|s| s.set_needs_relayout(f))
    }

    /// Sets the closure for `call_on_any()`.
    pub fn set_call_on_any<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, &Selector, AnyCb) + Send + Sync,
    {
        self.call_on_any = Box::new(f);
    }

    /// Sets the closure for `call_on_any()`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_call_on_any<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, &Selector, AnyCb) + Send + Sync,
    {
        self.with(|s| s.set_call_on_any(f))
    }

    /// Sets the closure for `important_area()`.
    pub fn set_important_area<F>(&mut self, f: F)
    where
        F: 'static + Fn(&T, Vec2) -> Rect + Send + Sync,
    {
        self.important_area = Box::new(f);
    }

    /// Sets the closure for `important_area()`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_important_area<F>(self, f: F) -> Self
    where
        F: 'static + Fn(&T, Vec2) -> Rect + Send + Sync,
    {
        self.with(|s| s.set_important_area(f))
    }

    /// Sets the closure for `focus_view()`.
    pub fn set_focus_view<F>(&mut self, f: F)
    where
        F: 'static + FnMut(&mut T, &Selector) -> Result<EventResult, ViewNotFound> + Send + Sync,
    {
        self.focus_view = Box::new(f);
    }

    /// Sets the closure for `focus_view()`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn with_focus_view<F>(self, f: F) -> Self
    where
        F: 'static + FnMut(&mut T, &Selector) -> Result<EventResult, ViewNotFound> + Send + Sync,
    {
        self.with(|s| s.set_focus_view(f))
    }
}

impl<T: 'static + Send + Sync> View for Canvas<T> {
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

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        (self.take_focus)(&mut self.state, source)
    }

    fn needs_relayout(&self) -> bool {
        (self.needs_relayout)(&self.state)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        (self.focus_view)(&mut self.state, selector)
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        (self.important_area)(&self.state, view_size)
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        (self.call_on_any)(&mut self.state, selector, cb);
    }
}
