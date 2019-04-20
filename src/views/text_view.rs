use std::ops::Deref;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use owning_ref::{ArcRef, OwningHandle};
use unicode_width::UnicodeWidthStr;

use crate::align::*;
use crate::theme::Effect;
use crate::utils::lines::spans::{LinesIterator, Row};
use crate::utils::markup::StyledString;
use crate::view::{SizeCache, View};
use crate::{Printer, Vec2, With, XY};

/// Provides access to the content of a [`TextView`].
///
/// Cloning this object will still point to the same content.
///
/// [`TextView`]: struct.TextView.html
///
/// # Examples
///
/// ```rust
/// # use cursive::views::{TextView, TextContent};
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
        let content = content.into();

        TextContent {
            content: Arc::new(Mutex::new(TextContentInner {
                content,
                size_cache: None,
            })),
        }
    }
}

/// A reference to the text content.
///
/// This can be deref'ed into a [`StyledString`].
///
/// [`StyledString`]: ../utils/markup/type.StyledString.html
///
/// This keeps the content locked. Do not store this!
pub struct TextContentRef {
    handle: OwningHandle<
        ArcRef<Mutex<TextContentInner>>,
        MutexGuard<'static, TextContentInner>,
    >,
}

impl Deref for TextContentRef {
    type Target = StyledString;

    fn deref(&self) -> &StyledString {
        &self.handle.content
    }
}

impl TextContent {
    /// Replaces the content with the given value.
    pub fn set_content<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.with_content(|c| *c = content.into());
    }

    /// Append `content` to the end of a `TextView`.
    pub fn append<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.with_content(|c| c.append(content))
    }

    /// Returns a reference to the content.
    ///
    /// This locks the data while the returned value is alive,
    /// so don't keep it too long.
    pub fn get_content(&self) -> TextContentRef {
        TextContentInner::get_content(&self.content)
    }

    fn with_content<F, O>(&mut self, f: F) -> O
    where
        F: FnOnce(&mut StyledString) -> O,
    {
        let mut lock = self.content.lock().unwrap();

        let out = f(&mut lock.content);

        lock.size_cache = None;

        out
    }
}

/// Internel representation of the content for a `TextView`.
///
/// This is mostly just a `StyledString`.
///
/// Can be shared (through a `Arc<Mutex>`).
struct TextContentInner {
    // content: String,
    content: StyledString,

    // We keep the cache here so it can be busted when we change the content.
    size_cache: Option<XY<SizeCache>>,
}

impl TextContentInner {
    /// From a shareable content (Arc + Mutex), return a
    fn get_content(content: &Arc<Mutex<TextContentInner>>) -> TextContentRef {
        let arc_ref: ArcRef<Mutex<TextContentInner>> =
            ArcRef::new(Arc::clone(content));

        TextContentRef {
            handle: OwningHandle::new_with_fn(arc_ref, |mutex| unsafe {
                (*mutex).lock().unwrap()
            }),
        }
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.size_cache {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }
}

/// A simple view showing a fixed text.
///
/// # Examples
///
/// ```rust
/// # use cursive::Cursive;
/// # use cursive::views::TextView;
/// let mut siv = Cursive::dummy();
///
/// siv.add_layer(TextView::new("Hello world!"));
/// ```
pub struct TextView {
    // content: String,
    content: Arc<Mutex<TextContentInner>>,
    rows: Vec<Row>,

    align: Align,
    effect: Effect,

    // True if we can wrap long lines.
    wrap: bool,

    // ScrollBase make many scrolling-related things easier
    last_size: Vec2,
    width: Option<usize>,
}

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new<S>(content: S) -> Self
    where
        S: Into<StyledString>,
    {
        Self::new_with_content(TextContent::new(content))
    }

    /// Creates a new TextView using the given `Arc<Mutex<String>>`.
    ///
    /// If you kept a clone of the given content, you'll be able to update it
    /// remotely.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::views::{TextView, TextContent};
    /// let mut content = TextContent::new("content");
    /// let view = TextView::new_with_content(content.clone());
    ///
    /// // Later, possibly in a different thread
    /// content.set_content("new content");
    /// assert!(view.get_content().source().contains("new"));
    /// ```
    pub fn new_with_content(content: TextContent) -> Self {
        TextView {
            content: content.content,
            effect: Effect::Simple,
            rows: Vec::new(),
            wrap: true,
            align: Align::top_left(),
            last_size: Vec2::zero(),
            width: None,
        }
    }

