//! Core mechanisms to implement scrolling.
//!
//! This module defines [`ScrollCore`](crate::view::scroll::ScrollCore) and related traits.
//!
//! [`ScrollView`](crate::views::ScrollView) may be an easier way to add scrolling to an existing view.
use std::cmp::min;

use crate::direction::{Direction, Orientation};
use crate::event::{AnyCb, Event, EventResult, Key, MouseButton, MouseEvent};
use crate::printer::Printer;
use crate::rect::Rect;
use crate::theme::ColorStyle;
use crate::vec::Vec2;
use crate::view::{ScrollStrategy, Selector, SizeCache, View};
use crate::with::With;
use crate::XY;

/// Inner implementation for `ScrollCore::on_event`
pub trait InnerOnEvent {
    /// Performs `View::on_event()`
    fn on_event(&mut self, event: Event) -> EventResult;

    /// Performs `View::important_area()`
    fn important_area(&self, size: Vec2) -> Rect;
}

impl<'a, V: View> InnerOnEvent for &'a mut V {
    fn on_event(&mut self, event: Event) -> EventResult {
        <V as View>::on_event(self, event)
    }
    fn important_area(&self, size: Vec2) -> Rect {
        <V as View>::important_area(self, size)
    }
}

/// Inner implementation for `ScrollCore::draw()`
pub trait InnerDraw {
    /// Performs `View::draw()`
    fn draw(&self, printer: &Printer<'_, '_>);
}

impl<'a, V: View> InnerDraw for &'a V {
    fn draw(&self, printer: &Printer<'_, '_>) {
        <V as View>::draw(self, printer);
    }
}

/// Inner implementation for `ScrollCore::InnerLayout()`
pub trait InnerLayout {
    /// Performs `View::layout()`
    fn layout(&mut self, size: Vec2);
    /// Performs `View::needs_relayout()`
    fn needs_relayout(&self) -> bool;
    /// Performs `View::required_size()`
    fn required_size(&mut self, constraint: Vec2) -> Vec2;
}

struct Layout2Sizes<'a, I> {
    inner: &'a mut I,
}

impl<'a, I: InnerLayout> InnerSizes for Layout2Sizes<'a, I> {
    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }
}

impl<'a, V: View> InnerLayout for &'a mut V {
    fn layout(&mut self, size: Vec2) {
        <V as View>::layout(self, size);
    }
    fn needs_relayout(&self) -> bool {
        <V as View>::needs_relayout(self)
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        <V as View>::required_size(self, constraint)
    }
}

/// Inner implementation for `ScrollCore::required_size()`
pub trait InnerRequiredSize {
    /// Performs `View::needs_relayout()`
    fn needs_relayout(&self) -> bool;
    /// Performs `View::required_size()`
    fn required_size(&mut self, constraint: Vec2) -> Vec2;
}

impl<V: View> InnerRequiredSize for &mut V {
    fn needs_relayout(&self) -> bool {
        <V as View>::needs_relayout(self)
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        <V as View>::required_size(self, constraint)
    }
}

struct Required2Sizes<'a, I> {
    inner: &'a mut I,
}

impl<'a, I: InnerRequiredSize> InnerSizes for Required2Sizes<'a, I> {
    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }
}

trait InnerSizes {
    fn needs_relayout(&self) -> bool;
    fn required_size(&mut self, constraint: Vec2) -> Vec2;
}

impl<I: InnerLayout> InnerSizes for &mut I {
    fn needs_relayout(&self) -> bool {
        <I as InnerLayout>::needs_relayout(self)
    }
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        <I as InnerLayout>::required_size(self, constraint)
    }
}

/// Core system for scrolling views.
///
/// See also [`ScrollView`](crate::views::ScrollView).
pub struct ScrollCore {
    /// This is the size the child thinks we're giving him.
    inner_size: Vec2,

    /// Offset into the inner view.
    ///
    /// Our `(0,0)` will be inner's `offset`
    offset: Vec2,

    /// What was our own size last time we checked.
    ///
    /// This includes scrollbars, if any.
    last_size: Vec2,

    /// Are we scrollable in each direction?
    enabled: XY<bool>,

    /// Should we show scrollbars?
    ///
    /// Even if this is true, no scrollbar will be printed if we don't need to
    /// scroll.
    ///
    /// TODO: have an option to always show the scrollbar.
    /// TODO: have an option to show scrollbar on top/left.
    show_scrollbars: bool,

    /// How much padding should be between content and scrollbar?
    ///
    /// scrollbar_padding.x is the horizontal padding before the vertical scrollbar.
    scrollbar_padding: Vec2,

    /// Initial position of the cursor when dragging.
    thumb_grab: Option<(Orientation, usize)>,

    /// We keep the cache here so it can be busted when we change the content.
    size_cache: Option<XY<SizeCache>>,

