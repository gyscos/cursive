use Printer;
use With;
use XY;
use align::*;
use direction::Direction;
use event::*;
use unicode_width::UnicodeWidthStr;
use utils::{LinesIterator, Row};
use vec::Vec2;
use view::{ScrollBase, ScrollStrategy, SizeCache, View};

/// A simple view showing a fixed text
pub struct TextView {
    content: String,
    rows: Vec<Row>,

    align: Align,

    // If `false`, disable scrolling.
    scrollable: bool,

    // ScrollBase make many scrolling-related things easier
    scrollbase: ScrollBase,
    scroll_strategy: ScrollStrategy,
    size_cache: Option<XY<SizeCache>>,
    last_size: Vec2,
    width: Option<usize>,
}

// If the last character is a newline, strip it.
fn strip_last_newline(content: &str) -> &str {
    if content.ends_with('\n') {
        &content[..content.len() - 1]
    } else {
        content
    }
}

impl TextView {
    /// Creates a new TextView with the given content.
    pub fn new<S: Into<String>>(content: S) -> Self {
        let content = content.into();
        TextView {
            content: content,
            rows: Vec::new(),
            scrollable: true,
            scrollbase: ScrollBase::new(),
            scroll_strategy: ScrollStrategy::KeepRow,
            align: Align::top_left(),
            size_cache: None,
            last_size: Vec2::zero(),
            width: None,
        }
    }

    /// Creates a new empty `TextView`.
    pub fn empty() -> Self {
        TextView::new("")
    }

    /// Enable or disable the view's scrolling capabilities.
    ///
    /// When disabled, the view will never attempt to scroll
    /// (and will always ask for the full height).
    pub fn set_scrollable(&mut self, scrollable: bool) {
        self.scrollable = scrollable;
    }

    /// Enable or disable the view's scrolling capabilities.
    ///
    /// When disabled, the view will never attempt to scroll
    /// (and will always ask for the full height).
    ///
    /// Chainable variant.
    pub fn scrollable(self, scrollable: bool) -> Self {
        self.with(|s| s.set_scrollable(scrollable))
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
    pub fn content<S: Into<String>>(self, content: S) -> Self {
        self.with(|s| s.set_content(content))
    }

    /// Replace the text in this view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        let content = content.into();
        self.content = content;
        self.invalidate();
    }

    /// Append content to the end of a TextView.
    pub fn append_content(&mut self, content: &str) {
        self.content.push_str(content);
        self.invalidate();
    }

    /// Returns the current text in this view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.size_cache {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
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

    // Apply the scrolling strategy to the current scroll position.
    //
    // Called when computing rows and when applying a new strategy.
    fn adjust_scroll(&mut self) {
        match self.scroll_strategy {
            ScrollStrategy::StickToTop => self.scrollbase.scroll_top(),
            ScrollStrategy::StickToBottom => self.scrollbase.scroll_bottom(),
            ScrollStrategy::KeepRow => (),
        };
    }

    // This must be non-destructive, as it may be called
    // multiple times during layout.
    fn compute_rows(&mut self, size: Vec2) {
        if self.is_cache_valid(size) {
            return;
        }

        // Completely bust the cache
        // Just in case we fail, we don't want to leave a bad cache.
        self.size_cache = None;

        if size.x == 0 {
            // Nothing we can do at this point.
            return;
        }

        // First attempt: naively hope that we won't need a scrollbar_width
        // (This means we try to use the entire available width for text).
        self.rows =
            LinesIterator::new(strip_last_newline(&self.content), size.x)
                .collect();

        // Width taken by the scrollbar. Without a scrollbar, it's 0.
        let mut scrollbar_width = 0;

        if self.scrollable && self.rows.len() > size.y {
            // We take 1 column for the bar itself + 1 spacing column
            scrollbar_width = 2;

            // If we're too high, include a scrollbar_width
            let available = match size.x.checked_sub(scrollbar_width) {
                Some(s) => s,
                None => return,
            };

            self.rows = LinesIterator::new(&self.content, available).collect();

            if self.rows.is_empty() && !self.content.is_empty() {
                // We have some content, we we didn't find any row for it?
                // This probably means we couldn't even make a single row
                // (for instance we only have 1 column and we have a wide
                // character).
                return;
            }
        }

        // Desired width, including the scrollbar_width.
        self.width = self.rows
            .iter()
            .map(|row| row.width)
            .max()
            .map(|w| w + scrollbar_width);

        // The entire "virtual" size (includes all rows)
        let mut my_size = Vec2::new(self.width.unwrap_or(0), self.rows.len());

        // If we're scrolling, cap the the available size.
        if self.scrollable && my_size.y > size.y {
            my_size.y = size.y;
        }


        // Build a fresh cache.
        self.size_cache = Some(SizeCache::build(my_size, size));
    }

