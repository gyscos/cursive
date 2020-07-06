use std::cmp::min;

use crate::direction::Orientation;
use crate::event::{AnyCb, Event, EventResult, Key, MouseButton, MouseEvent};
use crate::printer::Printer;
use crate::rect::Rect;
use crate::theme::ColorStyle;
use crate::view::{ScrollStrategy, Selector, SizeCache};
use crate::with::With;
use crate::Vec2;
use crate::XY;

/// Describes an item with a scroll core.
///
/// This trait is used to represent "something that can scroll".
/// All it needs is an accessible core.
///
/// See the various methods in the [`scroll`](crate::view::scroll) module.
pub trait Scroller {
    /// Returns a mutable access to the scroll core.
    fn get_scroller_mut(&mut self) -> &mut Core;

    /// Returns an immutable access to the scroll core.
    fn get_scroller(&self) -> &Core;
}

/// Implements the `Scroller` trait for any type.
#[macro_export]
macro_rules! impl_scroller {
    ($class:ident :: $core:ident) => {
        impl $crate::view::scroll::Scroller for $class {
            fn get_scroller_mut(
                &mut self,
            ) -> &mut $crate::view::scroll::Core {
                &mut self.$core
            }
            fn get_scroller(&self) -> &$crate::view::scroll::Core {
                &self.$core
            }
        }
    };
    ($class:ident < $($args:tt),* > :: $core:ident) => {
        impl <$( $args ),* > $crate::view::scroll::Scroller for $class<$($args),*> {

            fn get_scroller_mut(
                &mut self,
            ) -> &mut $crate::view::scroll::Core {
                &mut self.$core
            }
            fn get_scroller(&self) -> &$crate::view::scroll::Core {
                &self.$core
            }
        }
    };
}

/// Core system for scrolling views.
///
/// This is the lowest-level element handling scroll logic.
///
/// Higher-level abstractions are probably what you're after.
///
/// In particular, see also [`ScrollView`](crate::views::ScrollView).
#[derive(Debug)]
pub struct Core {
    /// This is the size the child thinks we're giving him.
    inner_size: Vec2,

    /// Offset into the inner view.
    ///
    /// Our `(0,0)` will be inner's `offset`
    offset: Vec2,

    /// What was the size available to print the child last time?
    ///
    /// Excludes any potential scrollbar.
    last_available: Vec2,

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
    size_cache: Option<XY<SizeCache<bool>>>,

    /// Defines how to update the offset when the view size changes.
    scroll_strategy: ScrollStrategy,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    /// Creates a new `Core`.
    pub fn new() -> Self {
        Core {
            inner_size: Vec2::zero(),
            offset: Vec2::zero(),
            last_available: Vec2::zero(),
            enabled: XY::new(false, true),
            show_scrollbars: true,
            scrollbar_padding: Vec2::new(1, 0),
            thumb_grab: None,
            size_cache: None,
            scroll_strategy: ScrollStrategy::KeepRow,
        }
    }

    /// Returns a sub-printer ready to draw the content.
    pub fn sub_printer<'a, 'b>(
        &self,
        printer: &Printer<'a, 'b>,
    ) -> Printer<'a, 'b> {
        // Draw scrollbar?

        let size = self.available_size();

        // Draw the scrollbars
        if self.get_show_scrollbars() {
            let scrolling = self.is_scrolling();

            let lengths = self.scrollbar_thumb_lengths();
            let offsets = self.scrollbar_thumb_offsets(lengths);

            let line_c = XY::new("-", "|");

            let color = if printer.focused {
                ColorStyle::highlight()
            } else {
                ColorStyle::highlight_inactive()
            };

            XY::zip5(lengths, offsets, size, line_c, Orientation::pair())
                .run_if(
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
        }

        // Draw content
        printer
            .cropped(size)
            .content_offset(self.offset)
            .inner_size(self.inner_size)
    }

    /// Returns `true` if `event` should be processed by the content.
    ///
    /// This also updates `event` so that it is relative to the content.
    pub fn is_event_inside(&self, event: &mut Event) -> bool {
        if let Event::Mouse {
            ref mut position,
            ref offset,
            ..
        } = event
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
        }
    }

    /// Handle an event after processing by the content.
    pub fn on_inner_event(
        &mut self,
        event: Event,
        inner_result: EventResult,
        important_area: Rect,
    ) -> EventResult {
        match inner_result {
            EventResult::Ignored => {
                // The view ignored the event, so we're free to use it.

                // If it's an arrow, try to scroll in the given direction.
                // If it's a mouse scroll, try to scroll as well.
                // Also allow Ctrl+arrow to move the view,
                // without affecting the selection.
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
                // The view consumed the event. Maybe something changed?

                self.scroll_to_rect(important_area);

                other
            }
        }
    }

    /// Specifies the size given in a layout phase.
    pub(crate) fn set_last_size(
        &mut self,
        last_size: Vec2,
        scrolling: XY<bool>,
    ) {
        self.last_available = last_size.saturating_sub(
            scrolling
                .swap()
                .select_or(self.scrollbar_padding + (1, 1), Vec2::zero()),
        );
    }

    /// Specifies the size allocated to the content.
    pub(crate) fn set_inner_size(&mut self, inner_size: Vec2) {
        self.inner_size = inner_size;
    }

    /// Rebuild the cache with the given parameters.
    pub(crate) fn build_cache(
        &mut self,
        self_size: Vec2,
        last_size: Vec2,
        scrolling: XY<bool>,
    ) {
        self.size_cache =
            Some(SizeCache::build_extra(self_size, last_size, scrolling));
    }

