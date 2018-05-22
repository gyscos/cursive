use direction::Direction;
use event::{AnyCb, Event, EventResult, Key, MouseEvent};
use rect::Rect;
use vec::Vec2;
use view::{Selector, View};
use xy::XY;
use Printer;
use With;

use std::cmp::min;

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

    // Should we show scrollbars?
    show_scrollbars: bool,

    // How much padding should be between content and scrollbar?
    scrollbar_padding: Vec2,
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
            show_scrollbars: true,
            scrollbar_padding: Vec2::new(1, 0),
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

impl<V> ScrollView<V>
where
    V: View,
{
    /// Compute the size we would need.
    ///
    /// Given the constraints, and the axis that need scrollbars.
    ///
    /// Returns `(inner_size, size, scrollable)`.
    fn sizes_when_scrolling(
        &mut self, constraint: Vec2, scrollable: XY<bool>,
    ) -> (Vec2, Vec2, XY<bool>) {
        // This is the size taken by the scrollbars.
        let scrollbar_size = scrollable
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero());

        let available = constraint.saturating_sub(scrollbar_size);

        let inner_size = self.inner.required_size(available);

        // Where we're "enabled", accept the constraints.
        // Where we're not, just forward inner_size.
        let size = self.enabled.select_or(
            Vec2::min(inner_size + scrollbar_size, constraint),
            inner_size + scrollbar_size,
        );

        let new_scrollable = inner_size.zip_map(size, |i, s| i > s);

        (inner_size, size, new_scrollable)
    }

    /// Computes the size we would need given the constraints.
    ///
    /// First be optimistic and try without scrollbars.
    /// Then try with scrollbars if needed.
    /// Then try again in case we now need to scroll both ways (!!!)
    ///
    /// Returns `(inner_size, size)`
    fn sizes(&mut self, constraint: Vec2) -> (Vec2, Vec2) {
        let (inner_size, size, scrollable) =
            self.sizes_when_scrolling(constraint, XY::new(false, false));

        // Did it work?
        if scrollable.any() && self.show_scrollbars {
            // Attempt 2: he wants to scroll? Sure! Try again with some space for the scrollbar.
            let (inner_size, size, new_scrollable) =
                self.sizes_when_scrolling(constraint, scrollable);
            if scrollable != new_scrollable {
                // Again? We're now scrolling in a new direction?
                // There is no end to this!
                let (inner_size, size, _) =
                    self.sizes_when_scrolling(constraint, new_scrollable);

                // That's enough. If the inner view changed again, ignore it!
                // That'll teach it.
                (inner_size, size)
            } else {
                // Yup, scrolling did it. We're goot to go now.
                (inner_size, size)
            }
        } else {
            // We're not showing any scrollbar, either because we don't scroll
            // or because scrollbars are hidden.
            (inner_size, size)
        }
    }
}

impl<V> View for ScrollView<V>
where
    V: View,
{
    fn draw(&self, printer: &Printer) {
        // Draw content
        let printer = printer
            .content_offset(self.offset)
            .inner_size(self.inner_size);
        self.inner.draw(&printer);

        // Draw scrollbar?
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // Relativize event accorging to the offset
        let mut relative_event = event.clone();
        // eprintln!("Mouse = {:?}", relative_event);
        if let Some(pos) = relative_event.mouse_position_mut() {
            *pos = *pos + self.offset;
        }
        match self.inner.on_event(relative_event) {
            EventResult::Ignored => {
                // If it's an arrow, try to scroll in the given direction.
                // If it's a mouse scroll, try to scroll as well.
                // Also allow Ctrl+arrow to move the view without moving selection.
                match event {
                    Event::Mouse {
                        event: MouseEvent::WheelUp,
                        ..
                    } if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y = self.offset.y.saturating_sub(3);
                        EventResult::Consumed(None)
                    }
                    Event::Mouse {
                        event: MouseEvent::WheelDown,
                        ..
                    } if self.enabled.y
                        && (self.offset.y + self.last_size.y
                            < self.inner_size.y) =>
                    {
                        self.offset.y = min(
                            self.inner_size.y.saturating_sub(self.last_size.y),
                            self.offset.y + 3,
                        );
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Up) | Event::Key(Key::Up)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Down) | Event::Key(Key::Down)
                        if self.enabled.y
                            && (self.offset.y + self.last_size.y
                                < self.inner_size.y) =>
                    {
                        self.offset.y += 1;
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Left) | Event::Key(Key::Left)
                        if self.enabled.x && self.offset.x > 0 =>
                    {
                        self.offset.x -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Right) | Event::Key(Key::Right)
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
            other => {
                // Fix offset?
                let important = self.inner.important_area(self.inner_size);

                // The furthest top-left we can go
                let top_left = (important.bottom_right() + (1, 1))
                    .saturating_sub(self.last_size);
                // The furthest bottom-right we can go
                let bottom_right = important.top_left();

                // "top_left < bottom_right" is NOT guaranteed
                // if the child is larger than the view.
                let offset_min = Vec2::min(top_left, bottom_right);
                let offset_max = Vec2::max(top_left, bottom_right);

                self.offset =
                    self.offset.or_max(offset_min).or_min(offset_max);

                other
            }
        }
    }

    fn layout(&mut self, size: Vec2) {
        // Size is final now
        self.last_size = size;

        let (inner_size, _) = self.sizes(size);

        // Ask one more time
        self.inner_size = inner_size;

        self.inner.layout(self.inner_size);

        // TODO: Refresh offset if needed!
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // Attempt 1: try without scrollbars
        let (_, size) = self.sizes(constraint);

        size
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
