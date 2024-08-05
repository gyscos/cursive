use crate::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    view::{scroll, CannotFocus, ScrollStrategy, Selector, View, ViewNotFound},
    Cursive, Printer, Rect, Vec2, With,
};

use std::sync::Arc;

type InnerScrollCallback<V> = dyn Fn(&mut ScrollView<V>, Rect) -> EventResult + Send + Sync;
type ScrollCallback = dyn Fn(&mut Cursive, Rect) + Send + Sync;

/// Wraps a view in a scrollable area.
pub struct ScrollView<V> {
    /// The wrapped view.
    inner: V,

    core: scroll::Core,

    on_scroll: Arc<InnerScrollCallback<V>>,
}

new_default!(ScrollView<V: Default>);

impl_scroller!(ScrollView<V>::core);

impl<V> ScrollView<V> {
    /// Creates a new ScrollView around `view`.
    pub fn new(inner: V) -> Self {
        ScrollView {
            inner,
            core: scroll::Core::new(),
            on_scroll: Arc::new(|_, _| EventResult::Ignored),
        }
    }
}
impl<V: 'static> ScrollView<V> {
    /// Returns the viewport in the inner content.
    pub fn content_viewport(&self) -> Rect {
        self.core.content_viewport()
    }

    /// Returns the size of the content view, as it was on the last layout
    /// phase.
    ///
    /// This is only the size the content _thinks_ it has, and may be larger
    /// than the actual size used by this `ScrollView`.
    pub fn inner_size(&self) -> Vec2 {
        self.core.inner_size()
    }

    /// Returns `true` if the top row of the content is in view.
    pub fn is_at_top(&self) -> bool {
        self.content_viewport().top() == 0
    }

    /// Returns `true` if the bottom row of the content is in view.
    pub fn is_at_bottom(&self) -> bool {
        // The viewport indicates which row is in view.
        // So the bottom row will be (height - 1)
        (1 + self.content_viewport().bottom()) >= self.inner_size().y
    }

    /// Return `true` if the left-most column of the content is in view.
    pub fn is_at_left_edge(&self) -> bool {
        self.content_viewport().left() == 0
    }

    /// Return `true` if the right-most column of the content is in view.
    pub fn is_at_right_edge(&self) -> bool {
        // The viewport indicates which row is in view.
        // So the right-most column will be (width - 1)
        (1 + self.content_viewport().right()) >= self.inner_size().x
    }

    /// Defines the way scrolling is adjusted on content or size change.
    ///
    /// The scroll strategy defines how the scrolling position is adjusted
    /// when the size of the view or the content change.
    ///
    /// It is reset to `ScrollStrategy::KeepRow` whenever the user scrolls
    /// manually.
    pub fn set_scroll_strategy(&mut self, strategy: ScrollStrategy) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| s.core.set_scroll_strategy(strategy))
    }

    /// Defines the way scrolling is adjusted on content or size change.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn scroll_strategy(self, strategy: ScrollStrategy) -> Self
    where
        V: View,
    {
        self.with(|s| {
            s.set_scroll_strategy(strategy);
        })
    }

    /// Control whether scroll bars are visible.
    ///
    /// Defaults to `true`.
    pub fn set_show_scrollbars(&mut self, show_scrollbars: bool) {
        self.core.set_show_scrollbars(show_scrollbars);
    }

    /// Control whether scroll bars are visible.
    ///
    /// Chainable variant
    #[must_use]
    pub fn show_scrollbars(self, show_scrollbars: bool) -> Self {
        self.with(|s| s.set_show_scrollbars(show_scrollbars))
    }

    /// Sets the scroll offset to the given value
    pub fn set_offset<S>(&mut self, offset: S) -> EventResult
    where
        V: View,
        S: Into<Vec2>,
    {
        self.scroll_operation(|s| s.core.set_offset(offset))
    }

    /// Controls whether this view can scroll vertically.
    ///
    /// Defaults to `true`.
    pub fn set_scroll_y(&mut self, enabled: bool) -> EventResult {
        self.core.set_scroll_y(enabled);

        // Egh~~ this is not actually a scroll operation, it cannot _by itself_ cause a callback.
        self.on_scroll_callback()
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    pub fn set_scroll_x(&mut self, enabled: bool) -> EventResult {
        self.core.set_scroll_x(enabled);

        self.on_scroll_callback()
    }

    /// Controls whether this view can scroll vertically.
    ///
    /// Defaults to `true`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn scroll_y(self, enabled: bool) -> Self {
        self.with(|s| {
            s.set_scroll_y(enabled);
        })
    }

    /// Controls whether this view can scroll horizontally.
    ///
    /// Defaults to `false`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn scroll_x(self, enabled: bool) -> Self {
        self.with(|s| {
            s.set_scroll_x(enabled);
        })
    }

    /// Programmatically scroll to the top of the view.
    pub fn scroll_to_top(&mut self) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| s.core.scroll_to_top())
    }

    /// Programmatically scroll to the bottom of the view.
    pub fn scroll_to_bottom(&mut self) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| s.core.scroll_to_bottom())
    }

    /// Programmatically scroll to the leftmost side of the view.
    pub fn scroll_to_left(&mut self) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| s.core.scroll_to_left())
    }

    /// Programmatically scroll to the rightmost side of the view.
    pub fn scroll_to_right(&mut self) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| s.core.scroll_to_right())
    }

    /// Programmatically scroll until the child's important area is in view.
    pub fn scroll_to_important_area(&mut self) -> EventResult
    where
        V: View,
    {
        self.scroll_operation(|s| {
            let important_area = s.inner.important_area(s.core.last_outer_size());
            s.core.scroll_to_rect(important_area)
        })
    }

    /// Returns the wrapped view.
    pub fn into_inner(self) -> V {
        self.inner
    }

    /// Sets a callback to be run whenever scrolling happens.
    ///
    /// This lets the callback access the `ScrollView` itself (and its child)
    /// if necessary.
    ///
    /// If you just need to run a callback on `&mut Cursive`, consider
    /// `set_on_scroll`.
    #[crate::callback_helpers]
    pub fn set_on_scroll_inner<F>(&mut self, on_scroll: F)
    where
        F: FnMut(&mut Self, Rect) -> EventResult + 'static + Send + Sync,
    {
        self.on_scroll = Arc::new(immut2!(on_scroll; else EventResult::Ignored));
    }

    /// Sets a callback to be run whenever scrolling happens.
    #[crate::callback_helpers]
    pub fn set_on_scroll<F>(&mut self, on_scroll: F)
    where
        F: FnMut(&mut Cursive, Rect) + 'static + Send + Sync,
    {
        let on_scroll: Arc<ScrollCallback> = Arc::new(immut2!(on_scroll));

        self.set_on_scroll_inner(move |_, rect| {
            let on_scroll = Arc::clone(&on_scroll);
            EventResult::with_cb(move |siv| on_scroll(siv, rect))
        })
    }

    /// Wrap a function and only calls it if the second parameter changed.
    ///
    /// Not 100% generic, only works for our use-case here.
    fn skip_unchanged<F, T, R, I>(
        mut f: F,
        mut if_skipped: I,
    ) -> impl for<'a> FnMut(&'a mut T, Rect) -> R + Send + Sync
    where
        F: for<'a> FnMut(&'a mut T, Rect) -> R + 'static + Send + Sync,
        I: FnMut() -> R + 'static + Send + Sync,
    {
        let mut previous = Rect::from_size((0, 0), (0, 0));
        move |t, r| {
            if r != previous {
                previous = r;
                f(t, r)
            } else {
                if_skipped()
            }
        }
    }

    /// Sets a callback to be run whenever the scroll offset changes.
    #[crate::callback_helpers]
    pub fn set_on_scroll_change_inner<F>(&mut self, on_scroll: F)
    where
        F: FnMut(&mut Self, Rect) -> EventResult + 'static + Send + Sync,
    {
        self.set_on_scroll_inner(Self::skip_unchanged(on_scroll, || EventResult::Ignored));
    }

    /// Sets a callback to be run whenever the scroll offset changes.
    #[crate::callback_helpers]
    pub fn set_on_scroll_change<F>(&mut self, on_scroll: F)
    where
        F: FnMut(&mut Cursive, Rect) + 'static + Send + Sync,
    {
        self.set_on_scroll(Self::skip_unchanged(on_scroll, || ()));
    }

    /// Sets a callback to be run whenever scrolling happens.
    ///
    /// This lets the callback access the `ScrollView` itself (and its child)
    /// if necessary.
    ///
    /// If you just need to run a callback on `&mut Cursive`, consider
    /// `set_on_scroll`.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_scroll_inner<F>(self, on_scroll: F) -> Self
    where
        F: Fn(&mut Self, Rect) -> EventResult + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_scroll_inner(on_scroll))
    }

    /// Sets a callback to be run whenever scrolling happens.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn on_scroll<F>(self, on_scroll: F) -> Self
    where
        F: FnMut(&mut crate::Cursive, Rect) + 'static + Send + Sync,
    {
        self.with(|s| s.set_on_scroll(on_scroll))
    }

    fn scroll_operation<F>(&mut self, f: F) -> EventResult
    where
        V: View,
        F: FnOnce(&mut Self),
    {
        self.refresh();

        f(self);

        self.on_scroll_callback()
    }

    fn refresh(&mut self)
    where
        V: View,
    {
        // Note: the child may have changed since the last call to layout().
        // We really should update that.
        // Ideally, we would only fetch the last inner size and update that.
        self.layout(self.core.last_outer_size());
    }

    /// Run any callback after scrolling.
    fn on_scroll_callback(&mut self) -> EventResult {
        let viewport = self.content_viewport();
        let on_scroll = Arc::clone(&self.on_scroll);
        (on_scroll)(self, viewport)
    }

    inner_getters!(self.inner: V);
}

