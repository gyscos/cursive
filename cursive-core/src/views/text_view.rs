use parking_lot::Mutex;
use std::ops::Deref;
use std::sync::Arc;

use unicode_width::UnicodeWidthStr;

use crate::align::*;
use crate::style::{Effect, StyleType};
use crate::utils::lines::spans::{LinesIterator, Row};
use crate::utils::markup::StyledString;
use crate::view::layout_pass::current_pass;
use crate::view::{View, combine_layout_key, fresh_layout_key};
use crate::{Printer, Vec2, With};

// Content type used internally for caching and storage
type InnerContentType = Arc<StyledString>;

/// Provides access to the content of a [`TextView`].
///
/// [`TextView`]: struct.TextView.html
///
/// Cloning this object will still point to the same content.
///
/// # Examples
///
/// ```rust
/// # use cursive_core::views::{TextView, TextContent};
/// let mut content = TextContent::new("content");
/// let view = TextView::new_with_content(content.clone());
///
/// // Later, possibly in a different thread
/// content.set_content("new content");
/// assert!(view.get_content().source().contains("new"));
/// ```
#[derive(Clone)]
pub struct TextContent {
    content: Arc<Mutex<TextContentInner>>,
}

impl TextContent {
    /// Creates a new text content around the given value.
    ///
    /// Parses the given value.
    pub fn new<S>(content: S) -> Self
    where
        S: Into<StyledString>,
    {
        let content = Arc::new(content.into());
        let version = fresh_layout_key();

        TextContent {
            content: Arc::new(Mutex::new(TextContentInner {
                content_value: content,
                version,
                replaced_version: version,
            })),
        }
    }
}

/// A reference to the text content.
///
/// This can be deref'ed into a [`StyledString`].
///
/// [`StyledString`]: ../utils/markup/type.StyledString.html
pub struct TextContentRef {
    // We also need to keep a copy of Arc so `deref` can return
    // a reference to the `StyledString`
    data: Arc<StyledString>,
}

impl Deref for TextContentRef {
    type Target = StyledString;

    fn deref(&self) -> &StyledString {
        self.data.as_ref()
    }
}

impl TextContent {
    /// Replaces the content with the given value.
    pub fn set_content<S>(&self, content: S)
    where
        S: Into<StyledString>,
    {
        self.with_content(|c| {
            *c = content.into();
        });
    }

    /// Append `content` to the end of a `TextView`.
    pub fn append<S>(&self, content: S)
    where
        S: Into<StyledString>,
    {
        // Views sharing this content can then keep the rows they computed.
        self.with_content_inner(true, |c| {
            // This will only clone content if content_cached and content_value
            // are sharing the same underlying Rc.
            Arc::make_mut(&mut c.content_value).append(content);
        })
    }

    /// Returns a reference to the content.
    ///
    /// This locks the data while the returned value is alive,
    /// so don't keep it too long.
    pub fn get_content(&self) -> TextContentRef {
        TextContentInner::get_content(&self.content)
    }

    /// Apply the given closure to the inner content, and bust the cache afterward.
    pub fn with_content<F, O>(&self, f: F) -> O
    where
        F: FnOnce(&mut StyledString) -> O,
    {
        self.with_content_inner(false, |c| f(Arc::make_mut(&mut c.content_value)))
    }

    /// Apply the given closure to the inner content, and bump its version.
    fn with_content_inner<F, O>(&self, appended: bool, f: F) -> O
    where
        F: FnOnce(&mut TextContentInner) -> O,
    {
        let mut content = self.content.lock();

        let out = f(&mut content);

        content.version = fresh_layout_key();
        if !appended {
            content.replaced_version = content.version;
        }

        out
    }

    // The current content, its version, and the version of the last change
    // that wasn't just appending to it.
    fn snapshot(&self) -> (InnerContentType, u64, u64) {
        let content = self.content.lock();
        (
            Arc::clone(&content.content_value),
            content.version,
            content.replaced_version,
        )
    }

    fn version(&self) -> u64 {
        self.content.lock().version
    }

    // The length of the current content, in bytes.
    fn len(&self) -> usize {
        self.content.lock().content_value.source().len()
    }
}

/// Internal representation of the content for a `TextView`.
///
/// This is mostly just a `StyledString`.
///
/// Can be shared (through a `Arc<Mutex>`).
struct TextContentInner {
    // This is what `set_content` changes.
    content_value: InnerContentType,

    // Changes whenever `content_value` does.
    //
    // Several views can share this content: each keeps track of the version
    // it computed its rows from, rather than relying on a shared cache.
    version: u64,

    // The version of the last change that wasn't just appending: content at
    // any version since then is a prefix of the current one.
    replaced_version: u64,
}

impl TextContentInner {
    /// From a shareable content (Arc + Mutex), return a
    fn get_content(content: &Arc<Mutex<TextContentInner>>) -> TextContentRef {
        let data = Arc::clone(&content.lock().content_value);
        TextContentRef { data }
    }
}

/// A simple view showing a text.
///
/// The text can be given in three ways:
///
/// * Fixed: [`TextView::new`] (and [`set_content`](TextView::set_content) to
///   replace it from the view).
/// * Shared: a [`TextContent`] handle (see
///   [`new_with_content`](TextView::new_with_content)) lets any other code
///   update the text, possibly from another thread.
/// * Live: [`TextView::live`] takes a closure, re-evaluated on each screen
///   refresh, for text derived from shared application state.
///
/// # Examples
///
/// ```rust
/// # use cursive_core::Cursive;
/// # use cursive_core::views::TextView;
/// let mut siv = Cursive::new();
///
/// siv.add_layer(TextView::new("Hello world!"));
/// ```
pub struct TextView {
    // Possibly shared content
    content: TextContent,

    // The content `rows` were computed from (the shared content may have
    // changed since), and its version.
    snapshot: InnerContentType,
    snapshot_version: u64,

    // Pre-computed rows for `snapshot`, based on the last view size.
    rows: Vec<Row>,