    // Invalidates the cache, so next call will recompute everything.
    fn invalidate(&mut self) {
        self.size_cache = None;
    }
}


impl View for TextView {
    fn draw(&self, printer: &Printer) {
        let h = self.rows.len();
        // If the content is smaller than the view, align it somewhere.
        let offset = self.align.v.get_offset(h, printer.size.y);
        let printer = &printer.offset((0, offset), true);

        self.scrollbase.draw(printer, |printer, i| {
            let row = &self.rows[i];
            let text = &self.content[row.start..row.end];
            let l = text.width();
            let x = self.align.h.get_offset(l, printer.size.x);
            printer.print((x, 0), text);
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.scrollbase.scrollable() {
            return EventResult::Ignored;
        }

        // We have a scrollbar, otherwise the event would just be ignored.
        match event {
            Event::Key(Key::Home) => self.scrollbase.scroll_top(),
            Event::Key(Key::End) => self.scrollbase.scroll_bottom(),
            Event::Key(Key::Up) if self.scrollbase.can_scroll_up() => {
                self.scrollbase.scroll_up(1)
            }
            Event::Key(Key::Down) if self.scrollbase.can_scroll_down() => {
                self.scrollbase.scroll_down(1)
            }
            Event::Mouse {
                event: MouseEvent::WheelDown,
                ..
            } if self.scrollbase.can_scroll_down() =>
            {
                self.scrollbase.scroll_down(5);
            }
            Event::Mouse {
                event: MouseEvent::WheelUp,
                ..
            } if self.scrollbase.can_scroll_up() =>
            {
                self.scrollbase.scroll_up(5);
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| {
                    self.scrollbase.start_drag(position, self.last_size.x)
                })
                .unwrap_or(false) =>
            {
                // Only consume the event if the mouse hits the scrollbar.
                // Start scroll drag at the given position.
            }
            Event::Mouse {
                event: MouseEvent::Hold(MouseButton::Left),
                position,
                offset,
            } => {
                // If the mouse is dragged, we always consume the event.
                let position = position.saturating_sub(offset);
                self.scrollbase.drag(position);
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                ..
            } => {
                self.scrollbase.release_grab();
            }
            Event::Key(Key::PageDown) => self.scrollbase.scroll_down(10),
            Event::Key(Key::PageUp) => self.scrollbase.scroll_up(10),
            _ => return EventResult::Ignored,
        }

        // We just scrolled manually, so reset the scroll strategy.
        self.scroll_strategy = ScrollStrategy::KeepRow;
        EventResult::Consumed(None)
    }

    fn needs_relayout(&self) -> bool {
        self.size_cache.is_none()
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        self.compute_rows(size);

        // This is what we'd like
        let mut ideal = Vec2::new(self.width.unwrap_or(0), self.rows.len());

        if self.scrollable && ideal.y > size.y {
            ideal.y = size.y;
        }

        ideal
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.scrollbase.scrollable()
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the text rows.
        self.last_size = size;
        self.compute_rows(size);
        // Adjust scrolling, in case we're sticking to the bottom for instance.
        self.scrollbase.set_heights(size.y, self.rows.len());
        self.adjust_scroll();
    }
}