impl<V> View for ScrollView<V>
where
    V: View,
{
    fn draw(&self, printer: &Printer) {
        scroll::draw(self, printer, |s, p| s.inner.draw(p));
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match scroll::on_event(
            self,
            event,
            |s, e| s.inner.on_event(e),
            |s, si| s.inner.important_area(si),
        ) {
            EventResult::Ignored => EventResult::Ignored,
            // If the event was consumed, then we may have scrolled.
            other => other.and(self.on_scroll_callback()),
        }
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
        scroll::required_size(self, constraint, self.inner.needs_relayout(), |s, c| {
            s.inner.required_size(c)
        })
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        // TODO: should we scroll_to_important_area here?
        // The callback may change the focus or some other thing.
        self.inner.call_on_any(selector, cb)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.inner.focus_view(selector).map(|res| {
            self.scroll_to_important_area();
            res
        })
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        // If the inner view takes focus, re-align the important area.
        match self.inner.take_focus(source) {
            Ok(res) => {
                // Don't do anything if we come from `None`
                if source != Direction::none() {
                    self.scroll_to_important_area();

                    // Note: we can't really return an `EventResult` here :(
                    self.on_scroll_callback();
                }
                Ok(res)
            }
            Err(CannotFocus) => self
                .core
                .is_scrolling()
                .any()
                .then(EventResult::consumed)
                .ok_or(CannotFocus),
        }
    }

    fn important_area(&self, size: Vec2) -> Rect {
        scroll::important_area(self, size, |s, si| s.inner.important_area(si))
    }
}