    // The width `rows` were computed for (`usize::MAX` without wrapping).
    rows_width: Option<usize>,

    // Whether some text was left out of `rows` (wider than a whole row).
    truncated: bool,

    // `snapshot_version` at the last `layout()`.
    laid_out_version: Option<u64>,

    // Sizes computed for `snapshot` at other widths (`(width, size)`), so
    // `required_size` can answer without wrapping the text again: parents
    // often ask for several widths during a single layout.
    sizes: Vec<(usize, Vec2)>,

    // Closure the content comes from, if this view is live.
    source: Option<Source>,

    // Text alignment
    align: Align,

    // Default style for the text.
    //
    // Note that the text itself can be styled, which will override this.
    style: StyleType,

    // True if we can wrap long lines.
    wrap: bool,

    // Last requested width.
    //
    // Usually the longest row, but if a row had to be wrapped, it may be a bit larger.
    width: Option<usize>,
    // Selection?
    // selection: Option<Selection>,
}

// A closure giving the content of a live `TextView`.
struct Source {
    // Evaluated at most once per layout pass.
    f: Box<dyn Fn() -> StyledString + Send + Sync>,
    // The layout pass `f` last ran in (unset outside of any pass).
    evaluated_in: Mutex<Option<u64>>,
}

// struct Selection {
//     segments: Vec<crate::utils::lines::spans::Segment>,
//     // dragging?
// }

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new<S>(content: S) -> Self
    where
        S: Into<StyledString>,
    {
        Self::new_with_content(TextContent::new(content))
    }

    /// Convenient function to create a TextView by parsing the given content as cursup.
    ///
    /// Shortcut for `TextView::new(cursup::parse(content))`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive_core::views::TextView;
    /// let view = TextView::cursup("/red+bold{warning}");
    /// ```
    pub fn cursup<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(crate::utils::markup::cursup::parse(content))
    }

    /// Creates a new TextView using the given `TextContent`.
    ///
    /// If you kept a clone of the given content, you'll be able to update it
    /// remotely.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive_core::views::{TextView, TextContent};
    /// let mut content = TextContent::new("content");
    /// let view = TextView::new_with_content(content.clone());
    ///
    /// // Later, possibly in a different thread
    /// content.set_content("new content");
    /// assert!(view.get_content().source().contains("new"));
    /// ```
    pub fn new_with_content(content: TextContent) -> Self {
        TextView {
            content,
            style: StyleType::default(),
            snapshot: Arc::new(StyledString::default()),
            snapshot_version: 0,
            rows: Vec::new(),
            rows_width: None,
            truncated: false,
            laid_out_version: None,
            sizes: Vec::new(),
            source: None,
            wrap: true,
            align: Align::top_left(),
            width: None,
        }
    }

    /// Creates a new empty `TextView`.
    pub fn empty() -> Self {
        TextView::new("")
    }

    /// Creates a text view showing whatever the closure returns, re-evaluated
    /// on each refresh.
    ///
    /// The text follows state that changes from anywhere (a shared counter, a
    /// mutex-protected struct, ...) without having to push a new string to a
    /// [`TextContent`] each time. It is shown like any other `TextView` text:
    /// same wrapping, alignment and styling, and styled content keeps its
    /// styles.
    ///
    /// # When the closure runs
    ///
    /// The closure runs at most once per layout pass, which is once per screen
    /// refresh: the first time during a refresh that a parent asks this view
    /// for its layout key, whether it needs a relayout, its size, or lays it
    /// out. The result is kept until the next refresh, so the text that is
    /// drawn is the text that was measured, and parents see a consistent size
    /// for the whole pass. If the text is different from last time, the layout
    /// key changes, so containers that cache sizes (such as
    /// [`LinearLayout`](crate::views::LinearLayout)) measure it again. If it
    /// is equal, nothing is touched.
    ///
    /// The closure never runs while drawing, and not when the view is created:
    /// a live view is empty until its first layout.
    ///
    /// Refreshes happen after events and callbacks, and every frame when
    /// [`Cursive::set_fps`](crate::Cursive::set_fps) is set: set it if the
    /// state can change without any event. The text is then at most one frame
    /// behind.
    ///
    /// Calling the view's methods directly, outside of a refresh (for example
    /// in a test, or from a view that lays out its child by itself), is not
    /// part of a layout pass: every such call may run the closure again.
    ///
    /// The closure runs on the UI thread, in the middle of the layout:
    ///
    /// * It must be fast.
    /// * It must not call back into [`Cursive`](crate::Cursive).
    /// * Locking a mutex that the UI thread already holds will deadlock.
    ///
    /// # Changing the content by other means
    ///
    /// Setting the content from the view ([`set_content`](Self::set_content),
    /// [`content`](Self::content) or [`append`](Self::append)) stops using the
    /// closure: the view becomes a normal one again, and keeps the content it
    /// was given. See also [`clear_source`](Self::clear_source).
    ///
    /// A change made through a shared [`TextContent`] handle (from
    /// [`get_shared_content`](Self::get_shared_content), or another view
    /// sharing the content) can't know about the closure: it is overwritten
    /// at the next refresh. Reading through such a handle works, and sees
    /// what the closure produced.
    ///
    /// A live view can't be built from a config file (there is no blueprint
    /// for closures).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::align::HAlign;
    /// use cursive_core::views::TextView;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// let counter = Arc::new(AtomicUsize::new(0));
    ///
    /// let view = TextView::live({
    ///     let counter = Arc::clone(&counter);
    ///     move || format!("{} requests", counter.load(Ordering::Relaxed))
    /// })
    /// .h_align(HAlign::Right);
    ///
    /// // Later, from anywhere: the next refresh shows the new value.
    /// counter.fetch_add(1, Ordering::Relaxed);
    /// ```
    pub fn live<F, S>(source: F) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
    {
        Self::empty().source(source)
    }

    /// Sets the closure this view takes its content from.
    ///
    /// The view becomes live, from its next layout: see
    /// [`live`](Self::live) for when the closure runs, what it must not do,
    /// and how other ways of changing the content interact with it. Replaces
    /// any previous closure. The current content stays until the closure
    /// first runs.
    ///
    /// If this view shares its content (see
    /// [`new_with_content`](Self::new_with_content)), the closure writes into
    /// the shared content, so the other views sharing it follow.
    pub fn set_source<F, S>(&mut self, source: F)
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
    {
        self.source = Some(Source {
            f: Box::new(move || source().into()),
            evaluated_in: Mutex::new(None),
        });
    }

    /// Sets the closure this view takes its content from.
    ///
    /// Chainable variant of [`set_source`](Self::set_source).
    #[must_use]
    pub fn source<F, S>(mut self, source: F) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: Into<StyledString>,
    {
        self.set_source(source);
        self
    }

    /// Stops updating the content from a closure.
    ///
    /// The current content stays. Does nothing on a view that isn't live.
    pub fn clear_source(&mut self) {
        self.source = None;
    }

    /// Whether this view takes its content from a closure.
    pub fn is_live(&self) -> bool {
        self.source.is_some()
    }

    // Runs the closure unless it already ran during this layout pass, and
    // publishes the result if it changed (which changes the layout key).
    //
    // Does nothing for a view without a source.
    fn refresh(&self) {
        let Some(source) = &self.source else {
            return;
        };

        let pass = current_pass();
        {
            let mut evaluated_in = source.evaluated_in.lock();
            if pass.is_some() && *evaluated_in == pass {
                return;
            }
            *evaluated_in = pass;
        }

        let text = (source.f)();
        // Equal text is not a change: don't invalidate every cached layout.
        if *self.content.get_content() != text {
            self.content.set_content(text);
        }
    }

    /// Sets the effect for the entire content.
    #[deprecated(since = "0.16.0", note = "Use `set_style()` instead.")]
    pub fn set_effect(&mut self, effect: Effect) {
        self.set_style(effect);
    }

    /// Sets the style for the content.
    pub fn set_style<S: Into<StyleType>>(&mut self, style: S) {
        self.style = style.into();
    }

    /// Sets the effect for the entire content.
    ///
    /// Chainable variant.
    #[deprecated(since = "0.16.0", note = "Use `style()` instead.")]
    #[must_use]
    pub fn effect(self, effect: Effect) -> Self {
        self.style(effect)
    }

    /// Sets the style for the entire content.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn style<S: Into<StyleType>>(self, style: S) -> Self {
        self.with(|s| s.set_style(style))
    }

    /// Disables content wrap for this view.
    ///
    /// This may be useful if you want horizontal scrolling.
    #[must_use]
    pub fn no_wrap(self) -> Self {
        self.with(|s| s.set_content_wrap(false))
    }

    /// Controls content wrap for this view.
    ///
    /// If `true` (the default), text will wrap long lines when needed.
    pub fn set_content_wrap(&mut self, wrap: bool) {
        if wrap != self.wrap {
            self.wrap = wrap;
            self.rows_width = None;
            self.sizes.clear();
            self.laid_out_version = None;
        }
    }

    /// Sets the horizontal alignment for this view.
    #[must_use]
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Sets the vertical alignment for this view.
    #[must_use]
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the alignment for this view.
    #[must_use]
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;

        self
    }

    /// Center the text horizontally and vertically inside the view.
    #[must_use]
    pub fn center(mut self) -> Self {
        self.align = Align::center();
        self
    }

    /// Replace the text in this view.
    ///
    /// On a [live](Self::live) view, this stops using the closure: the given
    /// content stays.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn content<S>(self, content: S) -> Self
    where
        S: Into<StyledString>,
    {
        self.with(|s| s.set_content(content))
    }

    /// Replace the text in this view.
    ///
    /// On a [live](Self::live) view, this stops using the closure: the given
    /// content stays.
    pub fn set_content<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.source = None;
        self.content.set_content(content);
    }

    /// Append `content` to the end of a `TextView`.
    ///
    /// On a [live](Self::live) view, this stops using the closure: the
    /// resulting content stays.
    pub fn append<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.source = None;
        self.content.append(content);
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> TextContentRef {
        TextContentInner::get_content(&self.content.content)
    }

    /// Returns a shared reference to the content, allowing content mutation.
    pub fn get_shared_content(&mut self) -> TextContent {
        // We take &mut here without really needing it,
        // because it sort of "makes sense".
        TextContent {
            content: Arc::clone(&self.content.content),
        }
    }

    // This must be non-destructive, as it may be called
    // multiple times during layout.
    // The width text is wrapped at, for the given view size.
    fn wrap_width(&self, size: Vec2) -> usize {
        if self.wrap { size.x } else { usize::MAX }
    }

    // Catches up with the shared content, dropping anything computed for
    // an older version.
    fn update_snapshot(&mut self) {
        let (content, version, replaced_version) = self.content.snapshot();
        if version != self.snapshot_version {
            // If text was only appended since, most rows are still valid.
            let appended = self.snapshot_version != 0 && self.snapshot_version >= replaced_version;
            match self.rows_width {
                Some(width) if appended && width > 0 && !self.truncated => {
                    self.append_rows(&content, width);
                }
                _ => self.rows_width = None,
            }
            self.snapshot = content;
            self.snapshot_version = version;
            self.sizes.clear();
        }
    }

    // Updates `rows` (computed for `self.snapshot` at `width`) for `content`,
    // which is `self.snapshot` with some text appended.
    //
    // Rows of a line only depend on that line: only the last line of the
    // snapshot (which may continue in `content`) and what follows need to be
    // wrapped again.
    fn append_rows(&mut self, content: &InnerContentType, width: usize) {
        let (span, offset) = last_line_start(&self.snapshot);
        let old_rows = LinesIterator::new_at(self.snapshot.as_ref(), width, span, offset).count();
        self.rows.truncate(self.rows.len().saturating_sub(old_rows));

        let mut lines = LinesIterator::new_at(content.as_ref(), width, span, offset);
        self.rows.extend(lines.by_ref());
        self.truncated = lines.is_truncated();
        self.update_width(width);
    }

    // Sets `self.width` for `rows` computed at `width`.
    fn update_width(&mut self, width: usize) {
        self.width = if self.truncated || self.rows.iter().any(|row| row.is_wrapped) {
            // If any rows are wrapped (or some text didn't even fit), then
            // require the full width.
            Some(width)
        } else {
            self.rows.iter().map(|row| row.width).max()
        };
    }

    fn compute_rows(&mut self, size: Vec2) {
        let width = self.wrap_width(size);
        self.update_snapshot();

        if self.rows_fit(width) {
            return;
        }

        self.rows_width = Some(width);
        if width == 0 {
            // Nothing fits.
            self.rows.clear();
            self.width = None;
            self.truncated = false;
            return;
        }

        let mut lines = LinesIterator::new(self.snapshot.as_ref(), width);
        self.rows = lines.by_ref().collect();
        self.truncated = lines.is_truncated();
        self.update_width(width);

        const SIZES_CAP: usize = 8;
        if self.sizes.len() == SIZES_CAP {
            self.sizes.remove(0);
        }
        self.sizes.push((width, self.current_size()));
    }

    fn current_size(&self) -> Vec2 {
        Vec2::new(self.width.unwrap_or(0), self.rows.len())
    }

    // The size for `width`, if we can tell without wrapping the text again.
    fn known_size(&self, width: usize) -> Option<Vec2> {
        if self.rows_fit(width) {
            return Some(self.current_size());
        }
        self.sizes
            .iter()
            .find(|&&(w, size)| fits(w, size.x, width))
            .map(|&(_, size)| size)
    }

    // Are `rows` what `width` would give?
    fn rows_fit(&self, width: usize) -> bool {
        self.rows_width
            .is_some_and(|w| fits(w, self.width.unwrap_or(0), width))
    }
}

