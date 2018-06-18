use direction::{Direction, Orientation};
use event::{AnyCb, Event, EventResult, Key, MouseButton, MouseEvent};
use rect::Rect;
use theme::ColorStyle;
use vec::Vec2;
use view::{Selector, View};
use xy::XY;
use Printer;
use With;

use std::cmp::min;

/// Wraps a view in a scrollable area.
pub struct ScrollView<V> {
    // The wrapped view.
    inner: V,

    // This is the size the child thinks we're giving him.
    inner_size: Vec2,

    // Offset into the inner view.
    //
    // Our `(0,0)` will be inner's `offset`
    offset: Vec2,

    // What was our own size last time we checked.
    //
    // This includes scrollbars, if any.
    last_size: Vec2,

    // Are we scrollable in each direction?
    enabled: XY<bool>,

    // Should we show scrollbars?
    //
    // Even if this is true, no scrollbar will be printed if we don't need to
    // scroll.
    //
    // TODO: have an option to always show the scrollbar.
    // TODO: have an option to show scrollbar on top/left.
    show_scrollbars: bool,

    // How much padding should be between content and scrollbar?
    scrollbar_padding: Vec2,

    /// Initial position of the cursor when dragging.
    thumb_grab: Option<(Orientation, usize)>,
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
            thumb_grab: None,
        }
    }

    /// Returns the viewport in the inner content.
    pub fn content_viewport(&self) -> Rect {
        Rect::from_size(self.offset, self.available_size())
    }

    /// Sets the scroll offset to the given value
    pub fn set_offset<S>(&mut self, offset: S)
    where
        S: Into<Vec2>,
    {
        let max_offset = self.inner_size.saturating_sub(self.available_size());
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

    /// Programmatically scroll to the top of the view.
    pub fn scroll_to_top(&mut self) {
        let curr_x = self.offset.x;
        self.set_offset((curr_x, 0));
    }

    /// Programmatically scroll to the bottom of the view.
    pub fn scroll_to_bottom(&mut self) {
        let max_y = self.inner_size.saturating_sub(self.available_size()).y;
        let curr_x = self.offset.x;
        self.set_offset((curr_x, max_y));
    }

    /// Programmatically scroll to the leftmost side of the view.
    pub fn scroll_to_left(&mut self) {
        let curr_y = self.offset.y;
        self.set_offset((0, curr_y));
    }

    /// Programmatically scroll to the rightmost side of the view.
    pub fn scroll_to_right(&mut self) {
        let max_x = self.inner_size.saturating_sub(self.available_size()).x;
        let curr_y = self.offset.y;
        self.set_offset((max_x, curr_y));
    }

    /// Returns for each axis if we are scrolling.
    fn is_scrolling(&self) -> XY<bool> {
        self.inner_size.zip_map(self.last_size, |i, s| i > s)
    }

    /// Stops grabbing the scrollbar.
    fn release_grab(&mut self) {
        self.thumb_grab = None;
    }

    /// Returns the size taken by the scrollbars.
    ///
    /// Will be zero in axis where we're not scrolling.
    fn scrollbar_size(&self) -> Vec2 {
        self.is_scrolling()
            .swap()
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero())
    }

    /// Returns the size available for the child view.
    fn available_size(&self) -> Vec2 {
        self.last_size.saturating_sub(self.scrollbar_size())
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
            .swap()
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero());

        let available = constraint.saturating_sub(scrollbar_size);

        // This the ideal size for the child. May not be what he gets.
        let inner_size = self.inner.required_size(available);

        // Where we're "enabled", accept the constraints.
        // Where we're not, just forward inner_size.
        let size = self.enabled.select_or(
            Vec2::min(inner_size + scrollbar_size, constraint),
            inner_size + scrollbar_size,
        );

        // On non-scrolling axis, give inner_size the available space instead.
        let inner_size = self
            .enabled
            .select_or(inner_size, size.saturating_sub(scrollbar_size));

        let new_scrollable = inner_size.zip_map(size, |i, s| i > s);

        (inner_size, size, new_scrollable)
    }

    /// Starts scrolling from the cursor position.
    ///
    /// Returns `true` if the event was consumed.
    fn start_drag(&mut self, position: Vec2) -> bool {
        let scrollbar_pos = self.last_size.saturating_sub((1, 1));

        let grabbed = scrollbar_pos.zip_map(position, |s, p| s == p);

        let lengths = self.scrollbar_thumb_lengths();
        let offsets = self.scrollbar_thumb_offsets(lengths);

        // See if we grabbed one of the scrollbars
        for (orientation, pos, length, offset) in
            XY::zip4(Orientation::pair(), position, lengths, offsets)
                .zip(grabbed.swap())
                .into_iter()
                .filter(|&(_, grab)| grab)
                .map(|(x, _)| x)
        {
            if pos >= offset && pos < offset + length {
                // We grabbed the thumb! Now scroll from that position.
                self.thumb_grab = Some((orientation, pos - offset));
            } else {
                // We hit the scrollbar, outside of the thumb.
                // Let's move the middle there.
                self.thumb_grab = Some((orientation, (length - 1) / 2));
                self.drag(position);
            }

            return true;
        }

        false
    }

    fn drag(&mut self, position: Vec2) {
        if let Some((orientation, grab)) = self.thumb_grab {
            self.scroll_to_thumb(
                orientation,
                position.get(orientation).saturating_sub(grab),
            );
        }
    }

    fn scroll_to_thumb(&mut self, orientation: Orientation, thumb_pos: usize) {
        let lengths = self.scrollbar_thumb_lengths();
        let available = self.available_size();

        // We want self.scrollbar_thumb_offsets() to be thumb_pos
        // steps * self.o / (self.inner + 1 - available) = thumb_pos
        // self.o = thumb_pos * (self.inner + 1 - available) / (available + 1 - lengths)

        // The new offset is:
        // thumb_pos * (content + 1 - available) / (available + 1 - thumb size)
        let new_offset = ((self.inner_size + (1, 1)).saturating_sub(available)
            * thumb_pos)
            .div_up((available + (1, 1)).saturating_sub(lengths));
        let max_offset = self.inner_size.saturating_sub(self.available_size());
        self.offset
            .set_axis_from(orientation, &new_offset.or_min(max_offset));
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

        // If we need to add scrollbars, the available size will change.
        if scrollable.any() && self.show_scrollbars {
            // Attempt 2: he wants to scroll? Sure!
            // Try again with some space for the scrollbar.
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

    fn scrollbar_thumb_lengths(&self) -> Vec2 {
        let available = self.available_size();
        (available * available / self.inner_size).or_max((1, 1))
    }

    fn scrollbar_thumb_offsets(&self, lengths: Vec2) -> Vec2 {
        let available = self.available_size();
        // The number of steps is 1 + the "extra space"
        let steps = (available + (1, 1)).saturating_sub(lengths);
        steps * self.offset / (self.inner_size + (1, 1) - available)
    }
}

impl<V> View for ScrollView<V>
where
    V: View,
{
    fn draw(&self, printer: &Printer) {
        // Draw scrollbar?
        let scrolling = self.is_scrolling();

        let lengths = self.scrollbar_thumb_lengths();
        let offsets = self.scrollbar_thumb_offsets(lengths);

        let line_c = XY::new("-", "|");

        let color = if printer.focused {
            ColorStyle::highlight()
        } else {
            ColorStyle::highlight_inactive()
        };

        let size = self.available_size();

        // TODO: use a more generic zip_all or something?
        XY::zip5(lengths, offsets, size, line_c, Orientation::pair()).run_if(
            scrolling,
            |(length, offset, size, c, orientation)| {
                let start = printer
                    .size
                    .saturating_sub((1, 1))
                    .with_axis(orientation, 0);
                let offset = orientation.make_vec(offset, 0);

                printer.print_line(orientation, start, size, c);

                let thumb_c = if self
                    .thumb_grab
                    .map(|(o, _)| o == orientation)
                    .unwrap_or(false)
                {
                    " "
                } else {
                    "▒"
                };
                printer.with_color(color, |printer| {
                    printer.print_line(
                        orientation,
                        start + offset,
                        length,
                        thumb_c,
                    );
                });
            },
        );

        if scrolling.both() {
            printer.print(printer.size.saturating_sub((1, 1)), "╳");
        }

        // Draw content
        let printer = printer
            .cropped(size)
            .content_offset(self.offset)
            .inner_size(self.inner_size);
        self.inner.draw(&printer);
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
                // Also allow Ctrl+arrow to move the view,
                // but not the selection.
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
                        && (self.offset.y + self.available_size().y
                            < self.inner_size.y) =>
                    {
                        self.offset.y = min(
                            self.inner_size
                                .y
                                .saturating_sub(self.available_size().y),
                            self.offset.y + 3,
                        );
                        EventResult::Consumed(None)
                    }
                    Event::Mouse {
                        event: MouseEvent::Press(MouseButton::Left),
                        position,
                        offset,
                    } if position
                        .checked_sub(offset)
                        .map(|position| self.start_drag(position))
                        .unwrap_or(false) =>
                    {
                        EventResult::Consumed(None)
                    }
                    Event::Mouse {
                        event: MouseEvent::Hold(MouseButton::Left),
                        position,
                        offset,
                    } => {
                        let position = position.saturating_sub(offset);
                        self.drag(position);
                        EventResult::Consumed(None)
                    }
                    Event::Mouse {
                        event: MouseEvent::Release(MouseButton::Left),
                        ..
                    } => {
                        self.release_grab();
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Home) if self.enabled.any() => {
                        self.offset =
                            self.enabled.select_or(Vec2::zero(), self.offset);
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::End) if self.enabled.any() => {
                        let max_offset = self
                            .inner_size
                            .saturating_sub(self.available_size());
                        self.offset =
                            self.enabled.select_or(max_offset, self.offset);
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Up) | Event::Key(Key::Up)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::PageUp)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y = self.offset.y.saturating_sub(5);
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::PageDown)
                        if self.enabled.y
                            && (self.offset.y + self.available_size().y
                                < self.inner_size.y) =>
                    {
                        self.offset.y += 5;
                        EventResult::Consumed(None)
                    }
                    Event::Ctrl(Key::Down) | Event::Key(Key::Down)
                        if self.enabled.y
                            && (self.offset.y + self.available_size().y
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
                            && (self.offset.x + self.available_size().x
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
                    .saturating_sub(self.available_size());
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

        // The offset cannot be more than content - available
        self.offset = self
            .offset
            .or_min(inner_size.saturating_sub(self.available_size()));
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
        let is_scrollable = self.is_scrolling().any();
        self.inner.take_focus(source) || is_scrollable
    }
}