#[crate::blueprint(ScrollView::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,

    scroll_x: Option<bool>,
    scroll_y: Option<bool>,
    scroll_strategy: Option<ScrollStrategy>,
    show_scrollbars: Option<bool>,

    on_scroll: Option<_>,
    on_scroll_inner: Option<_>,

    on_scroll_change: Option<_>,
    on_scroll_change_inner: Option<_>,
}

// ```yaml
// - TextView
//     content: $content
//     with:
//         - name: text
//         - scroll: true
// ```
crate::manual_blueprint!(with scroll, |config, context| {
    use crate::builder::{Config, Error};

    // Value could be:
    // - Null (y-scroll)
    // - Boolean (y-scroll?)
    // - Array of strings [x y]
    // - XY<bool>
    //      - Array of booleans
    //      - x: bool, y: bool
    // TODO: Simplify? Re-use Resolvable for XY<bool>?
    let (x, y) = match config {
        Config::Null => (false, true),
        Config::Bool(b) => (false, *b),
        Config::String(s) if s == "x" => (true, false),
        Config::String(s) if s == "y" => (false, true),
        Config::Array(array) if array.len() <= 2 => {
            let mut xy = [false, false];

            // Sooo right now we allow `- scroll: [x, true]` and it'll return (true, true)
            // Do we care enough to reject it?
            for (i, value) in array.iter().enumerate() {
                match value {
                    Config::String(v) if v == "x" => xy[0] = true,
                    Config::String(v) if v == "y" => xy[1] = true,
                    // Anything else we try to resolve as bool. Maybe variable?
                    other => xy[i] = context.resolve(other)?,
                }
            }

            (xy[0], xy[1])
        }
        Config::Object(_) => {
            let x = context.resolve_or(&config["x"], false)?;
            let y = context.resolve_or(&config["y"], false)?;
            (x, y)
        }
        _ => return Err(Error::invalid_config("Expected differently", config)),
    };

    Ok(move |view| ScrollView::new(view).scroll_x(x).scroll_y(y))
});