    /// Creates a new empty `TextView`.
    pub fn empty() -> Self {
        TextView::new("")
    }

    /// Sets the effect for the entire content.
    pub fn set_effect(&mut self, effect: Effect) {
        self.effect = effect;
    }

    /// Sets the effect for the entire content.
    ///
    /// Chainable variant.
    pub fn effect(self, effect: Effect) -> Self {
        self.with(|s| s.set_effect(effect))
    }

    /// Disables content wrap for this view.
    ///
    /// This may be useful if you want horizontal scrolling.
    pub fn no_wrap(self) -> Self {
        self.with(|s| s.set_content_wrap(false))
    }

    /// Controls content wrap for this view.
    ///
    /// If `true` (the default), text will wrap long lines when needed.
    pub fn set_content_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }

    /// Sets the horizontal alignment for this view.
    pub fn h_align(mut self, h: HAlign) -> Self {
        self.align.h = h;

        self
    }

    /// Sets the vertical alignment for this view.
    pub fn v_align(mut self, v: VAlign) -> Self {
        self.align.v = v;

        self
    }

    /// Sets the alignment for this view.
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;

        self
    }

    /// Center the text horizontally and vertically inside the view.
    pub fn center(mut self) -> Self {
        self.align = Align::center();
        self
    }

    /// Replace the text in this view.
    ///
    /// Chainable variant.
    pub fn content<S>(self, content: S) -> Self
    where
        S: Into<StyledString>,
    {
        self.with(|s| s.set_content(content))
    }

    /// Replace the text in this view.
    pub fn set_content<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.content.lock().unwrap().content = content.into();
        self.invalidate();
    }

    /// Append `content` to the end of a `TextView`.
    pub fn append<S>(&mut self, content: S)
    where
        S: Into<StyledString>,
    {
        self.content.lock().unwrap().content.append(content.into());
        self.invalidate();
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> TextContentRef {
        TextContentInner::get_content(&self.content)
    }

    /// Returns a shared reference to the content, allowing content mutation.
    pub fn get_shared_content(&mut self) -> TextContent {
        // We take &mut here without really needing it,
        // because it sort of "makes sense".

        TextContent {
            content: Arc::clone(&self.content),
        }
    }

    // This must be non-destructive, as it may be called
    // multiple times during layout.
    fn compute_rows(&mut self, size: Vec2) {
        let size = if self.wrap { size } else { Vec2::max_value() };

        let mut content = self.content.lock().unwrap();
        if content.is_cache_valid(size) {
            return;
        }

        // Completely bust the cache
        // Just in case we fail, we don't want to leave a bad cache.
        content.size_cache = None;

        if size.x == 0 {
            // Nothing we can do at this point.
            return;
        }

        self.rows = LinesIterator::new(&content.content, size.x).collect();

        // Desired width
        self.width = self.rows.iter().map(|row| row.width).max();
    }

    // Invalidates the cache, so next call will recompute everything.
    fn invalidate(&mut self) {
        let mut content = self.content.lock().unwrap();
        content.size_cache = None;
    }
}

impl View for TextView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let h = self.rows.len();
        // If the content is smaller than the view, align it somewhere.
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset));

        let content = self.content.lock().unwrap();

        printer.with_effect(self.effect, |printer| {
            for (y, row) in self.rows.iter().enumerate() {
                let l = row.width;
                let mut x = self.align.h.get_offset(l, printer.size.x);

                for span in row.resolve(&content.content) {
                    printer.with_style(*span.attr, |printer| {
                        printer.print((x, y), span.content);
                        x += span.content.width();
                    });
                }
            }
        });
    }

    fn needs_relayout(&self) -> bool {
        let content = self.content.lock().unwrap();
        content.size_cache.is_none()
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        self.compute_rows(size);

        Vec2::new(self.width.unwrap_or(0), self.rows.len())
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.last_size = size;
        self.compute_rows(size);

        // The entire "virtual" size (includes all rows)
        let my_size = Vec2::new(self.width.unwrap_or(0), self.rows.len());

        // Build a fresh cache.
        let mut content = self.content.lock().unwrap();
        content.size_cache = Some(SizeCache::build(my_size, size));
    }
}