    /// Makes sure the viewport is within the content.
    pub(crate) fn update_offset(&mut self) {
        // Keep the offset in the valid range.
        self.offset = self
            .offset
            .or_min(self.inner_size.saturating_sub(self.available_size()));

        // Possibly update the offset if we're following a specific strategy.
        self.adjust_scroll();
    }

    /// Returns `true` if we should relayout, no matter the content.
    ///
    /// Even if this returns `false`, the content itself might still needs to relayout.
    pub fn needs_relayout(&self) -> bool {
        self.size_cache.is_none()
    }

    /// Performs `View::call_on_any()`
    pub fn call_on_any<'a, F>(
        &mut self,
        selector: &Selector<'_>,
        cb: AnyCb<'a>,
        inner_call_on_any: F,
    ) where
        F: FnOnce(&Selector, AnyCb),
    {
        inner_call_on_any(selector, cb)
    }

    /// Performs `View::focus_view()`
    pub fn focus_view<F>(
        &mut self,
        selector: &Selector<'_>,
        inner_focus_view: F,
    ) -> Result<(), ()>
    where
        F: FnOnce(&Selector) -> Result<(), ()>,
    {
        inner_focus_view(selector)
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

    /// Sets the padding between content and scrollbar.
    pub fn set_scrollbar_padding<V: Into<Vec2>>(
        &mut self,
        scrollbar_padding: V,
    ) {
        self.scrollbar_padding = scrollbar_padding.into();
    }

    /// Sets the padding between content and scrollbar.
    ///
    /// Chainable variant.
    pub fn scrollbar_padding<V: Into<Vec2>>(
        self,
        scrollbar_padding: V,
    ) -> Self {
        self.with(|s| s.set_scrollbar_padding(scrollbar_padding))
    }

    /// Returns the padding between content and scrollbar.
    pub fn get_scrollbar_padding(&self) -> Vec2 {
        self.scrollbar_padding
    }

    /// For each axis, returns `true` if this view can scroll.
    ///
    /// For example, a vertically-scrolling view will return
    /// `XY { x: false, y: true }`.
    pub fn is_enabled(&self) -> XY<bool> {
        self.enabled
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

    /// Returns `true` if we will show scrollbars when needed.
    ///
    /// Scrollbars are always hidden when not needed.
    pub fn get_show_scrollbars(&self) -> bool {
        self.show_scrollbars
    }

    /// Returns the size given to the content on the last layout phase.
    pub fn inner_size(&self) -> Vec2 {
        self.inner_size
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

    /// Try to keep the given `rect` in view.
    pub fn keep_in_view(&mut self, rect: Rect) {
        let min = rect.bottom_right().saturating_sub(self.available_size());
        let max = rect.top_left();
        let (min, max) = (Vec2::min(min, max), Vec2::max(min, max));

        self.offset = self.offset.or_min(max).or_max(min);
    }

    /// Scrolls until the given rect is in view.
    pub fn scroll_to_rect(&mut self, important_area: Rect) {
        // The furthest top-left we can go
        let top_left = (important_area.bottom_right() + (1, 1))
            .saturating_sub(self.available_size());
        // The furthest bottom-right we can go
        let bottom_right = important_area.top_left();

        // "top_left < bottom_right" is NOT guaranteed
        // if the child is larger than the view.
        let offset_min = Vec2::min(top_left, bottom_right);
        let offset_max = Vec2::max(top_left, bottom_right);

        self.offset = self.offset.or_max(offset_min).or_min(offset_max);
    }

    /// Scroll until the given point is visible.
    pub fn scroll_to(&mut self, pos: Vec2) {
        // The furthest top-left we can go
        let min = pos.saturating_sub(self.available_size());
        // How far to the bottom-right we can go
        let max = pos;

        self.offset = self.offset.or_min(max).or_max(min);
    }

    /// Scroll until the given column is visible.
    pub fn scroll_to_x(&mut self, x: usize) {
        if x >= self.offset.x + self.available_size().x {
            self.offset.x = 1 + x - self.available_size().x;
        } else if x < self.offset.x {
            self.offset.x = x;
        }
    }

    /// Scroll until the given row is visible.
    pub fn scroll_to_y(&mut self, y: usize) {
        if y >= self.offset.y + self.available_size().y {
            self.offset.y = 1 + y - self.available_size().y;
        } else if y < self.offset.y {
            self.offset.y = y;
        }
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
    pub fn is_scrolling(&self) -> XY<bool> {
        self.inner_size.zip_map(self.available_size(), |i, s| i > s)
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
    pub fn scrollbar_size(&self) -> Vec2 {
        self.is_scrolling()
            .swap()
            .select_or(self.scrollbar_padding + (1, 1), Vec2::zero())
    }

    /// Returns the size available for the child view.
    fn available_size(&self) -> Vec2 {
        self.last_available
    }

    /// Returns the last size given by `layout`.
    pub fn last_outer_size(&self) -> Vec2 {
        self.available_size() + self.scrollbar_size()
    }

    /// Starts scrolling from the cursor position.
    ///
    /// Returns `true` if the event was consumed.
    fn start_drag(&mut self, position: Vec2) -> bool {
        // For each scrollbar, how far it is.
        let scrollbar_pos = self.last_outer_size().saturating_sub((1, 1));
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

    /// Tries to apply the cache to the current constraint.
    ///
    /// Returns the cached value if it works, or `None`.
    pub(crate) fn try_cache(
        &self,
        constraint: Vec2,
    ) -> Option<(Vec2, Vec2, XY<bool>)> {
        self.size_cache.and_then(|cache| {
            if cache.zip_map(constraint, SizeCache::accept).both() {
                Some((
                    self.inner_size,
                    cache.map(|c| c.value),
                    cache.map(|c| c.extra),
                ))
            } else {
                None
            }
        })
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