// Where the last line of `text` starts: (span, byte offset in that span).
//
// That's right after its last line break, skipping empty spans (as iterating
// over the whole text would).
fn last_line_start(text: &StyledString) -> (usize, usize) {
    let spans = text.spans_raw();
    let span_text = |i: usize| spans[i].content.resolve(text.source());

    let Some((mut span, mut offset)) = (0..spans.len())
        .rev()
        .find_map(|i| span_text(i).rfind('\n').map(|pos| (i, pos + 1)))
    else {
        return (0, 0);
    };
    while span < spans.len() && offset >= span_text(span).len() {
        span += 1;
        offset = 0;
    }
    (span, offset)
}

// Is what was computed at width `computed` (using `used` of it) also what
// `width` would give?
//
// For the same width, or when it didn't use all the width it had (so nothing
// was wrapped) and `width` still holds it. Not when it used exactly all of
// it: a trailing space may have been dropped without the row counting as
// wrapped. Never for width 0 either, where nothing is shown at all.
fn fits(computed: usize, used: usize, width: usize) -> bool {
    computed == width || (width > 0 && used < computed && used <= width)
}

impl View for TextView {
    fn draw(&self, printer: &Printer) {
        let h = self.rows.len();
        // If the content is smaller than the view, align it somewhere.
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset));

        printer.with_style(self.style, |printer| {
            for (y, row) in self
                .rows
                .iter()
                .enumerate()
                .skip(printer.content_offset.y)
                .take(printer.output_size.y)
            {
                let l = row.width;
                let mut x = self.align.h.get_offset(l, printer.size.x);

                for span in row.resolve_stream(self.snapshot.as_ref()) {
                    printer.with_style(*span.attr, |printer| {
                        printer.print((x, y), span.content);
                        x += span.content.width();
                    });
                }
            }
        });
    }

    fn needs_relayout(&self) -> bool {
        self.refresh();
        self.laid_out_version != Some(self.content.version())
    }

    fn layout_key(&self) -> u64 {
        self.refresh();
        combine_layout_key(self.content.version(), &self.wrap)
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        self.refresh();
        let width = self.wrap_width(size);
        if width == 0 {
            // No room at all: we'd need at least one column, and at most one
            // row per byte (each row at width 1 holds at least one).
            // (Without updating the snapshot: `rows` must stay in sync with
            // it, and there are none to compute here.)
            let len = self.content.len();
            return Vec2::new(usize::from(len > 0), len);
        }
        self.update_snapshot();
        if let Some(size) = self.known_size(width) {
            return size;
        }

        self.compute_rows(size);
        self.current_size()
    }

    fn layout(&mut self, size: Vec2) {
        self.refresh();
        // Compute the text rows.
        self.compute_rows(size);
        self.laid_out_version = Some(self.snapshot_version);
    }
}

