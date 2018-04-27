use direction::Direction;
use event::{AnyCb, Event, EventResult, Key};
use rect::Rect;
use vec::Vec2;
use view::{Selector, View};
use xy::XY;
use With;
use Printer;

/// Wraps a view in a scrollable area.
pub struct ScrollView<V> {
    inner_size: Vec2,
    inner: V,
    // Offset into the inner view.
    //
    // Our `(0,0)` will be inner's `offset`
    offset: Vec2,

    last_size: Vec2,

    // Can we scroll horizontally?
    enabled: XY<bool>,
}

impl<V> ScrollView<V> {
    /// Creates a new ScrollView around `view`.
    pub fn new(view: V) -> Self {
        ScrollView {
            inner: view,
            inner_size: Vec2::zero(),
            offset: Vec2::zero(),
            last_size: Vec2::zero(),
            enabled: XY::new(false, true),
        }
    }

    /// Returns the viewport in the inner content.
    pub fn content_viewport(&self) -> Rect {
        Rect::from_size(self.offset, self.last_size)
    }

    /// Sets the scroll offset to the given value
    pub fn set_offset<S>(&mut self, offset: S)
    where
        S: Into<Vec2>,
    {
        let max_offset = self.inner_size.saturating_sub(self.last_size);
        self.offset = offset.into().or_min(max_offset);
    }

    /// Controls whether this view can scroll vertically.
    ///
    /// Defaults to `true`.
    pub fn set_scroll_y(&mut self, enabled: bool) {
        self.enabled.y = enabled;
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    pub fn set_scroll_x(&mut self, enabled: bool) {
        self.enabled.x = enabled;
    }

    /// Controls whether this view can scroll vertically.
    ///
    /// Defaults to `true`.
    ///
    /// Chainable variant.
    pub fn scroll_y(self, enabled: bool) -> Self {
        self.with(|s| s.set_scroll_y(enabled))
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    ///
    /// Chainable variant.
    pub fn scroll_x(self, enabled: bool) -> Self {
        self.with(|s| s.set_scroll_x(enabled))
    }
}

impl<V> View for ScrollView<V>
where
    V: View,
{
    fn draw(&self, printer: &Printer) {
        // Draw content
        let printer = printer.content_offset(self.offset).inner_size(self.inner_size);
        self.inner.draw(&printer);

        // Draw scrollbar?
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // Relativize event accorging to the offset
        let mut relative_event = event.clone();
        if let Some(pos) = relative_event.mouse_position_mut() {
            *pos = *pos + self.offset;
        }
        match self.inner.on_event(relative_event) {
            EventResult::Ignored => {
                // If it's an arrow, try to scroll in the given direction.
                // If it's a mouse scroll, try to scroll as well.
                match event {
                    Event::Key(Key::Up)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Down)
                        if self.enabled.y
                            && (self.offset.y + self.last_size.y
                                < self.inner_size.y) =>
                    {
                        self.offset.y += 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Left)
                        if self.enabled.x && self.offset.x > 0 =>
                    {
                        self.offset.x -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Right)
                        if self.enabled.x
                            && (self.offset.x + self.last_size.x
                                < self.inner_size.x) =>
                    {
                        self.offset.x += 1;
                        EventResult::Consumed(None)
                    }
                    _ => EventResult::Ignored,
                }
            }
            other => other,
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;

        // Ask one more time
        self.inner_size = self.inner.required_size(size).or_max(size);
        self.inner.layout(self.inner_size);

        // TODO: Refresh offset if needed!
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // TODO: depend on allowed axis
        let size = self.inner.required_size(constraint);

        // For each enabled direction, return the min of size and constraint.
        // For the rest, directly return the size.
        self.enabled
            .select_or(Vec2::min(size, constraint), size)
    }

    fn call_on_any<'a>(&mut self, selector: &Selector, cb: AnyCb<'a>) {
        self.inner.call_on_any(selector, cb)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        self.inner.focus_view(selector)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        let is_scrollable =
            self.enabled.any() && (self.inner_size != self.last_size);
        self.inner.take_focus(source) || is_scrollable
    }
}