    /// Defines how to update the offset when the view size changes.
    scroll_strategy: ScrollStrategy,
}

impl Default for ScrollCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollCore {
    /// Creates a new `ScrollCore`.
    pub fn new() -> Self {
        ScrollCore {
            inner_size: Vec2::zero(),
            offset: Vec2::zero(),
            last_size: Vec2::zero(),
            enabled: XY::new(false, true),
            show_scrollbars: true,
            scrollbar_padding: Vec2::new(1, 0),
            thumb_grab: None,
            size_cache: None,
            scroll_strategy: ScrollStrategy::KeepRow,
        }
    }

    /// Performs the `View::draw()` operation.
    pub fn draw<I: InnerDraw>(&self, printer: &Printer<'_, '_>, inner: I) {
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

        // Draw the scrollbars
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

        // Draw the X between the two scrollbars.
        if scrolling.both() {
            printer.print(printer.size.saturating_sub((1, 1)), "╳");
        }

        // Draw content
        let printer = printer
            .cropped(size)
            .content_offset(self.offset)
            .inner_size(self.inner_size);

        inner.draw(&printer);
    }

    /// Performs `View::on_event()`
    pub fn on_event<I: InnerOnEvent>(
        &mut self, event: Event, mut inner: I,
    ) -> EventResult {
        // Relativize event accorging to the offset
        let mut relative_event = event.clone();

        // Should the event be treated inside, by the inner view?
        let inside = if let Event::Mouse {
            ref mut position,
            ref offset,
            ..
        } = relative_event
        {
            // For mouse events, check if it falls inside the available area
            let inside = position
                .checked_sub(offset)
                .map(|p| p.fits_in(self.available_size()))
                .unwrap_or(false);
            *position = *position + self.offset;
            inside
        } else {
            // For key events, assume it's inside by default.
            true
        };

        let result = if inside {
            // If the event is inside, give it to the child.
            inner.on_event(relative_event)
        } else {
            // Otherwise, pretend it wasn't there.
            EventResult::Ignored
        };

        match result {
            EventResult::Ignored => {
                // If it's an arrow, try to scroll in the given direction.
                // If it's a mouse scroll, try to scroll as well.
                // Also allow Ctrl+arrow to move the view,
                // but not the selection.
                match event {
                    Event::Mouse {
                        event: MouseEvent::WheelUp,
                        ..
                    } if self.enabled.y && self.offset.y > 0 => {
                        self.offset.y = self.offset.y.saturating_sub(3);
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
                    }
                    Event::Mouse {
                        event: MouseEvent::Press(MouseButton::Left),
                        position,
                        offset,
                    } if self.show_scrollbars
                        && position
                            .checked_sub(offset)
                            .map(|position| self.start_drag(position))
                            .unwrap_or(false) =>
                    {
                        // Just consume the event.
                    }
                    Event::Mouse {
                        event: MouseEvent::Hold(MouseButton::Left),
                        position,
                        offset,
                    } if self.show_scrollbars => {
                        let position = position.saturating_sub(offset);
                        self.drag(position);
                    }
                    Event::Mouse {
                        event: MouseEvent::Release(MouseButton::Left),
                        ..
                    } => {
                        self.release_grab();
                    }
                    Event::Key(Key::Home) if self.enabled.any() => {
                        self.offset =
                            self.enabled.select_or(Vec2::zero(), self.offset);
                    }
                    Event::Key(Key::End) if self.enabled.any() => {
                        let max_offset = self
                            .inner_size
                            .saturating_sub(self.available_size());
                        self.offset =
                            self.enabled.select_or(max_offset, self.offset);
                    }
                    Event::Ctrl(Key::Up) | Event::Key(Key::Up)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y -= 1;
                    }
                    Event::Key(Key::PageUp)
                        if self.enabled.y && self.offset.y > 0 =>
                    {
                        self.offset.y = self.offset.y.saturating_sub(5);
                    }
                    Event::Key(Key::PageDown)
                        if self.enabled.y
                            && (self.offset.y + self.available_size().y
                                < self.inner_size.y) =>
                    {
                        self.offset.y += 5;
                    }
                    Event::Ctrl(Key::Down) | Event::Key(Key::Down)
                        if self.enabled.y
                            && (self.offset.y + self.available_size().y
                                < self.inner_size.y) =>
                    {
                        self.offset.y += 1;
                    }
                    Event::Ctrl(Key::Left) | Event::Key(Key::Left)
                        if self.enabled.x && self.offset.x > 0 =>
                    {
                        self.offset.x -= 1;
                    }
                    Event::Ctrl(Key::Right) | Event::Key(Key::Right)
                        if self.enabled.x
                            && (self.offset.x + self.available_size().x
                                < self.inner_size.x) =>
                    {
                        self.offset.x += 1;
                    }
                    _ => return EventResult::Ignored,
                };

                // We just scrolled manually, so reset the scroll strategy.
                self.scroll_strategy = ScrollStrategy::KeepRow;
                // TODO: return callback on_scroll?
                EventResult::Consumed(None)
            }
            other => {
                // Fix offset?
                let important = inner.important_area(self.inner_size);

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

    /// Performs `View::layout()`
    pub fn layout<I: InnerLayout>(&mut self, size: Vec2, mut inner: I) {
        // Size is final now, negociations are over.
        self.last_size = size;

        // This is what we'd like
        let (inner_size, self_size) =
            self.sizes(size, true, Layout2Sizes { inner: &mut inner });

        self.inner_size = inner_size;

        self.size_cache = Some(SizeCache::build(self_size, size));

        inner.layout(self.inner_size);

        // Keep the offset in the valid range.
        self.offset = self
            .offset
            .or_min(self.inner_size.saturating_sub(self.available_size()));

        // Possibly update the offset if we're following a specific strategy.
        self.adjust_scroll();
    }

    /// Performs `View::needs_relayout()`
    pub fn needs_relayout<F>(&self, inner_needs_relayout: F) -> bool
    where
        F: FnOnce() -> bool,
    {
        self.size_cache.is_none() || inner_needs_relayout()
    }

    /// Performs `View::required_size()`
    pub fn required_size<I: InnerRequiredSize>(
        &mut self, constraint: Vec2, mut inner: I,
    ) -> Vec2 {
        let (_, size) = self.sizes(
            constraint,
            false,
            Required2Sizes { inner: &mut inner },
        );

        size
    }

    /// Performs `View::call_on_any()`
    pub fn call_on_any<'a, F>(
        &mut self, selector: &Selector<'_>, cb: AnyCb<'a>,
        inner_call_on_any: F,
    ) where
        F: FnOnce(&Selector, AnyCb),
    {
        inner_call_on_any(selector, cb)
    }

    /// Performs `View::focus_view()`
    pub fn focus_view<F>(
        &mut self, selector: &Selector<'_>, inner_focus_view: F,
    ) -> Result<(), ()>
    where
        F: FnOnce(&Selector) -> Result<(), ()>,
    {
        inner_focus_view(selector)
    }

    /// Performs `View::take_focus()`
    pub fn take_focus<F>(
        &mut self, source: Direction, inner_take_focus: F,
    ) -> bool
    where
        F: FnOnce(Direction) -> bool,
    {
        let is_scrollable = self.is_scrolling().any();
        inner_take_focus(source) || is_scrollable
    }

    /// Returns the viewport in the inner content.
    pub fn content_viewport(&self) -> Rect {
        Rect::from_size(self.offset, self.available_size())
    }

    /// Defines the way scrolling is adjusted on content or size change.
    ///
    /// The scroll strategy defines how the scrolling position is adjusted
    /// when the size of the view or the content change.
    ///
    /// It is reset to `ScrollStrategy::KeepRow` whenever the user scrolls
    /// manually.
    pub fn set_scroll_strategy(&mut self, strategy: ScrollStrategy) {
        self.scroll_strategy = strategy;
        self.adjust_scroll();
    }

    /// Defines the way scrolling is adjusted on content or size change.
    ///
    /// Chainable variant.
    pub fn scroll_strategy(self, strategy: ScrollStrategy) -> Self {
        self.with(|s| s.set_scroll_strategy(strategy))
    }

    /// Control whether scroll bars are visibile.
    ///
    /// Defaults to `true`.
    pub fn set_show_scrollbars(&mut self, show_scrollbars: bool) {
        self.show_scrollbars = show_scrollbars;
    }

    /// Control whether scroll bars are visibile.
    ///
    /// Chainable variant
    pub fn show_scrollbars(self, show_scrollbars: bool) -> Self {
        self.with(|s| s.set_show_scrollbars(show_scrollbars))
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
        self.invalidate_cache();
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    pub fn set_scroll_x(&mut self, enabled: bool) {
        self.enabled.x = enabled;
        self.invalidate_cache();
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

    /// Clears the cache.
    fn invalidate_cache(&mut self) {
        self.size_cache = None;
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
    ///
    /// The scrollbar_size().x will be the horizontal space taken by the vertical scrollbar.
    fn scrollbar_size(&self) -> Vec2 {
        self.is_scrolling()
            .swap()
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero())
    }

    /// Returns the size available for the child view.
    fn available_size(&self) -> Vec2 {
        if self.show_scrollbars {
            self.last_size.saturating_sub(self.scrollbar_size())
        } else {
            self.last_size
        }
    }

    /// Compute the size we would need.
    ///
    /// Given the constraints, and the axis that need scrollbars.
    ///
    /// Returns `(inner_size, size, scrollable)`.
    fn sizes_when_scrolling<I: InnerSizes>(
        &mut self, constraint: Vec2, scrollable: XY<bool>, strict: bool,
        inner: &mut I,
    ) -> (Vec2, Vec2, XY<bool>) {
        // This is the size taken by the scrollbars.
        let scrollbar_size = scrollable
            .swap()
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero());

        let available = constraint.saturating_sub(scrollbar_size);

        // This the ideal size for the child. May not be what he gets.
        let inner_size = inner.required_size(available);

        // Where we're "enabled", accept the constraints.
        // Where we're not, just forward inner_size.
        let size = self.enabled.select_or(
            Vec2::min(inner_size + scrollbar_size, constraint),
            inner_size + scrollbar_size,
        );

        // In strict mode, there's no way our size is over constraints.
        let size = if strict {
            size.or_min(constraint)
        } else {
            size
        };

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
        // For each scrollbar, how far it is.
        let scrollbar_pos = self.last_size.saturating_sub((1, 1));
        let lengths = self.scrollbar_thumb_lengths();
        let offsets = self.scrollbar_thumb_offsets(lengths);
        let available = self.available_size();

        // This is true for Y if we grabbed the vertical scrollbar
        // More specifically, we need both (for instance for the vertical bar):
        // * To be in the right column: X == scrollbar_pos
        // * To be in the right range: Y < available
        let grabbed = position
            .zip_map(scrollbar_pos, |p, s| p == s)
            .swap()
            .and(position.zip_map(available, |p, a| p < a));

        // Iterate on axises, and keep the one we grabbed.
        if let Some((orientation, pos, length, offset)) =
            XY::zip4(Orientation::pair(), position, lengths, offsets)
                .keep(grabbed.and(self.enabled))
                .into_iter()
                .filter_map(|x| x)
                .next()
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

    /// Called when a mouse drag is detected.
    fn drag(&mut self, position: Vec2) {
        // Only do something if we grabbed something before.
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
        let extra =
            (available + (1, 1)).saturating_sub(lengths).or_max((1, 1));

        // We're dividing by this value, so make sure it's positive!
        assert!(extra > Vec2::zero());

        let new_offset =
            ((self.inner_size + (1, 1)).saturating_sub(available) * thumb_pos)
                .div_up(extra);
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
    /// Returns `(inner_size, desired_size)`
    fn sizes<I: InnerSizes>(
        &mut self, constraint: Vec2, strict: bool, mut inner: I,
    ) -> (Vec2, Vec2) {
        // First: try the cache
        let valid_cache = !inner.needs_relayout()
            && self
                .size_cache
                .map(|cache| {
                    cache.zip_map(constraint, SizeCache::accept).both()
                })
                .unwrap_or(false);

        if valid_cache {
            // eprintln!("Cache: {:?}; constraint: {:?}", self.size_cache, constraint);

            // The new constraint shouldn't change much,
            // so we can re-use previous values
            return (
                self.inner_size,
                self.size_cache.unwrap().map(|c| c.value),
            );
        }

        // Attempt 1: try without scrollbars
        let (inner_size, size, scrollable) = self.sizes_when_scrolling(
            constraint,
            XY::new(false, false),
            strict,
            &mut inner,
        );

        // If we need to add scrollbars, the available size will change.
        if scrollable.any() && self.show_scrollbars {
            // Attempt 2: he wants to scroll? Sure!
            // Try again with some space for the scrollbar.
            let (inner_size, size, new_scrollable) = self
                .sizes_when_scrolling(
                    constraint, scrollable, strict, &mut inner,
                );
            if scrollable == new_scrollable {
                // Yup, scrolling did it. We're good to go now.
                (inner_size, size)
            } else {
                // Again? We're now scrolling in a new direction?
                // There is no end to this!
                let (inner_size, size, _) = self.sizes_when_scrolling(
                    constraint,
                    new_scrollable,
                    strict,
                    &mut inner,
                );

                // That's enough. If the inner view changed again, ignore it!
                // That'll teach it.
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
        // The length should be (visible / total) * visible

        (available * available / self.inner_size.or_max((1, 1))).or_max((1, 1))
    }

    fn scrollbar_thumb_offsets(&self, lengths: Vec2) -> Vec2 {
        let available = self.available_size();
        // The number of steps is 1 + the "extra space"
        let steps = (available + (1, 1)).saturating_sub(lengths);
        let max_offset = self.inner_size.saturating_sub(available) + (1, 1);

        steps * self.offset / max_offset
    }

    /// Apply the scrolling strategy to the current scroll position.
    fn adjust_scroll(&mut self) {
        match self.scroll_strategy {
            ScrollStrategy::StickToTop => self.scroll_to_top(),
            ScrollStrategy::StickToBottom => self.scroll_to_bottom(),
            ScrollStrategy::KeepRow => (),
        }
    }
}