// Need: a name, a base (potential dependencies), setters
#[crate::blueprint(TextView::empty())]
enum Blueprint {
    // We accept `TextView` without even a body
    Empty,

    // Inline content
    Content(StyledString),

    // Full object with optional content field
    // This is also used to add a `with` block
    Object { content: Option<StyledString> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Backend;
    use crate::buffer::PrintBuffer;
    use crate::event::{Event, Key};
    use crate::style::Style;
    use crate::theme::Theme;
    use crate::view::layout_pass::in_layout_pass;
    use crate::views::{Dialog, LinearLayout, ResizedView, ScrollView};
    use crate::{Cursive, CursiveRunner};
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A closure returning `text`, which can be changed, and a call counter.
    struct Probe {
        text: Arc<Mutex<StyledString>>,
        calls: Arc<AtomicUsize>,
    }

    impl Probe {
        fn new(text: &str) -> Self {
            Probe {
                text: Arc::new(Mutex::new(StyledString::plain(text))),
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn set(&self, text: impl Into<StyledString>) {
            *self.text.lock() = text.into();
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        fn view(&self) -> TextView {
            let text = Arc::clone(&self.text);
            let calls = Arc::clone(&self.calls);
            TextView::live(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                text.lock().clone()
            })
        }
    }

    fn buffer(size: Vec2) -> RwLock<PrintBuffer> {
        let mut buffer = PrintBuffer::new();
        buffer.resize(size);
        RwLock::new(buffer)
    }

    /// Draws the view and returns the buffer.
    fn draw(view: &dyn View, size: Vec2) -> RwLock<PrintBuffer> {
        let buffer = buffer(size);
        view.draw(&Printer::new(size, &Theme::retro(), &buffer));
        buffer
    }

    /// The lines of text, with `.` for the cells that were not printed.
    fn lines(buffer: &RwLock<PrintBuffer>) -> Vec<String> {
        let buffer = buffer.read();
        let size = buffer.size();
        (0..size.y)
            .map(|y| {
                (0..size.x)
                    .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                        None | Some("") | Some(" ") => ".",
                        Some(text) => text,
                    })
                    .collect()
            })
            .collect()
    }

    /// One frame as `Cursive` does it: a pass around the layout, then draw.
    fn frame(view: &mut dyn View, size: Vec2) -> Vec<String> {
        in_layout_pass(|| {
            let wanted = view.required_size(size);
            view.layout(Vec2::min(wanted, size));
        });
        lines(&draw(view, size))
    }

    #[test]
    fn live_follows_the_closure_between_frames() {
        let probe = Probe::new("one");
        let mut view = probe.view();
        let size = Vec2::new(5, 1);

        assert_eq!(frame(&mut view, size), ["one.."]);
        probe.set("three");
        assert_eq!(frame(&mut view, size), ["three"]);
        probe.set("2");
        assert_eq!(frame(&mut view, size), ["2...."]);
    }

    #[test]
    fn live_size_follows_the_content() {
        let probe = Probe::new("ab");
        let mut view = probe.view();
        let max = Vec2::new(10, 10);

        let size = in_layout_pass(|| view.required_size(max));
        assert_eq!(size, Vec2::new(2, 1));

        probe.set("abcdef");
        let size = in_layout_pass(|| view.required_size(max));
        assert_eq!(size, Vec2::new(6, 1));

        probe.set("ab\ncd\nefg");
        let size = in_layout_pass(|| view.required_size(max));
        assert_eq!(size, Vec2::new(3, 3));

        // It wraps like a TextView.
        probe.set("aaaa bbbb");
        let size = in_layout_pass(|| view.required_size(Vec2::new(5, 10)));
        assert_eq!(size, Vec2::new(5, 2));
    }

    #[test]
    fn live_styled_content_keeps_its_styles() {
        let probe = Probe::new("");
        let mut styled = StyledString::plain("a");
        styled.append_styled("b", Effect::Bold);
        probe.set(styled);
        let mut view = probe.view();

        let size = Vec2::new(2, 1);
        frame(&mut view, size);
        let buffer = draw(&view, size);
        let buffer = buffer.read();
        let plain = buffer.cell_style(Vec2::new(0, 0)).unwrap();
        let bold = buffer.cell_style(Vec2::new(1, 0)).unwrap();
        assert!(!plain.effects.contains(Effect::Bold));
        assert!(bold.effects.contains(Effect::Bold));
    }

    #[test]
    fn live_view_style_applies() {
        let probe = Probe::new("a");
        let mut view = probe.view().style(Style::from(Effect::Bold));
        let size = Vec2::new(1, 1);
        frame(&mut view, size);
        let buffer = draw(&view, size);
        let style = buffer.read().cell_style(Vec2::new(0, 0)).unwrap();
        assert!(style.effects.contains(Effect::Bold));
    }

    #[test]
    fn live_alignment() {
        let probe = Probe::new("ab");
        let mut view = probe.view().h_align(HAlign::Right);
        assert_eq!(frame(&mut view, Vec2::new(5, 1)), ["...ab"]);

        let mut view = probe.view().center();
        assert_eq!(frame(&mut view, Vec2::new(4, 3)), ["....", ".ab.", "...."]);

        let mut view = probe.view().align(Align::bot_right());
        assert_eq!(frame(&mut view, Vec2::new(3, 2)), ["...", ".ab"]);

        let mut view = probe.view().v_align(VAlign::Bottom);
        assert_eq!(frame(&mut view, Vec2::new(2, 2)), ["..", "ab"]);
    }

    #[test]
    fn live_wrapping_and_no_wrap() {
        let probe = Probe::new("aaa bbb");

        let mut view = probe.view();
        assert_eq!(frame(&mut view, Vec2::new(3, 2)), ["aaa", "bbb"]);

        let mut view = probe.view().no_wrap();
        assert_eq!(
            in_layout_pass(|| view.required_size(Vec2::new(3, 2))),
            Vec2::new(7, 1)
        );
        assert_eq!(frame(&mut view, Vec2::new(3, 2)), ["aaa", "..."]);

        let mut view = probe.view();
        view.set_content_wrap(false);
        assert_eq!(
            in_layout_pass(|| view.required_size(Vec2::new(3, 2))),
            Vec2::new(7, 1)
        );
    }

    #[test]
    fn live_one_call_per_layout_pass() {
        let probe = Probe::new("hello");
        let mut view = probe.view();
        let size = Vec2::new(10, 2);

        // Creating the view and drawing it don't run the closure.
        assert_eq!(probe.calls(), 0);

        // One pass, however many probes: a single call.
        in_layout_pass(|| {
            view.needs_relayout();
            view.layout_key();
            view.required_size(size);
            view.required_size(Vec2::new(3, 2));
            view.required_size(size);
            view.layout(Vec2::new(5, 1));
            view.layout_key();
        });
        assert_eq!(probe.calls(), 1);

        draw(&view, size);
        draw(&view, size);
        assert_eq!(probe.calls(), 1);

        // Nested passes are the same pass.
        in_layout_pass(|| {
            in_layout_pass(|| view.required_size(size));
            view.layout(size);
        });
        assert_eq!(probe.calls(), 2);

        // A whole frame is one call too.
        frame(&mut view, size);
        assert_eq!(probe.calls(), 3);
    }

    #[test]
    fn live_draw_shows_what_was_laid_out() {
        let probe = Probe::new("old");
        let mut view = probe.view();
        let size = Vec2::new(3, 1);
        frame(&mut view, size);

        // Changed after the layout: not seen until the next one.
        probe.set("new");
        assert_eq!(lines(&draw(&view, size)), ["old"]);
        assert_eq!(probe.calls(), 1);
        assert_eq!(frame(&mut view, size), ["new"]);
    }

    #[test]
    fn live_same_text_keeps_the_layout_key() {
        let probe = Probe::new("same");
        let view = probe.view();
        let first = in_layout_pass(|| view.layout_key());
        let second = in_layout_pass(|| view.layout_key());
        assert_eq!(first, second);

        probe.set("different");
        let third = in_layout_pass(|| view.layout_key());
        assert_ne!(first, third);
    }

    #[test]
    fn live_inside_a_linear_layout() {
        let probe = Probe::new("a");
        let mut layout =
            LinearLayout::vertical()
                .child(probe.view())
                .child(ResizedView::with_fixed_size(
                    (4, 1),
                    crate::views::DummyView,
                ));
        let size = Vec2::new(4, 6);

        let frame = |layout: &mut LinearLayout| {
            in_layout_pass(|| {
                let wanted = layout.required_size(size);
                layout.layout(wanted);
                wanted
            })
        };

        let wanted = frame(&mut layout);
        assert_eq!(wanted, Vec2::new(4, 2));
        assert_eq!(
            lines(&draw(&layout, wanted))[..1],
            ["a..."].map(String::from)
        );

        // More lines, then still more, then shorter again: the layout and
        // its cached sizes follow each time.
        for (text, rows, first) in [
            ("a\nb\nc", 4, "a..."),
            ("a\nb\nc\nd\ne", 6, "a..."),
            ("zzzz", 2, "zzzz"),
            ("z", 2, "z..."),
        ] {
            probe.set(text);
            let wanted = frame(&mut layout);
            assert_eq!(wanted, Vec2::new(4, rows), "{text:?}");
            let drawn = lines(&draw(&layout, wanted));
            assert_eq!(drawn[0], first, "{text:?}");
            assert_eq!(drawn.len(), rows);
            assert_eq!(drawn.iter().filter(|l| *l != "....").count(), rows - 1);
        }

        // One closure call per frame, through all the layout's probing.
        assert_eq!(probe.calls(), 5);
    }

    #[test]
    fn live_inside_a_resized_view_and_horizontal_layout() {
        let probe = Probe::new("ab");
        let mut layout = LinearLayout::horizontal()
            .child(ResizedView::with_min_width(3, probe.view()))
            .child(TextView::new("|"));
        let size = Vec2::new(20, 3);

        for (text, width) in [("ab", 4), ("abcdef", 7), ("abcdefghij", 11), ("x", 4)] {
            probe.set(text);
            let wanted = in_layout_pass(|| {
                let wanted = layout.required_size(size);
                layout.layout(wanted);
                wanted
            });
            assert_eq!(wanted.x, width, "{text:?}");
            let drawn = lines(&draw(&layout, wanted));
            let expected: String = format!("{:.<w$}|", text, w = width - 1);
            assert_eq!(drawn[0], expected, "{text:?}");
        }
    }

    // A backend recording what is printed, to run a real `Cursive`.
    struct Recorder {
        size: Vec2,
        printed: Arc<Mutex<String>>,
    }

    impl Backend for Recorder {
        fn name(&self) -> &str {
            "Recorder"
        }
        fn screen_size(&self) -> Vec2 {
            self.size
        }
        fn poll_event(&mut self) -> Option<crate::event::Event> {
            None
        }
        fn set_title(&mut self, _: String) {}
        fn refresh(&mut self) {}
        fn has_colors(&self) -> bool {
            false
        }
        fn move_to(&self, _: Vec2) {}
        fn print(&self, text: &str) {
            self.printed.lock().push_str(text);
        }
        fn clear(&self, _: crate::style::Color) {}
        fn set_color(&self, colors: crate::style::ColorPair) -> crate::style::ColorPair {
            colors
        }
        fn set_effect(&self, _: crate::style::Effect) {}
        fn unset_effect(&self, _: crate::style::Effect) {}
    }

    #[test]
    fn live_end_to_end_with_cursive() {
        use crate::view::Resizable;

        let probe = Probe::new("first");
        let mut siv = Cursive::new();
        siv.add_layer(
            LinearLayout::vertical()
                .child(probe.view())
                .child(TextView::new("footer"))
                .fixed_width(10),
        );
        let printed = Arc::new(Mutex::new(String::new()));
        let mut runner = CursiveRunner::new(
            siv,
            Box::new(Recorder {
                size: Vec2::new(30, 12),
                printed: Arc::clone(&printed),
            }),
        );

        runner.refresh();
        assert_eq!(probe.calls(), 1);
        assert!(printed.lock().contains("first"));
        let first_height = runner.screen().layer_sizes()[0].y;

        // One closure call per refresh, and the new text is shown at once,
        // with a taller layer for the extra lines.
        printed.lock().clear();
        probe.set("second\nline two\nline three");
        runner.refresh();
        assert_eq!(probe.calls(), 2);
        let printed = printed.lock();
        assert!(printed.contains("second"), "{printed:?}");
        assert!(printed.contains("line three"), "{printed:?}");
        assert!(runner.screen().layer_sizes()[0].y > first_height);
    }

    #[test]
    fn set_content_clears_the_source() {
        let probe = Probe::new("live");
        let mut view = probe.view();
        let size = Vec2::new(8, 1);
        assert!(view.is_live());
        assert_eq!(frame(&mut view, size), ["live...."]);
        assert_eq!(probe.calls(), 1);

        view.set_content("manual");
        assert!(!view.is_live());
        probe.set("other");
        for _ in 0..3 {
            assert_eq!(frame(&mut view, size), ["manual.."]);
        }
        assert_eq!(probe.calls(), 1);

        // The chainable variant too.
        let view = probe.view().content("fixed");
        assert!(!view.is_live());
    }

    #[test]
    fn append_clears_the_source() {
        let probe = Probe::new("ab");
        let mut view = probe.view();
        let size = Vec2::new(8, 1);
        frame(&mut view, size);

        view.append("cd");
        assert!(!view.is_live());
        probe.set("zzz");
        for _ in 0..3 {
            assert_eq!(frame(&mut view, size), ["abcd...."]);
        }
        assert_eq!(probe.calls(), 1);
    }

    #[test]
    fn clear_source_keeps_the_last_content() {
        let probe = Probe::new("last");
        let mut view = probe.view();
        let size = Vec2::new(6, 1);
        frame(&mut view, size);
        probe.set("newer");

        view.clear_source();
        assert!(!view.is_live());
        for _ in 0..3 {
            assert_eq!(frame(&mut view, size), ["last.."]);
        }
        assert_eq!(probe.calls(), 1);

        // Nothing to clear on a plain view.
        view.clear_source();
        assert_eq!(frame(&mut view, size), ["last.."]);
    }

    #[test]
    fn set_source_makes_a_plain_view_live() {
        let probe = Probe::new("live");
        let mut view = TextView::new("plain");
        let size = Vec2::new(6, 1);
        assert!(!view.is_live());
        assert_eq!(frame(&mut view, size), ["plain."]);

        let text = Arc::clone(&probe.text);
        let calls = Arc::clone(&probe.calls);
        view.set_source(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            text.lock().clone()
        });
        assert!(view.is_live());
        // Not evaluated until the next pass.
        assert_eq!(probe.calls(), 0);
        assert_eq!(frame(&mut view, size), ["live.."]);
        assert_eq!(probe.calls(), 1);
    }

