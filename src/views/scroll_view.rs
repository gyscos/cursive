use crate::direction::Direction;
use crate::event::{AnyCb, Event, EventResult};
use crate::view::{scroll, ScrollStrategy, Selector, View};
use crate::{Printer, Rect, Vec2, With};

/// Wraps a view in a scrollable area.
pub struct ScrollView<V> {
    /// The wrapped view.
    inner: V,

    core: scroll::Core,
}

impl<V> scroll::Scroller for ScrollView<V>
where
    V: View,
{
    fn get_scroller(&self) -> &scroll::Core {
        &self.core
    }
    fn get_scroller_mut(&mut self) -> &mut scroll::Core {
        &mut self.core
    }
}

impl<V> ScrollView<V>
where
    V: View,
{
    /// Creates a new ScrollView around `view`.
    pub fn new(inner: V) -> Self {
        ScrollView {
            inner,
            core: scroll::Core::new(),
        }
    }

    /// Returns the viewport in the inner content.
    pub fn content_viewport(&self) -> Rect {
        self.core.content_viewport()
    }

    /// Defines the way scrolling is adjusted on content or size change.
    ///
    /// The scroll strategy defines how the scrolling position is adjusted
    /// when the size of the view or the content change.
    ///
    /// It is reset to `ScrollStrategy::KeepRow` whenever the user scrolls
    /// manually.
    pub fn set_scroll_strategy(&mut self, strategy: ScrollStrategy) {
        self.core.set_scroll_strategy(strategy);
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
        self.core.set_show_scrollbars(show_scrollbars);
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
        self.core.set_offset(offset);
    }

    /// Controls whether this view can scroll vertically.
    ///
    /// Defaults to `true`.
    pub fn set_scroll_y(&mut self, enabled: bool) {
        self.core.set_scroll_y(enabled);
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    pub fn set_scroll_x(&mut self, enabled: bool) {
        self.core.set_scroll_x(enabled);
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
        self.core.scroll_to_top();
    }

    /// Programmatically scroll to the bottom of the view.
    pub fn scroll_to_bottom(&mut self) {
        self.core.scroll_to_bottom();
    }

    /// Programmatically scroll to the leftmost side of the view.
    pub fn scroll_to_left(&mut self) {
        self.core.scroll_to_left();
    }

    /// Programmatically scroll to the rightmost side of the view.
    pub fn scroll_to_right(&mut self) {
        self.core.scroll_to_right();
    }

    /// Returns the wrapped view.
    pub fn into_inner(self) -> V {
        self.inner
    }

    inner_getters!(self.inner: V);
}

impl<V> View for ScrollView<V>
where
    V: View,
{
    fn draw(&self, printer: &Printer<'_, '_>) {
        scroll::draw(self, printer, |s, p| s.inner.draw(p));
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        scroll::on_event(
            self,
            event,
            |s, e| s.inner.on_event(e),
            |s, si| s.inner.important_area(si),
        )
    }

    fn layout(&mut self, size: Vec2) {
        scroll::layout(
            self,
            size,
            self.inner.needs_relayout(),
            |s, si| s.inner.layout(si),
            |s, c| s.inner.required_size(c),
        );
    }

    fn needs_relayout(&self) -> bool {
        self.core.needs_relayout() || self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        scroll::required_size(
            self,
            constraint,
            self.inner.needs_relayout(),
            |s, c| s.inner.required_size(c),
        )
    }

    fn call_on_any<'a>(&mut self, selector: &Selector<'_>, cb: AnyCb<'a>) {
        self.inner.call_on_any(selector, cb)
    }

    fn focus_view(&mut self, selector: &Selector<'_>) -> Result<(), ()> {
        self.inner.focus_view(selector)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.inner.take_focus(source) || self.core.is_scrolling().any()
    }

    fn important_area(&self, size: Vec2) -> Rect {
        scroll::important_area(self, size, |s, si| s.inner.important_area(si))
    }
}