    #[test]
    fn replacing_the_source_switches_closures() {
        let first = Probe::new("first");
        let second = Probe::new("second");
        let mut view = first.view();
        let size = Vec2::new(6, 1);
        assert_eq!(frame(&mut view, size), ["first."]);

        let text = Arc::clone(&second.text);
        let calls = Arc::clone(&second.calls);
        view.set_source(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            text.lock().clone()
        });
        assert_eq!(frame(&mut view, size), ["second"]);
        first.set("changed");
        assert_eq!(frame(&mut view, size), ["second"]);
        assert_eq!(first.calls(), 1);
        assert_eq!(second.calls(), 2);

        // The chainable variant.
        let mut view = TextView::new("x").source(|| "y");
        assert_eq!(frame(&mut view, Vec2::new(1, 1)), ["y"]);
    }

    #[test]
    fn shared_content_change_is_overwritten() {
        let probe = Probe::new("closure");
        let mut view = probe.view();
        let handle = view.get_shared_content();
        let size = Vec2::new(7, 1);
        assert_eq!(frame(&mut view, size), ["closure"]);
        // The handle sees what the closure produced.
        assert_eq!(handle.get_content().source(), "closure");

        handle.set_content("handle");
        assert_eq!(frame(&mut view, size), ["closure"]);
        assert_eq!(handle.get_content().source(), "closure");
        assert!(view.is_live());
    }

    #[test]
    fn view_sharing_content_follows_the_live_one() {
        let probe = Probe::new("one");
        let content = TextContent::new("");
        let mut live = TextView::new_with_content(content.clone());
        let mut follower = TextView::new_with_content(content);
        let text = Arc::clone(&probe.text);
        live.set_source(move || text.lock().clone());
        let size = Vec2::new(5, 1);

        assert_eq!(frame(&mut follower, size), ["....."]);
        frame(&mut live, size);
        assert_eq!(frame(&mut follower, size), ["one.."]);
        probe.set("two");
        frame(&mut live, size);
        assert_eq!(frame(&mut follower, size), ["two.."]);
    }

    #[test]
    fn plain_view_never_runs_a_closure_or_changes_key() {
        let view = TextView::new("same");
        assert!(!view.is_live());
        let key = view.layout_key();
        assert_eq!(in_layout_pass(|| view.layout_key()), key);
        assert!(view.needs_relayout());
    }

    #[test]
    fn appended_rows_match_fresh_rows() {
        // Appending only wraps the last line again: after any sequence of
        // appends (and replacements), rows must be what wrapping everything
        // gives.
        use crate::style::Effect;
        use crate::utils::markup::StyledString;
        use crate::views::TextContent;

        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let pieces = [
            "a",
            "word ",
            " leading",
            "\n",
            "\n\n",
            "e\u{301}",
            "中文",
            "trailing  ",
            "",
            "longer text that wraps\n",
            "\u{200b}",
            "tab\tthere",
            "x",
        ];
        let random_text = |rnd: &mut dyn FnMut(usize) -> usize| {
            let mut text = StyledString::new();
            for _ in 0..rnd(4) {
                let piece = pieces[rnd(pieces.len())];
                if rnd(3) == 0 {
                    text.append_styled(piece, Effect::Bold);
                } else {
                    text.append_plain(piece);
                }
            }
            text
        };

        for _ in 0..300 {
            let content = TextContent::new(random_text(&mut rnd));
            let mut view = TextView::new_with_content(content.clone());
            for _ in 0..30 {
                match rnd(8) {
                    0 => content.set_content(random_text(&mut rnd)),
                    1 | 2 => {}
                    _ => content.append(random_text(&mut rnd)),
                }
                let size = Vec2::new(rnd(30), 5);
                if rnd(2) == 0 {
                    view.layout(size);
                }
                let mut fresh = TextView::new(content.get_content().clone());
                assert_eq!(view.required_size(size), fresh.required_size(size));
                view.layout(size);
                fresh.layout(size);
                assert_eq!(
                    view.rows,
                    fresh.rows,
                    "{:?}",
                    content.get_content().source()
                );
            }
        }
    }

    #[test]
    fn size_cache_stays_bounded() {
        let mut view = TextView::new("some text that wraps at many widths");
        for i in 0..5000 {
            view.required_size(Vec2::new(i % 300, 5));
            if i % 7 == 0 {
                view.layout(Vec2::new(i % 300, 5));
            }
            assert!(view.sizes.len() <= 8);
        }
    }

    #[test]
    fn zero_width_keeps_rows_in_sync() {
        // Rows must match the snapshot they were computed from (`draw` uses
        // both): asking for width 0 after a change must not replace one
        // without the other.
        let mut view = TextView::new("first");
        view.layout(Vec2::new(10, 5));
        let rows_version = view.snapshot_version;

        view.set_content("second, longer text");
        assert_eq!(view.required_size(Vec2::new(0, 5)).x, 1);
        assert_eq!(view.snapshot_version, rows_version);
    }

    #[test]
    fn zero_width_needs_a_column() {
        // Not "nothing": parents measuring their minimum size through
        // decorations reach width 0, and must not think the text is free.
        let size = TextView::new("abc def").required_size(Vec2::new(0, 5));
        assert_eq!(size.x, 1);
        assert!(size.y >= TextView::new("abc def").required_size(Vec2::new(1, 5)).y);
        assert_eq!(
            TextView::new("").required_size(Vec2::new(0, 5)),
            Vec2::zero()
        );
    }

    #[test]
    fn reused_rows_match_fresh_rows() {
        // A view measured at one width, then another, must answer like a
        // view measured at the second width directly: reusing rows across
        // widths must not change anything. Covers trailing spaces (dropped
        // when they don't fit, without counting as wrapped) and characters
        // wider than the whole line (left out).
        let words = [
            "a",
            "ab ",
            " ",
            "few words",
            "中",
            "中文字",
            "a中",
            "🦀",
            "e\u{301}",
            "\u{200b}",
            "日本 語",
            "x\n中",
            "",
        ];
        for a in words {
            for b in words {
                for c in words {
                    let text = format!("{a} {b}{c}");
                    for w0 in 0..14 {
                        let mut view = TextView::new(text.clone());
                        view.required_size(Vec2::new(w0, 10));
                        for w in 0..14 {
                            let fresh = TextView::new(text.clone()).required_size(Vec2::new(w, 10));
                            let reused = view.required_size(Vec2::new(w, 10));
                            assert_eq!(reused, fresh, "{text:?}: width {w0} then {w}");
                            // Go back, so the next width is also reached from `w0`.
                            view.required_size(Vec2::new(w0, 10));
                        }
                    }
                }
            }
        }
    }

    fn live_counting(calls: &Arc<AtomicUsize>, text: &Arc<Mutex<String>>) -> TextView {
        let calls = Arc::clone(calls);
        let text = Arc::clone(text);
        TextView::live(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            text.lock().clone()
        })
    }

    #[test]
    fn live_one_call_per_refresh_in_a_dialog_with_a_scroll_view() {
        let calls = Arc::new(AtomicUsize::new(0));
        let text = Arc::new(Mutex::new(String::from(
            "one two three four five six seven",
        )));
        let mut siv = Cursive::new();
        siv.add_layer(
            Dialog::around(ScrollView::new(live_counting(&calls, &text)))
                .title("t")
                .button("Ok", |_| ()),
        );
        let printed = Arc::new(Mutex::new(String::new()));
        let mut runner = CursiveRunner::new(
            siv,
            Box::new(Recorder {
                size: Vec2::new(14, 6),
                printed: Arc::clone(&printed),
            }),
        );
        let mut per_refresh = Vec::new();
        for t in [
            "one two three four five six seven",
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj",
            "short",
            "short",
        ] {
            *text.lock() = t.to_string();
            let before = calls.load(Ordering::SeqCst);
            runner.refresh();
            per_refresh.push(calls.load(Ordering::SeqCst) - before);
        }
        // An event, then a refresh.
        let before = calls.load(Ordering::SeqCst);
        runner.on_event(Event::Key(Key::Down));
        let during_event = calls.load(Ordering::SeqCst) - before;
        runner.refresh();
        let after = calls.load(Ordering::SeqCst) - before;
        assert!(
            per_refresh.iter().all(|&n| n == 1),
            "calls per refresh {per_refresh:?}, during event {during_event}, event+refresh {after}"
        );
    }

    #[test]
    fn set_content_sticks_across_passes_on_a_shared_view() {
        let content = TextContent::new("shared");
        let mut a = TextView::new_with_content(content.clone());
        let b = TextView::new_with_content(content.clone());
        a.set_source(|| "live");
        in_layout_pass(|| {
            a.required_size(Vec2::new(10, 1));
            a.layout(Vec2::new(10, 1));
        });
        assert_eq!(b.get_content().source(), "live");
        a.set_content("manual");
        for _ in 0..3 {
            in_layout_pass(|| {
                a.required_size(Vec2::new(10, 1));
                a.layout(Vec2::new(10, 1));
            });
            assert_eq!(a.get_content().source(), "manual");
            assert_eq!(b.get_content().source(), "manual");
        }
    }

    #[test]
    fn live_in_a_horizontal_layout_is_called_once_per_pass() {
        // Closure count with a layout that probes children several times.
        let calls = Arc::new(AtomicUsize::new(0));
        let text = Arc::new(Mutex::new(String::from("aaa bbb ccc ddd eee")));
        let mut layout = LinearLayout::horizontal()
            .child(live_counting(&calls, &text))
            .child(live_counting(&calls, &text))
            .child(TextView::new("x"));
        for w in [4, 10, 20, 40] {
            let before = calls.load(Ordering::SeqCst);
            in_layout_pass(|| {
                let s = layout.required_size(Vec2::new(w, 10));
                layout.layout(s);
            });
            assert_eq!(calls.load(Ordering::SeqCst) - before, 2, "width {w}");
        }
    }
}
